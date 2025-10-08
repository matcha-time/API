use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};

#[derive(Clone)]
pub struct AppState {}

pub fn router() -> Router {
    Router::new()
        .route("/", get(health))
        .fallback(handler_404)
        .with_state(AppState {})
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "The requested resource was not found",
    )
}
