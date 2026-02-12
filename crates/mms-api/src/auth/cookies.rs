use axum_extra::extract::cookie::{Cookie, SameSite};

use crate::config::Environment;

/// Build an HttpOnly, SameSite=Lax cookie with domain and secure flag
/// derived from the environment.
///
/// - Production: Secure=true, for HTTPS-only access across subdomains
/// - Development: Secure=false, allowing HTTP on localhost
fn build_cookie(
    name: &str,
    value: String,
    max_age: time::Duration,
    environment: &Environment,
    cookie_domain: &str,
) -> Cookie<'static> {
    Cookie::build((name.to_owned(), value))
        .path("/")
        .max_age(max_age)
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(!environment.is_development())
        .domain(cookie_domain.to_owned())
        .build()
}

/// Create an auth cookie with the JWT token
pub fn create_auth_cookie(
    token: String,
    environment: &Environment,
    expiry_hours: i64,
    cookie_domain: &str,
) -> Cookie<'static> {
    build_cookie(
        "auth_token",
        token,
        time::Duration::hours(expiry_hours),
        environment,
        cookie_domain,
    )
}

/// Create a temporary OIDC flow cookie
pub fn create_oidc_flow_cookie(
    oidc_json: String,
    environment: &Environment,
    expiry_minutes: i64,
    cookie_domain: &str,
) -> Cookie<'static> {
    build_cookie(
        "oidc_flow",
        oidc_json,
        time::Duration::minutes(expiry_minutes),
        environment,
        cookie_domain,
    )
}

/// Create a refresh token cookie
pub fn create_refresh_token_cookie(
    token: String,
    environment: &Environment,
    expiry_days: i64,
    cookie_domain: &str,
) -> Cookie<'static> {
    build_cookie(
        "refresh_token",
        token,
        time::Duration::days(expiry_days),
        environment,
        cookie_domain,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_auth_cookie_development() {
        let token = "test_token".to_string();
        let environment = Environment::Development;

        let cookie = create_auth_cookie(token.clone(), &environment, 24, "localhost");

        assert_eq!(cookie.name(), "auth_token");
        assert_eq!(cookie.value(), token);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            !cookie.secure().unwrap_or(true),
            "Should not be secure in development"
        );
        assert_eq!(cookie.domain(), Some("localhost"));
    }

    #[test]
    fn test_create_auth_cookie_production() {
        let token = "test_token".to_string();
        let environment = Environment::Production;

        let cookie = create_auth_cookie(token.clone(), &environment, 24, ".matcha-time.dev");

        assert_eq!(cookie.name(), "auth_token");
        assert_eq!(cookie.value(), token);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            cookie.secure().unwrap_or(false),
            "Should be secure in production"
        );
        // Note: The cookie library may strip the leading dot, but it's still set correctly
        // The leading dot is implicit in modern cookie handling
        let domain = cookie.domain().unwrap();
        assert!(
            domain == ".matcha-time.dev" || domain == "matcha-time.dev",
            "Should have domain for cross-subdomain support, got: {}",
            domain
        );
    }

    #[test]
    fn test_create_oidc_flow_cookie_development() {
        let oidc_json =
            r#"{"csrf_token":"test","nonce":"test","pkce_verifier":"test"}"#.to_string();
        let environment = Environment::Development;

        let cookie = create_oidc_flow_cookie(oidc_json.clone(), &environment, 10, "localhost");

        assert_eq!(cookie.name(), "oidc_flow");
        assert_eq!(cookie.value(), oidc_json);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            !cookie.secure().unwrap_or(true),
            "Should not be secure in development"
        );
        assert_eq!(cookie.domain(), Some("localhost"));
    }

    #[test]
    fn test_create_oidc_flow_cookie_production() {
        let oidc_json =
            r#"{"csrf_token":"test","nonce":"test","pkce_verifier":"test"}"#.to_string();
        let environment = Environment::Production;

        let cookie =
            create_oidc_flow_cookie(oidc_json.clone(), &environment, 10, ".matcha-time.dev");

        assert_eq!(cookie.name(), "oidc_flow");
        assert_eq!(cookie.value(), oidc_json);
        assert_eq!(cookie.path(), Some("/"));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(
            cookie.secure().unwrap_or(false),
            "Should be secure in production"
        );
        // Note: The cookie library may strip the leading dot, but it's still set correctly
        // The leading dot is implicit in modern cookie handling
        let domain = cookie.domain().unwrap();
        assert!(
            domain == ".matcha-time.dev" || domain == "matcha-time.dev",
            "Should have domain for cross-subdomain support, got: {}",
            domain
        );
    }
}
