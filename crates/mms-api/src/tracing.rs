//! Tracing and logging configuration for the application
//!
//! This module provides structured logging with different configurations
//! for development and production environments.

use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Environment;

/// Initialize tracing/logging based on the environment
///
/// # Development Mode
/// - Pretty-printed, human-readable logs with colors
/// - Default level: DEBUG
/// - Shows file locations and line numbers
///
/// # Production Mode
/// - JSON-formatted structured logs
/// - Default level: INFO
/// - Optimized for log aggregation systems (ELK, Datadog, etc.)
/// - Includes request IDs, user IDs, and other structured fields
///
/// # Environment Variables
/// - `RUST_LOG`: Override default log level (e.g., `RUST_LOG=debug,tower_http=trace`)
pub fn init_tracing(env: &Environment) {
    if env.is_development() {
        init_development_tracing();
    } else {
        init_production_tracing();
    }
}

/// Initialize development-friendly tracing with pretty output
fn init_development_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("debug,tower_http=debug,sqlx=warn"));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_line_number(true)
                .with_file(true)
                .pretty()
                .with_filter(env_filter),
        )
        .init();

    tracing::info!("Tracing initialized in development mode");
}

/// Initialize production tracing with JSON output
fn init_production_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn"));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
                .flatten_event(true)
                .with_target(true)
                .with_filter(env_filter),
        )
        .init();

    tracing::info!("Tracing initialized in production mode");
}
