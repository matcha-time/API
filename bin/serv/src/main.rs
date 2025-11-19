use mms_api::{config::ApiConfig, state::ApiState};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    let config = ApiConfig::from_env()?;

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

    // Create the application router
    let app = mms_api::router::router()
        .with_state(state)
        .layer(rate_limit)
        .layer(cors);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
