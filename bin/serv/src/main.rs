use mms_api::{config::ApiConfig, state::ApiState};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config = ApiConfig::from_env()?;

    // Initialize tracing/logging based on environment
    mms_api::tracing::init_tracing(&config.env);

    // Initialize the application state
    let state = ApiState::new(config.clone()).await?;

    // Configure CORS with allowed origins from config
    let cors = mms_api::middleware::cors::create_cors_layer(config.parsed_allowed_origins());

    // Configure rate limiting with values from config
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(config.rate_limit_per_second)
        .burst_size(config.rate_limit_burst_size)
        .finish()
        .expect("Failed to build rate limiter configuration");
    let rate_limit = GovernorLayer::new(governor_conf);

    // Configure HTTP request/response tracing
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    // Create the application router
    let app = mms_api::router::router()
        .with_state(state)
        .layer(trace_layer)
        .layer(rate_limit)
        .layer(cors);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server starting on http://localhost:3000");
    tracing::info!("Environment: {:?}", config.env);
    tracing::info!(
        "Rate limit: {} req/s, burst: {}",
        config.rate_limit_per_second,
        config.rate_limit_burst_size
    );
    axum::serve(listener, app).await?;

    Ok(())
}
