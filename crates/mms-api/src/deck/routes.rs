use axum::{
    //    Json,
    Router,
    //     extract::Path,
    //     http::StatusCode,
    //     response::IntoResponse,
    //     routing::{delete, get, post, put},
};

use crate::ApiState;

/// Create the deck routes
pub fn routes() -> Router<ApiState> {
    Router::new()
    // .route("/decks", get(get_all_decks))
    // .route("/decks/{id}", get(get_deck_by_id))
    // .route("/decks", post(create_deck))
    // .route("/decks/{id}", put(update_deck))
    // .route("/decks/{id}", delete(delete_deck))
    // .route("/decks/topic/{topic_id}", get(get_decks_by_topic))
    //.with_state(state)
}
