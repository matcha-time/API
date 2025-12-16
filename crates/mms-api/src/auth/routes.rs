use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use serde::Serialize;
use sqlx::types::Uuid;

use super::{cookies, jwt, middleware::AuthUser, refresh_token as rt};
use crate::{ApiState, error::ApiError, middleware::rate_limit};

pub fn routes() -> Router<ApiState> {
    use crate::make_rate_limit_layer;

    // Authenticated routes with general rate limiting
    Router::new()
        .route("/auth/me", get(auth_me))
        .route("/auth/refresh", post(refresh_token))
        .route("/auth/logout", post(logout))
        .layer(make_rate_limit_layer!(
            rate_limit::GENERAL_RATE_PER_SECOND,
            rate_limit::GENERAL_BURST_SIZE
        ))
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub profile_picture_url: Option<String>,
}

async fn auth_me(
    auth_user: AuthUser,
    State(state): State<ApiState>,
) -> Result<Json<UserResponse>, ApiError> {
    // Fetch full user details from database
    let user = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, profile_picture_url
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| ApiError::Auth("User not found".to_string()))?;

    Ok(Json(UserResponse {
        id: user.0,
        username: user.1,
        email: user.2,
        profile_picture_url: user.3,
    }))
}

async fn refresh_token(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
) -> Result<(PrivateCookieJar, Json<serde_json::Value>), ApiError> {
    // Get refresh token from cookie
    let refresh_cookie = jar
        .get("refresh_token")
        .ok_or_else(|| ApiError::Auth("No refresh token found".to_string()))?;

    let old_refresh_token = refresh_cookie.value();

    // Verify and rotate the refresh token
    let (user_id, new_refresh_token, _) = rt::verify_and_rotate_refresh_token(
        &state.pool,
        old_refresh_token,
        state.refresh_token_expiry_days,
    )
    .await?;

    // Fetch user email and verify account status
    let (email, email_verified) = sqlx::query_as::<_, (String, bool)>(
        // language=PostgreSQL
        r#"
            SELECT email, email_verified
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::Auth("User account no longer exists".to_string()))?;

    // Ensure email is still verified
    if !email_verified {
        return Err(ApiError::Auth(
            "Email verification required. Please verify your email.".to_string(),
        ));
    }

    // Generate new JWT access token
    let new_access_token =
        jwt::generate_jwt_token(user_id, email, &state.jwt_secret, state.jwt_expiry_hours)?;

    // Update cookies
    let auth_cookie = cookies::create_auth_cookie(
        new_access_token.clone(),
        &state.environment,
        state.jwt_expiry_hours,
        &state.cookie_domain,
    );
    let refresh_cookie = cookies::create_refresh_token_cookie(
        new_refresh_token,
        &state.environment,
        state.refresh_token_expiry_days,
        &state.cookie_domain,
    );
    let jar = jar.add(auth_cookie).add(refresh_cookie);

    Ok((
        jar,
        Json(serde_json::json!({
            "token": new_access_token,
            "message": "Token refreshed successfully"
        })),
    ))
}

async fn logout(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
) -> (PrivateCookieJar, Json<serde_json::Value>) {
    // Revoke refresh token if present
    if let Some(refresh_cookie) = jar.get("refresh_token")
        && let Err(e) = rt::revoke_refresh_token(&state.pool, refresh_cookie.value()).await
    {
        tracing::error!(error = %e, "Failed to revoke refresh token during logout");
        // Still proceed with logout - clear cookies anyway
    }

    // Remove both auth and refresh token cookies
    let auth_cookie = Cookie::build(("auth_token", "")).path("/").build();
    let refresh_cookie = Cookie::build(("refresh_token", "")).path("/").build();
    let jar = jar.remove(auth_cookie).remove(refresh_cookie);

    (
        jar,
        Json(serde_json::json!({ "message": "Logged out successfully" })),
    )
}
