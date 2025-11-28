//! Request ID middleware for distributed tracing.
//!
//! This middleware adds a unique request ID to each incoming request,
//! which is then propagated through logs for better debugging and tracing.

use axum::{extract::Request, http::header::HeaderName, middleware::Next, response::Response};
use uuid::Uuid;

/// Header name for the request ID
pub const REQUEST_ID_HEADER: &str = "X-Request-ID";

/// Middleware to add request ID to each request
///
/// If the client provides an X-Request-ID header, it will be preserved.
/// Otherwise, a new UUID will be generated.
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Add request ID to request extensions so it can be accessed by handlers
    req.extensions_mut().insert(RequestId(request_id.clone()));

    // Create a tracing span with the request ID
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %req.method(),
        uri = %req.uri(),
    );

    // Process request within the span
    let response = {
        let _guard = span.enter();
        next.run(req).await
    };

    // Add request ID to response headers
    let mut response = response;
    if let Ok(header_value) = request_id.parse() {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), header_value);
    }

    response
}

/// Request ID wrapper for extraction in handlers
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    /// Get the request ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_display() {
        let id = RequestId("test-123".to_string());
        assert_eq!(id.to_string(), "test-123");
        assert_eq!(id.as_str(), "test-123");
    }
}
