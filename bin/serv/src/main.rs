#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = mms_api::router::router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
