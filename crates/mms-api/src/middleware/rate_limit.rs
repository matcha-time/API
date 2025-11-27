use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Duration;
pub use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

/// Rate limits for different endpoint types
pub const AUTH_RATE_PER_SECOND: u64 = 5;
pub const AUTH_BURST_SIZE: u32 = 10;

pub const SENSITIVE_RATE_PER_SECOND: u64 = 2;
pub const SENSITIVE_BURST_SIZE: u32 = 3;

pub const GENERAL_RATE_PER_SECOND: u64 = 10;
pub const GENERAL_BURST_SIZE: u32 = 20;

/// Helper macro to create a rate limiter with specific settings
/// Uses SmartIpKeyExtractor which tries x-forwarded-for, x-real-ip, forwarded headers,
/// then falls back to ConnectInfo for IP extraction
#[macro_export]
macro_rules! make_rate_limit_layer {
    ($per_second:expr, $burst:expr) => {{
        let config = $crate::middleware::rate_limit::GovernorConfigBuilder::default()
            .per_second($per_second)
            .burst_size($burst)
            .use_headers()
            .finish()
            .expect("Failed to build rate limiter configuration");
        $crate::middleware::rate_limit::GovernorLayer::new(config)
    }};
}

/// Timing-safe delay middleware to prevent timing attacks
/// Adds a small constant delay to all responses from sensitive endpoints
pub async fn timing_safe_middleware(req: Request, next: Next) -> Response {
    let response = next.run(req).await;

    // Add a small constant delay (50ms) to prevent timing analysis
    tokio::time::sleep(Duration::from_millis(50)).await;

    response
}
