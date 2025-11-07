use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;

use crate::ApiState;

use super::model::{Deck, get_dummy_decks};

/// Create the deck routes
pub fn routes(_: ApiState) -> Router<ApiState> {
    Router::new()
        .route("/decks", get(get_all_decks))
        .route("/decks/{id}", get(get_deck_by_id))
        .route("/decks", post(create_deck))
        .route("/decks/{id}", put(update_deck))
        .route("/decks/{id}", delete(delete_deck))
        .route("/decks/topic/{topic_id}", get(get_decks_by_topic))
    //.with_state(state)
}

/// Get all decks
async fn get_all_decks() -> impl IntoResponse {
    let decks = get_dummy_decks();
    Json(decks)
}

/// Get deck by ID
async fn get_deck_by_id(Path(id): Path<u32>) -> impl IntoResponse {
    let decks = get_dummy_decks();

    if let Some(deck) = decks.iter().find(|d| d.id == id) {
        (StatusCode::OK, Json(deck.clone())).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Deck not found",
                "id": id
            })),
        )
            .into_response()
    }
}

/// Get decks by topic ID
async fn get_decks_by_topic(Path(topic_id): Path<u32>) -> impl IntoResponse {
    let decks = get_dummy_decks();
    let filtered_decks: Vec<Deck> = decks
        .into_iter()
        .filter(|d| d.topic_id == topic_id)
        .collect();

    if filtered_decks.is_empty() {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "No decks found for topic",
                "topic_id": topic_id
            })),
        )
            .into_response()
    } else {
        (StatusCode::OK, Json(filtered_decks)).into_response()
    }
}

/// Create a new deck
async fn create_deck(Json(payload): Json<Deck>) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(json!({
            "message": "Deck created successfully",
            "deck": payload
        })),
    )
}

/// Update an existing deck
async fn update_deck(Path(id): Path<u32>, Json(payload): Json<Deck>) -> impl IntoResponse {
    let decks = get_dummy_decks();

    if decks.iter().any(|d| d.id == id) {
        (
            StatusCode::OK,
            Json(json!({
                "message": "Deck updated successfully",
                "deck": payload
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Deck not found",
                "id": id
            })),
        )
            .into_response()
    }
}

/// Delete a deck
async fn delete_deck(Path(id): Path<u32>) -> impl IntoResponse {
    let decks = get_dummy_decks();

    if decks.iter().any(|d| d.id == id) {
        (
            StatusCode::OK,
            Json(json!({
                "message": "Deck deleted successfully",
                "id": id
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Deck not found",
                "id": id
            })),
        )
            .into_response()
    }
}
