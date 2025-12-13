use axum::{
    Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::get,
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope};
use openidconnect::{AuthenticationFlow, Nonce, TokenResponse, core::CoreResponseType};
use serde::Deserialize;

use super::{models::OidcFlowData, service};
use crate::auth::{cookies, jwt, refresh_token as rt};
use crate::{ApiState, error::ApiError, middleware::rate_limit};

pub fn routes() -> Router<ApiState> {
    use crate::make_rate_limit_layer;

    Router::new()
        .route("/auth/google", get(google_auth))
        .route("/auth/callback", get(auth_callback))
        .layer(make_rate_limit_layer!(
            rate_limit::GENERAL_RATE_PER_SECOND,
            rate_limit::GENERAL_BURST_SIZE
        ))
}

async fn google_auth(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
) -> Result<(PrivateCookieJar, Redirect), ApiError> {
    // Generate PKCE code verifier and challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate CSRF token and nonce
    let (auth_url, csrf_token, nonce) = state
        .oidc_client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Store CSRF token, nonce, and PKCE verifier in encrypted cookie
    let oidc_data = OidcFlowData {
        csrf_token: csrf_token.secret().clone(),
        nonce: nonce.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
    };

    let oidc_json = serde_json::to_string(&oidc_data)
        .map_err(|e| ApiError::Cookie(format!("Failed to serialize OIDC data: {}", e)))?;

    let cookie = cookies::create_oidc_flow_cookie(
        oidc_json,
        &state.environment,
        state.oidc_flow_expiry_minutes,
    );
    let jar = jar.add(cookie);

    Ok((jar, Redirect::to(auth_url.as_str())))
}

#[derive(Debug, Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

async fn auth_callback(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Query(query): Query<AuthRequest>,
) -> Result<(PrivateCookieJar, impl IntoResponse), ApiError> {
    // Retrieve OIDC flow data from cookie
    let oidc_cookie = jar
        .get("oidc_flow")
        .ok_or_else(|| ApiError::Cookie("No OIDC flow cookie found".to_string()))?;

    let oidc_data: OidcFlowData = serde_json::from_str(oidc_cookie.value())
        .map_err(|e| ApiError::Cookie(format!("Failed to parse OIDC data: {}", e)))?;

    // Verify CSRF token
    if oidc_data.csrf_token != query.state {
        return Err(ApiError::Cookie("Invalid CSRF token".to_string()));
    }

    // Remove the OIDC flow cookie
    let jar = jar.remove(Cookie::from("oidc_flow"));

    // Exchange authorization code for tokens with PKCE verifier
    let token_response = state
        .oidc_client
        .exchange_code(AuthorizationCode::new(query.code))
        .map_err(|e| ApiError::Oidc(format!("Token exchange failed: {}", e)))?
        .set_pkce_verifier(PkceCodeVerifier::new(oidc_data.pkce_verifier))
        .request_async(&reqwest::Client::new())
        .await
        .map_err(|e| ApiError::Oidc(format!("Token exchange failed: {}", e)))?;

    // Get and verify the ID token
    let id_token = token_response
        .id_token()
        .ok_or_else(|| ApiError::InvalidIdToken("No ID token in response".to_string()))?;

    let id_token_verifier = state.oidc_client.id_token_verifier();
    let id_token_claims = id_token
        .claims(&id_token_verifier, &Nonce::new(oidc_data.nonce))
        .map_err(|e| ApiError::InvalidIdToken(format!("ID token verification failed: {}", e)))?;

    // Extract user info from ID token
    let google_id = id_token_claims.subject().to_string();
    let email = id_token_claims
        .email()
        .ok_or_else(|| ApiError::InvalidIdToken("No email in ID token".to_string()))?
        .to_string();
    let email_verified = id_token_claims.email_verified().unwrap_or(false);
    let name = id_token_claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.to_string());
    let picture = id_token_claims
        .picture()
        .and_then(|p| p.get(None))
        .map(|p| p.to_string());

    if !email_verified {
        return Err(ApiError::Oidc("Email not verified".to_string()));
    }

    // Find or create user in database
    let user = service::find_or_create_google_user(
        &state.pool,
        &google_id,
        &email,
        name.as_deref(),
        picture.as_deref(),
    )
    .await?;

    // Generate JWT access token
    let token = jwt::generate_jwt_token(
        user.id,
        user.email.clone(),
        &state.jwt_secret,
        state.jwt_expiry_hours,
    )?;

    // Generate refresh token
    let (refresh_token, refresh_token_hash) = rt::generate_refresh_token();
    rt::store_refresh_token(
        &state.pool,
        user.id,
        &refresh_token_hash,
        None,
        None,
        state.refresh_token_expiry_days,
    )
    .await?;

    // Set cookies with JWT and refresh token
    let auth_cookie =
        cookies::create_auth_cookie(token.clone(), &state.environment, state.jwt_expiry_hours);
    let refresh_cookie = cookies::create_refresh_token_cookie(
        refresh_token,
        &state.environment,
        state.refresh_token_expiry_days,
    );
    let jar = jar.add(auth_cookie).add(refresh_cookie);

    // Create HTML response with frontend URL from config
    let html = format!(
        r#"
            <!DOCTYPE html>
            <html>
            <head><title>Authentication Successful</title></head>
                <body>
                    <script>
                        // Close popup and notify parent
                        window.opener.postMessage(
                            {{ type: 'google-auth-success' }},
                            '{}'
                        );
                        window.close();
                    </script>
                </body>
            </html>
        "#,
        state.frontend_url
    );

    Ok((jar, axum::response::Html(html)))
}
