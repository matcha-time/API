use axum_extra::extract::cookie::Cookie;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::{config::Environment, error::ApiError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id as string
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

/// Generate a JWT token for a user
pub fn generate_jwt_token(
    user_id: Uuid,
    email: String,
    jwt_secret: &str,
) -> Result<String, ApiError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email,
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )?;

    Ok(token)
}

/// Verify and decode a JWT token
pub fn verify_jwt_token(token: &str, jwt_secret: &str) -> Result<Claims, ApiError> {
    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::Auth("Invalid or expired token".to_string()))?;

    Ok(token_data.claims)
}

/// Create an auth cookie with the JWT token
///
/// Cookies are secure (HTTPS-only) by default in production.
/// In development mode, cookies can be used over HTTP.
pub fn create_auth_cookie(token: String, environment: &Environment) -> Cookie<'static> {
    let is_development = environment.is_development();

    Cookie::build(("auth_token", token))
        .path("/")
        .max_age(time::Duration::hours(24))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(!is_development) // Secure by default, insecure only in development
        .build()
}

/// Create a temporary OIDC flow cookie
///
/// Cookies are secure (HTTPS-only) by default in production.
/// In development mode, cookies can be used over HTTP.
pub fn create_oidc_flow_cookie(oidc_json: String, environment: &Environment) -> Cookie<'static> {
    let is_development = environment.is_development();

    Cookie::build(("oidc_flow", oidc_json))
        .path("/")
        .max_age(time::Duration::minutes(10))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(!is_development) // Secure by default, insecure only in development
        .build()
}
