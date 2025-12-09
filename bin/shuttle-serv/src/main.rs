use axum::{Router, middleware, routing::get};
use mms_api::{config::ApiConfig, state::ApiState};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] pool: sqlx::PgPool,
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> shuttle_axum::ShuttleAxum {
    // Load configuration from Shuttle secrets
    let config = ApiConfig::from_shuttle_secrets(&secrets)
        .map_err(|e| anyhow::anyhow!("Config error: {}", e))?;

    // Note: Shuttle already initializes tracing, so we skip our custom init
    // The Shuttle runtime provides default tracing subscriber

    // Initialize Prometheus metrics exporter
    let metrics_handle = mms_api::metrics::init_metrics()?;
    tracing::info!("Prometheus metrics exporter initialized");

    // Run migrations on Shuttle-provided pool
    sqlx::migrate!("../../crates/mms-db/migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Migration error: {}", e))?;

    // Initialize the application state with the provided pool
    let state = ApiState::new(config.clone(), pool).await?;

    // Start background jobs for periodic maintenance
    let _job_handles = mms_api::jobs::start_background_jobs(state.pool.clone());
    tracing::info!("Background jobs started (token cleanup, unverified account cleanup)");

    // Configure CORS with allowed origins from config
    let cors = mms_api::middleware::cors::create_cors_layer(config.parsed_allowed_origins());

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
        .layer(cors)
        .layer(trace_layer)
        .layer(middleware::from_fn(mms_api::metrics::track_metrics))
        .layer(middleware::from_fn(
            mms_api::middleware::request_id::request_id_middleware,
        ));

    // Apply security headers (X-Content-Type-Options, X-Frame-Options, HSTS)
    let app =
        mms_api::middleware::security_headers::apply_security_headers(app, config.env.clone());

    tracing::info!("Environment: {:?}", config.env);
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

    // Note: Shuttle.rs handles the service conversion internally
    // We return the plain Router and Shuttle will serve it with ConnectInfo support
    Ok(app.into())
}
