use axum::{
    // Json,
    Router,
    // extract::Path,
    // http::StatusCode,
    // response::IntoResponse,
    // routing::{delete, get, post, put},
};

use crate::ApiState;

/// Create the flashcard routes
pub fn routes() -> Router<ApiState> {
    Router::new()
    // .route("/flashcards", get(get_all_flashcards))
    // .route("/flashcards/{id}", get(get_flashcard_by_id))
    // .route("/flashcards", post(create_flashcard))
    // .route("/flashcards/{id}", put(update_flashcard))
    // .route("/flashcards/{id}", delete(delete_flashcard))
    //.with_state(state)
}
