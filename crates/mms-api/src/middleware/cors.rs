use axum::http::{Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Creates a CORS layer with configured allowed origins and standard settings
///
/// # Arguments
/// * `allowed_origins` - List of allowed origin URLs as strings
///
/// # Returns
/// A configured `CorsLayer` with:
/// - Allowed origins parsed from the provided list
/// - Standard HTTP methods (GET, POST, PUT, PATCH, DELETE, OPTIONS)
/// - Standard headers (Content-Type, Accept)
/// - Credentials enabled
pub fn create_cors_layer(allowed_origins: Vec<String>) -> CorsLayer {
    let origins = allowed_origins
        .into_iter()
        .filter_map(|s| s.parse::<axum::http::HeaderValue>().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
        .allow_credentials(true)
}
