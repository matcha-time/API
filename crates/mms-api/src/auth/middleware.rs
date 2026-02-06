use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::{PrivateCookieJar, cookie::Key};
use sqlx::types::Uuid;

use super::jwt::verify_jwt_token;
use crate::{error::ApiError, state::AuthConfig};

/// Authenticated user extractor
///
/// Use this in route handlers to ensure the user is authenticated.
/// It will automatically validate the JWT token from the cookie.
///
/// # Example
/// ```
/// use axum::extract::State;
/// use mms_api::{error::ApiError, auth::AuthUser, ApiState};
///
///
/// async fn protected_route(
///     auth_user: AuthUser,
///     State(state): State<ApiState>,
/// ) -> Result<(), ApiError> {
///     // auth_user.user_id and auth_user.email are available
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub email: String,
}

impl<S> FromRequestParts<S> for AuthUser
where
    AuthConfig: FromRef<S>,
    Key: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the auth config
        let auth_config = AuthConfig::from_ref(state);

        // Extract the cookie jar
        let jar = PrivateCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::Auth("Failed to read cookies".to_string()))?;

        // Get the auth token from cookie
        let token = jar
            .get("auth_token")
            .ok_or(ApiError::Auth("Not authenticated".to_string()))?
            .value()
            .to_owned();

        // Verify the token
        let claims = verify_jwt_token(&token, &auth_config.jwt_secret)?;

        // Parse user_id from claims
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| ApiError::Auth("Invalid user ID in token".to_string()))?;

        Ok(AuthUser {
            user_id,
            email: claims.email,
        })
    }
}
