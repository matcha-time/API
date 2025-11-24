use axum::{
    Router,
    extract::Request,
    http::header,
    middleware::{self, Next},
    response::Response,
};

use crate::config::Environment;

/// Security headers middleware
/// Adds essential security headers to all responses
pub async fn security_headers_middleware(
    environment: Environment,
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Prevent MIME type sniffing - CRITICAL for APIs
    // Ensures JSON is always treated as JSON, not executable code
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        header::HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking - API responses should not be framed
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        header::HeaderValue::from_static("DENY"),
    );

    // Strict Transport Security (HSTS) - enforce HTTPS
    // Only in production to avoid issues in local development
    if environment.is_production() {
        headers.insert(
            header::HeaderName::from_static("strict-transport-security"),
            header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}

/// Apply security headers to a router
pub fn apply_security_headers<S>(router: Router<S>, environment: Environment) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(middleware::from_fn(move |req, next| {
        security_headers_middleware(environment.clone(), req, next)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router, http::StatusCode};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_security_headers_applied_production() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(move |req, next| {
                security_headers_middleware(Environment::Production, req, next)
            }));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();

        // Check critical security headers are present
        assert_eq!(
            headers.get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            headers.get("x-frame-options").unwrap(),
            "DENY"
        );
        assert!(
            headers.get("strict-transport-security").is_some(),
            "HSTS should be present in production"
        );
    }

    #[tokio::test]
    async fn test_security_headers_development_no_hsts() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn(move |req, next| {
                security_headers_middleware(Environment::Development, req, next)
            }));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/test")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let headers = response.headers();

        // Basic headers should still be present
        assert_eq!(
            headers.get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            headers.get("x-frame-options").unwrap(),
            "DENY"
        );

        // HSTS should NOT be present in development
        assert!(
            headers.get("strict-transport-security").is_none(),
            "HSTS should not be present in development"
        );
    }
}
