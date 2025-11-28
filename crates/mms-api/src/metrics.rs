//! Prometheus metrics for monitoring API performance and health.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use metrics::{counter, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

/// Initialize Prometheus metrics exporter
pub fn init_metrics() -> anyhow::Result<PrometheusHandle> {
    let builder = PrometheusBuilder::new();

    // Configure histogram buckets for request duration (in seconds)
    let builder = builder.set_buckets_for_metric(
        Matcher::Full("http_request_duration_seconds".to_string()),
        &[
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ],
    )?;

    // Install the exporter and get the handle
    let handle = builder.install_recorder()?;

    Ok(handle)
}

/// Middleware to record HTTP request metrics
pub async fn track_metrics(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Normalize path to avoid high cardinality (replace IDs with placeholders)
    let normalized_path = normalize_path(&path);

    // Track in-flight requests
    counter!("http_requests_in_flight", "method" => method.clone(), "path" => normalized_path.clone()).increment(1);

    // Process the request
    let response: Response = next.run(req).await;

    // Track request completion
    counter!("http_requests_in_flight", "method" => method.clone(), "path" => normalized_path.clone()).absolute(0);

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    // Request counter
    counter!(
        "http_requests_total",
        "method" => method.clone(),
        "path" => normalized_path.clone(),
        "status" => status.clone()
    )
    .increment(1);

    // Request duration histogram
    histogram!(
        "http_request_duration_seconds",
        "method" => method.clone(),
        "path" => normalized_path.clone(),
        "status" => status
    )
    .record(duration);

    response
}

/// Normalize URL paths to reduce cardinality in metrics
/// Replaces UUIDs and numeric IDs with placeholders
fn normalize_path(path: &str) -> String {
    let uuid_regex =
        regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap();
    let number_regex = regex::Regex::new(r"/\d+").unwrap();

    let mut normalized = uuid_regex.replace_all(path, ":id").to_string();
    normalized = number_regex.replace_all(&normalized, "/:id").to_string();

    normalized
}

/// Handler for the /metrics endpoint
pub async fn metrics_handler(
    axum::extract::State(handle): axum::extract::State<PrometheusHandle>,
) -> impl IntoResponse {
    (StatusCode::OK, handle.render())
}

/// Record database query metrics
pub fn record_db_query(query_name: &str, duration_secs: f64, success: bool) {
    let status = if success { "success" } else { "error" };

    counter!(
        "db_queries_total",
        "query" => query_name.to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    histogram!(
        "db_query_duration_seconds",
        "query" => query_name.to_string()
    )
    .record(duration_secs);
}

/// Record authentication events
pub fn record_auth_event(event_type: &str, method: &str, success: bool) {
    let status = if success { "success" } else { "failure" };

    counter!(
        "auth_events_total",
        "type" => event_type.to_string(),
        "method" => method.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Record email sending events
pub fn record_email_event(email_type: &str, success: bool) {
    let status = if success { "success" } else { "failure" };

    counter!(
        "email_events_total",
        "type" => email_type.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(
            normalize_path("/api/users/550e8400-e29b-41d4-a716-446655440000"),
            "/api/users/:id"
        );
        assert_eq!(normalize_path("/api/decks/123"), "/api/decks/:id");
        assert_eq!(
            normalize_path("/api/decks/550e8400-e29b-41d4-a716-446655440000/flashcards/456"),
            "/api/decks/:id/flashcards/:id"
        );
        assert_eq!(normalize_path("/api/health"), "/api/health");
    }
}
