use axum::{Router, middleware, routing::get};
use mms_api::middleware::request_id::request_id_middleware;
use mms_api::{config::ApiConfig, state::ApiState};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config = ApiConfig::from_env()?;

    // Initialize tracing/logging based on environment
    mms_api::tracing::init_tracing(&config.env);

    // Initialize Prometheus metrics exporter
    let metrics_handle = mms_api::metrics::init_metrics()?;
    tracing::info!("Prometheus metrics exporter initialized");

    // Initialize database pool and run migrations
    let pool = mms_db::create_pool(&config.database_url, config.database_max_connections).await?;
    let create_db_if_missing = config.env == mms_api::config::Environment::Development;
    mms_db::ensure_db_and_migrate(&config.database_url, &pool, create_db_if_missing).await?;

    // Extract values needed after state construction, then consume config
    let allowed_origins = config.parsed_allowed_origins();
    let environment = config.env.clone();
    let port = config.port;

    // Initialize the application state (consumes config)
    let state = ApiState::new(config, pool).await?;

    // Start background jobs for periodic maintenance
    let _job_handles = mms_api::jobs::start_background_jobs(state.pool.clone());
    tracing::info!("Background jobs started (token cleanup, unverified account cleanup)");

    // Configure CORS with allowed origins from config
    let cors = mms_api::middleware::cors::create_cors_layer(allowed_origins);

    // Configure HTTP request/response tracing with request ID
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(true),
        )
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    // Create metrics endpoint (separate from main app for better isolation)
    let metrics_app = Router::new()
        .route("/metrics", get(mms_api::metrics::metrics_handler))
        .with_state(metrics_handle);

    // Create the application router with endpoint-specific rate limiting
    // Note: Rate limiting is now applied per-route in the route handlers for better granularity
    let app = mms_api::router::router()
        .merge(metrics_app)
        .with_state(state)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(mms_api::metrics::track_metrics))
        .layer(trace_layer)
        .layer(cors);

    // Apply security headers (X-Content-Type-Options, X-Frame-Options, HSTS)
    let app =
        mms_api::middleware::security_headers::apply_security_headers(app, environment.clone());

    // Start the server
    let bind_address = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!("Server starting on http://localhost:{}", port);
    tracing::info!("Environment: {:?}", environment);
    tracing::info!("Production features enabled:");
    tracing::info!("  - Prometheus metrics at /metrics");
    tracing::info!("  - Health check at /health (liveness)");
    tracing::info!("  - Readiness check at /health/ready");
    tracing::info!("  - Request ID tracing (X-Request-ID header)");
    tracing::info!("  - Background jobs (token cleanup every 6h, unverified accounts daily)");
    tracing::info!(
        "  - Endpoint-specific rate limiting (auth: 5/s, sensitive: 2/min, general: 10/s)"
    );
    tracing::info!("  - SameSite::Strict cookies");
    tracing::info!("  - Security headers (X-Content-Type-Options, X-Frame-Options, HSTS)");
    tracing::info!("  - Timing-safe responses for sensitive endpoints");

    // Create graceful shutdown signal handler
    // IMPORTANT: Use into_make_service_with_connect_info for tower_governor to extract IP addresses
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );

    // Graceful shutdown with signal handling
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    tracing::info!("Server ready to accept connections");
    graceful.await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Handle shutdown signals for graceful termination
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT (Ctrl+C), starting graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown...");
        },
    }
}
