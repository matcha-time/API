use mms_api::{config::ApiConfig, state::ApiState};
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables
    dotenvy::dotenv().ok();
    let config = ApiConfig::from_env()?;

    // Initialize the application state
    let state = ApiState::new(config).await?;

    // Configure CORS
    // let allowed_origins = std::env::var("ALLOWED_ORIGINS")
    //     .unwrap_or_else(|_| "http://localhost:8080".to_string())
    //     .split(',')
    //     .filter_map(|s| s.trim().parse::<axum::http::HeaderValue>().ok())
    //     .collect::<Vec<_>>();

    // let cors = CorsLayer::new()
    //     .allow_origin(AllowOrigin::list(allowed_origins))
    //     .allow_methods([
    //         Method::GET,
    //         Method::POST,
    //         Method::PUT,
    //         Method::PATCH,
    //         Method::DELETE,
    //         Method::OPTIONS,
    //     ])
    //     .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
    //     .allow_credentials(true);

    // Create the application router
    let app = mms_api::router::router()
        .with_state(state)
        .layer(CorsLayer::very_permissive());

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
