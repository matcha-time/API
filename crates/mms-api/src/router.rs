use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};

use crate::{auth, deck, state::ApiState, topic, user};

pub fn router(state: ApiState) -> Router<ApiState> {
    Router::new()
        .route("/health", get(health))
        .merge(user::routes::routes(state.clone()))
        .merge(topic::routes::routes(state.clone()))
        .merge(deck::routes::routes(state.clone()))
        .merge(auth::routes::routes(state.clone()))
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
