use mms_api::{config::ApiConfig, state::ApiState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    dotenvy::dotenv().ok();
    let config = ApiConfig::from_env()?;

    // Initialize the application state
    let state = ApiState::new(config).await?;

    // Create the application router
    let app = mms_api::router::router(state.clone()).with_state(state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
