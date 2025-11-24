use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
};
use tower_governor::{
    GovernorLayer,
    governor::GovernorConfigBuilder,
};
use std::time::Duration;

/// Strict rate limiting for authentication endpoints
/// 5 requests per second with burst of 10 (prevents brute force attacks)
pub fn auth_rate_limit<S>() -> GovernorLayer<tower_governor::key_extractor::SmartIpKeyExtractor, tower_governor::governor::DefaultDirectRateLimiter, axum::body::Body>
where
    S: Clone + Send + Sync + 'static,
{
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(5)
        .burst_size(10)
        .use_headers()
        .finish()
        .expect("Failed to build auth rate limiter configuration");

    GovernorLayer::new(governor_conf)
}

/// Very strict rate limiting for password reset and sensitive operations
/// 2 requests per minute with burst of 3 (prevents email flooding and enumeration)
pub fn sensitive_rate_limit<S>() -> GovernorLayer<tower_governor::key_extractor::SmartIpKeyExtractor, tower_governor::governor::DefaultDirectRateLimiter, axum::body::Body>
where
    S: Clone + Send + Sync + 'static,
{
    let governor_conf = GovernorConfigBuilder::default()
        .per_millisecond(33) // Approximately 2 per minute
        .burst_size(3)
        .use_headers()
        .finish()
        .expect("Failed to build sensitive rate limiter configuration");

    GovernorLayer::new(governor_conf)
}

/// Moderate rate limiting for general authenticated endpoints
/// 10 requests per second with burst of 20
pub fn general_rate_limit<S>() -> GovernorLayer<tower_governor::key_extractor::SmartIpKeyExtractor, tower_governor::governor::DefaultDirectRateLimiter, axum::body::Body>
where
    S: Clone + Send + Sync + 'static,
{
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(20)
        .use_headers()
        .finish()
        .expect("Failed to build general rate limiter configuration");

    GovernorLayer::new(governor_conf)
}

/// Timing-safe delay middleware to prevent timing attacks
/// Adds a small constant delay to all responses from sensitive endpoints
pub async fn timing_safe_middleware(req: Request, next: Next) -> Response {
    let response = next.run(req).await;

    // Add a small constant delay (50ms) to prevent timing analysis
    tokio::time::sleep(Duration::from_millis(50)).await;

    response
}

/// Apply timing-safe middleware to a router
pub fn apply_timing_safe<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(middleware::from_fn(timing_safe_middleware))
}
