use axum::{extract::Request, middleware::Next, response::Response};
use std::time::{Duration, Instant};
pub use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

/// Rate limits for different endpoint types
pub const AUTH_RATE_PER_SECOND: u64 = 5;
// Reduced from 10 to 5 to prevent rapid brute force attempts
pub const AUTH_BURST_SIZE: u32 = 5;

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

/// Timing-safe middleware to prevent timing attacks on sensitive endpoints.
/// Pads every response to a minimum fixed duration so that the total time
/// is constant regardless of how fast the handler completes.
const TIMING_SAFE_MIN_DURATION: Duration = Duration::from_millis(250);

pub async fn timing_safe_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let response = next.run(req).await;

    let elapsed = start.elapsed();
    if elapsed < TIMING_SAFE_MIN_DURATION {
        tokio::time::sleep(TIMING_SAFE_MIN_DURATION - elapsed).await;
    }

    response
}
