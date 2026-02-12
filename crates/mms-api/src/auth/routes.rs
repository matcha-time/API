use axum::{
    Json, Router,
    extract::State,
    routing::{get, patch, post},
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use super::{cookies, jwt, middleware::AuthUser, refresh_token as rt};
use crate::{ApiState, error::ApiError, middleware::rate_limit, validation};

use mms_db::models::{UserCredentials, UserProfile};
use mms_db::repositories::user as user_repo;

pub fn routes() -> Router<ApiState> {
    use crate::make_rate_limit_layer;

    // Authenticated routes with general rate limiting
    Router::new()
        .route("/auth/me", get(auth_me))
        .route("/auth/refresh", post(refresh_token))
        .route("/auth/logout", post(logout))
        .route(
            "/users/me/language-preferences",
            patch(update_language_preferences),
        )
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
    pub native_language: Option<String>,
    pub learning_language: Option<String>,
}

impl From<UserProfile> for UserResponse {
    fn from(user: UserProfile) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            profile_picture_url: user.profile_picture_url,
            native_language: user.native_language,
            learning_language: user.learning_language,
        }
    }
}

impl From<UserCredentials> for UserResponse {
    fn from(user: UserCredentials) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            profile_picture_url: user.profile_picture_url,
            native_language: user.native_language,
            learning_language: user.learning_language,
        }
    }
}

async fn auth_me(
    auth_user: AuthUser,
    State(state): State<ApiState>,
) -> Result<Json<UserResponse>, ApiError> {
    // Fetch full user details from database
    let user = user_repo::find_profile_by_id(&state.pool, auth_user.user_id)
        .await
        .map_err(|_| ApiError::Auth("User not found".to_string()))?
        .ok_or_else(|| ApiError::Auth("User not found".to_string()))?;

    Ok(Json(user.into()))
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
        state.auth.refresh_token_expiry_days,
    )
    .await?;

    // Fetch user email and verify account status
    let status = user_repo::find_email_verified_status(&state.pool, user_id)
        .await?
        .ok_or_else(|| ApiError::Auth("User account no longer exists".to_string()))?;

    // Ensure email is still verified
    if !status.email_verified {
        return Err(ApiError::Auth(
            "Email verification required. Please verify your email.".to_string(),
        ));
    }

    // Generate new JWT access token
    let new_access_token = jwt::generate_jwt_token(
        user_id,
        status.email,
        &state.auth.jwt_secret,
        state.auth.jwt_expiry_hours,
    )?;

    // Update cookies
    let auth_cookie = cookies::create_auth_cookie(
        new_access_token.clone(),
        &state.cookie.environment,
        state.auth.jwt_expiry_hours,
        &state.cookie.cookie_domain,
    );
    let refresh_cookie = cookies::create_refresh_token_cookie(
        new_refresh_token,
        &state.cookie.environment,
        state.auth.refresh_token_expiry_days,
        &state.cookie.cookie_domain,
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
    // IMPORTANT: domain and path must match the original cookie attributes for proper removal
    let cookie_domain = state.cookie.cookie_domain.to_string();
    let auth_cookie = Cookie::build(("auth_token", ""))
        .path("/")
        .domain(cookie_domain.clone())
        .build();
    let refresh_cookie = Cookie::build(("refresh_token", ""))
        .path("/")
        .domain(cookie_domain)
        .build();
    let jar = jar.remove(auth_cookie).remove(refresh_cookie);

    (
        jar,
        Json(serde_json::json!({ "message": "Logged out successfully" })),
    )
}

#[derive(Debug, Deserialize)]
struct UpdateLanguagePreferencesRequest {
    native_language: String,
    learning_language: String,
}

#[derive(Debug, Serialize)]
struct UpdateLanguagePreferencesResponse {
    message: String,
    user: UserResponse,
}

async fn update_language_preferences(
    auth_user: AuthUser,
    State(state): State<ApiState>,
    Json(payload): Json<UpdateLanguagePreferencesRequest>,
) -> Result<Json<UpdateLanguagePreferencesResponse>, ApiError> {
    // Validate language codes against the allowed whitelist
    validation::validate_language_code(&payload.native_language)?;
    validation::validate_language_code(&payload.learning_language)?;

    // Update both language preferences
    let updated_user = user_repo::update_language_preferences(
        &state.pool,
        auth_user.user_id,
        &payload.native_language,
        &payload.learning_language,
    )
    .await?;

    Ok(Json(UpdateLanguagePreferencesResponse {
        message: "Language preferences updated successfully".to_string(),
        user: updated_user.into(),
    }))
}
