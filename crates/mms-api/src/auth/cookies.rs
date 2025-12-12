use axum_extra::extract::cookie::Cookie;

/// Create a refresh token cookie
///
/// Cookies are secure (HTTPS-only) by default in production.
/// In development mode, cookies can be used over HTTP.
pub fn create_refresh_token_cookie(
    token: String,
    environment: &crate::config::Environment,
    expiry_days: i64,
) -> Cookie<'static> {
    let is_development = environment.is_development();
    let same_site = if is_development {
        axum_extra::extract::cookie::SameSite::Lax
    } else {
        axum_extra::extract::cookie::SameSite::Strict
    };

    Cookie::build(("refresh_token", token))
        .path("/")
        .max_age(time::Duration::days(expiry_days))
        .http_only(true)
        .same_site(same_site)
        .secure(!is_development)
        .build()
}
