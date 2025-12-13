use axum_extra::extract::cookie::Cookie;

use crate::config::Environment;

/// Create an auth cookie with the JWT token
///
/// Cookies are secure (HTTPS-only) by default in production.
/// In development mode, cookies can be used over HTTP.
///
/// For production with separate subdomains (api.matcha-time.dev and matcha-time.dev):
/// - Uses SameSite=Lax for CSRF protection while allowing subdomain access
/// - Sets Domain to work across all subdomains (e.g., ".matcha-time.dev")
/// - Always secure and HttpOnly in production for maximum security
pub fn create_auth_cookie(
    token: String,
    environment: &Environment,
    expiry_hours: i64,
    cookie_domain: &str,
) -> Cookie<'static> {
    let is_development = environment.is_development();

    Cookie::build(("auth_token", token))
        .path("/")
        .max_age(time::Duration::hours(expiry_hours))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(!is_development)
        .domain(cookie_domain.to_string())
        .build()
}

/// Create a temporary OIDC flow cookie
///
/// Cookies are secure (HTTPS-only) by default in production.
/// In development mode, cookies can be used over HTTP.
///
/// For production with separate subdomains (api.matcha-time.dev and matcha-time.dev):
/// - Uses SameSite=Lax for CSRF protection while allowing subdomain access
/// - Sets Domain to work across all subdomains (e.g., ".matcha-time.dev")
/// - Always secure and HttpOnly in production for maximum security
pub fn create_oidc_flow_cookie(
    oidc_json: String,
    environment: &Environment,
    expiry_minutes: i64,
    cookie_domain: &str,
) -> Cookie<'static> {
    let is_development = environment.is_development();

    Cookie::build(("oidc_flow", oidc_json))
        .path("/")
        .max_age(time::Duration::minutes(expiry_minutes))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(!is_development)
        .domain(cookie_domain.to_string())
        .build()
}

/// Create a refresh token cookie
///
/// Cookies are secure (HTTPS-only) by default in production.
/// In development mode, cookies can be used over HTTP.
///
/// For production with separate subdomains (api.matcha-time.dev and matcha-time.dev):
/// - Uses SameSite=Lax for CSRF protection while allowing subdomain access
/// - Sets Domain to work across all subdomains (e.g., ".matcha-time.dev")
/// - Always secure and HttpOnly in production for maximum security
pub fn create_refresh_token_cookie(
    token: String,
    environment: &Environment,
    expiry_days: i64,
    cookie_domain: &str,
) -> Cookie<'static> {
    let is_development = environment.is_development();

    Cookie::build(("refresh_token", token))
        .path("/")
        .max_age(time::Duration::days(expiry_days))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(!is_development)
        .domain(cookie_domain.to_string())
        .build()
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
