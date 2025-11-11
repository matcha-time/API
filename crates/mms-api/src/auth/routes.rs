use axum::{
    Json, Router,
    extract::{Query, State},
    response::Redirect,
    routing::get,
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use chrono::Utc;
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope};
use openidconnect::{AuthenticationFlow, Nonce, TokenResponse, core::CoreResponseType};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::{ApiState, auth::models::OidcFlowData, error::ApiError};

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/auth/google", get(google_auth))
        .route("/auth/callback", get(auth_callback))
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

    let cookie = Cookie::build(("oidc_flow", oidc_json))
        .path("/")
        .max_age(time::Duration::minutes(10))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(false) // Set to true in production with HTTPS
        .build();

    let jar = jar.add(cookie);

    Ok((jar, Redirect::to(auth_url.as_str())))
}

#[derive(Debug, Deserialize)]
struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

async fn auth_callback(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Query(query): Query<AuthRequest>,
) -> Result<(PrivateCookieJar, Json<AuthResponse>), ApiError> {
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
    let email = id_token_claims
        .email()
        .ok_or_else(|| ApiError::InvalidIdToken("No email in ID token".to_string()))?
        .to_string();
    let email_verified = id_token_claims.email_verified().unwrap_or(false);

    if !email_verified {
        return Err(ApiError::Oidc("Email not verified".to_string()));
    }

    // Get username from name or use email prefix
    let username = id_token_claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.to_string())
        .unwrap_or_else(|| {
            // Use email prefix as username, but sanitize it
            email.split('@').next().unwrap_or("user").to_string()
        });

    // Check if user exists, if not create them
    let user = sqlx::query_as::<_, (Uuid, String, String)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email
            FROM users
            WHERE email = $1
        "#,
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;

    let (user_id, username, email) = if let Some(user) = user {
        user
    } else {
        // Create new user with OAuth
        // Generate a random password hash since OAuth users won't use password login
        // We use a random string that will never match any actual password
        let random_password_hash = bcrypt::hash(
            &format!("oauth_{}", uuid::Uuid::new_v4()),
            bcrypt::DEFAULT_COST,
        )?;

        // Try to insert user, handle username conflicts
        let mut final_username = username.clone();
        let mut attempts = 0;
        let user_id = loop {
            match sqlx::query_scalar::<_, Uuid>(
                // language=PostgreSQL
                r#"
                    INSERT INTO users (username, email, password_hash)
                    VALUES ($1, $2, $3)
                    RETURNING id
                "#,
            )
            .bind(&final_username)
            .bind(&email)
            .bind(&random_password_hash)
            .fetch_optional(&state.pool)
            .await
            {
                Ok(Some(id)) => break id,
                Ok(None) => {
                    // This shouldn't happen with RETURNING, but handle it
                    return Err(ApiError::Database(sqlx::Error::RowNotFound));
                }
                Err(sqlx::Error::Database(db_err)) if db_err.constraint().is_some() => {
                    // Constraint violation (username or email conflict)
                    // Check if it's an email conflict (race condition - user created between check and insert)
                    if db_err.message().contains("email")
                        || db_err.message().contains("users_email_key")
                    {
                        // Email already exists, fetch the existing user
                        let existing_user = sqlx::query_as::<_, (Uuid, String, String)>(
                            // language=PostgreSQL
                            r#"
                                SELECT id, username, email
                                FROM users
                                WHERE email = $1
                            "#,
                        )
                        .bind(&email)
                        .fetch_one(&state.pool)
                        .await?;
                        break existing_user.0;
                    }
                    // Username conflict, try with a suffix
                    attempts += 1;
                    if attempts > 10 {
                        return Err(ApiError::Auth(
                            "Failed to create user after multiple attempts".to_string(),
                        ));
                    }
                    let suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
                    final_username = format!("{}_{}", username, suffix);
                }
                Err(e) => return Err(ApiError::Database(e)),
            }
        };

        // Fetch the final user data (in case of race condition, get the actual username)
        let final_user = sqlx::query_as::<_, (Uuid, String, String)>(
            // language=PostgreSQL
            r#"
                SELECT id, username, email
                FROM users
                WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&state.pool)
        .await?;

        // Ensure user_stats exists (idempotent - won't create if already exists)
        sqlx::query(
            // language=PostgreSQL
            r#"
                INSERT INTO user_stats (user_id)
                VALUES ($1)
                ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .execute(&state.pool)
        .await?;

        final_user
    };

    // Generate JWT token using database user ID
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.clone(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )?;

    // Set auth cookie with JWT
    let auth_cookie = Cookie::build(("auth_token", token.clone()))
        .path("/")
        .max_age(time::Duration::hours(24))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(false) // Set to true in production with HTTPS
        .build();

    let jar = jar.add(auth_cookie);

    Ok((
        jar,
        Json(AuthResponse {
            token,
            user: UserResponse {
                id: user_id,
                username,
                email,
            },
        }),
    ))
}
