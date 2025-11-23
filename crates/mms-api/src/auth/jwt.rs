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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Environment;

    #[test]
    fn test_generate_and_verify_jwt_token() {
        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();
        let secret = "test_jwt_secret_minimum_32_characters_long";

        // Generate token
        let token =
            generate_jwt_token(user_id, email.clone(), secret).expect("Failed to generate token");

        assert!(!token.is_empty(), "Token should not be empty");

        // Verify token
        let claims = verify_jwt_token(&token, secret).expect("Failed to verify token");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, email);
        assert!(
            claims.exp > claims.iat,
            "Expiration should be after issued at"
        );
    }

    #[test]
    fn test_verify_jwt_token_with_wrong_secret() {
        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();
        let secret = "test_jwt_secret_minimum_32_characters_long";
        let wrong_secret = "wrong_jwt_secret_minimum_32_characters_long";

        // Generate token with correct secret
        let token = generate_jwt_token(user_id, email, secret).expect("Failed to generate token");

        // Try to verify with wrong secret
        let result = verify_jwt_token(&token, wrong_secret);

        assert!(
            result.is_err(),
            "Verification should fail with wrong secret"
        );
        match result {
            Err(ApiError::Auth(msg)) => {
                assert!(msg.contains("Invalid or expired token"));
            }
            _ => panic!("Expected Auth error"),
        }
    }

    #[test]
    fn test_verify_invalid_jwt_token() {
        let secret = "test_jwt_secret_minimum_32_characters_long";
        let invalid_token = "invalid.jwt.token";

        let result = verify_jwt_token(invalid_token, secret);

        assert!(
            result.is_err(),
            "Verification should fail for invalid token"
        );
        match result {
            Err(ApiError::Auth(msg)) => {
                assert!(msg.contains("Invalid or expired token"));
            }
            _ => panic!("Expected Auth error"),
        }
    }

    #[test]
    fn test_jwt_token_expiration() {
        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();
        let secret = "test_jwt_secret_minimum_32_characters_long";

        let token = generate_jwt_token(user_id, email, secret).expect("Failed to generate token");

        let claims = verify_jwt_token(&token, secret).expect("Failed to verify token");

        // Token should expire in approximately 24 hours (86400 seconds)
        let expiration_duration = claims.exp - claims.iat;
        assert!(
            expiration_duration >= 86390 && expiration_duration <= 86410,
            "Token should expire in approximately 24 hours, got {} seconds",
            expiration_duration
        );
    }

    #[test]
    fn test_create_auth_cookie_development() {
        let token = "test_token".to_string();
        let environment = Environment::Development;

        let cookie = create_auth_cookie(token.clone(), &environment);

        assert_eq!(cookie.name(), "auth_token");
        assert_eq!(cookie.value(), token);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            !cookie.secure().unwrap_or(true),
            "Should not be secure in development"
        );
    }

    #[test]
    fn test_create_auth_cookie_production() {
        let token = "test_token".to_string();
        let environment = Environment::Production;

        let cookie = create_auth_cookie(token.clone(), &environment);

        assert_eq!(cookie.name(), "auth_token");
        assert_eq!(cookie.value(), token);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            cookie.secure().unwrap_or(false),
            "Should be secure in production"
        );
    }

    #[test]
    fn test_create_oidc_flow_cookie_development() {
        let oidc_json =
            r#"{"csrf_token":"test","nonce":"test","pkce_verifier":"test"}"#.to_string();
        let environment = Environment::Development;

        let cookie = create_oidc_flow_cookie(oidc_json.clone(), &environment);

        assert_eq!(cookie.name(), "oidc_flow");
        assert_eq!(cookie.value(), oidc_json);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            !cookie.secure().unwrap_or(true),
            "Should not be secure in development"
        );
    }

    #[test]
    fn test_create_oidc_flow_cookie_production() {
        let oidc_json =
            r#"{"csrf_token":"test","nonce":"test","pkce_verifier":"test"}"#.to_string();
        let environment = Environment::Production;

        let cookie = create_oidc_flow_cookie(oidc_json.clone(), &environment);

        assert_eq!(cookie.name(), "oidc_flow");
        assert_eq!(cookie.value(), oidc_json);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            cookie.secure().unwrap_or(false),
            "Should be secure in production"
        );
    }

    #[test]
    fn test_claims_serialization() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();

        let claims = Claims {
            sub: user_id.to_string(),
            email: "test@example.com".to_string(),
            iat: now.timestamp() as usize,
            exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
        };

        // Test serialization
        let json = serde_json::to_string(&claims).expect("Failed to serialize claims");
        assert!(json.contains(&user_id.to_string()));
        assert!(json.contains("test@example.com"));

        // Test deserialization
        let deserialized: Claims =
            serde_json::from_str(&json).expect("Failed to deserialize claims");
        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.email, claims.email);
        assert_eq!(deserialized.iat, claims.iat);
        assert_eq!(deserialized.exp, claims.exp);
    }
}
