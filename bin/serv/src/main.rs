use mms_api::{config::ApiConfig, state::ApiState};
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

    // Configure HTTP request/response tracing
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    // Create the application router with endpoint-specific rate limiting
    // Note: Rate limiting is now applied per-route in the route handlers for better granularity
    let app = mms_api::router::router()
        .with_state(state)
        .layer(trace_layer)
        .layer(cors);

    // Apply security headers (X-Content-Type-Options, X-Frame-Options, HSTS)
    let app = mms_api::middleware::security_headers::apply_security_headers(app, config.env.clone());

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Server starting on http://localhost:3000");
    tracing::info!("Environment: {:?}", config.env);
    tracing::info!("Security features enabled:");
    tracing::info!("  - Endpoint-specific rate limiting (auth: 5/s, sensitive: 2/min, general: 10/s)");
    tracing::info!("  - SameSite::Strict cookies");
    tracing::info!("  - Security headers (X-Content-Type-Options, X-Frame-Options, HSTS)");
    tracing::info!("  - Timing-safe responses for sensitive endpoints");
    axum::serve(listener, app).await?;

    Ok(())
}
