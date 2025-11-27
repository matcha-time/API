use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};

use crate::{auth, deck, practice, roadmap, state::ApiState, user};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/health", get(health))
        .merge(user::routes())
        .merge(deck::routes())
        .merge(auth::routes())
        .merge(roadmap::routes())
        .merge(practice::routes())
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
