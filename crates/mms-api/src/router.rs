use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};

use crate::user;

#[derive(Clone)]
pub struct AppState {}

pub fn router() -> Router {
    Router::new()
        .route("/", get(health))
        .merge(user::routes::routes())
        .fallback(handler_404)
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
