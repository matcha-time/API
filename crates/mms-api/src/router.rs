use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};

use crate::{auth, deck, roadmap, state::ApiState, user};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/health", get(health))
        .merge(user::routes::routes())
        .merge(deck::routes::routes())
        .merge(auth::routes::routes())
        .merge(roadmap::routes())
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
