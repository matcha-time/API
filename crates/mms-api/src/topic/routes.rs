use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;

use crate::ApiState;

use super::model::{Topic, get_dummy_topics};

/// Create the topic routes
pub fn routes(state: ApiState) -> Router<ApiState> {
    Router::new()
        .route("/topics", get(get_all_topics))
        .route("/topics/{id}", get(get_topic_by_id))
        .route("/topics", post(create_topic))
        .route("/topics/{id}", put(update_topic))
        .route("/topics/{id}", delete(delete_topic))
        .with_state(state)
}

/// Get all topics
async fn get_all_topics() -> impl IntoResponse {
    let topics = get_dummy_topics();
    Json(topics)
}

/// Get topic by ID
async fn get_topic_by_id(Path(id): Path<u32>) -> impl IntoResponse {
    let topics = get_dummy_topics();

    if let Some(topic) = topics.iter().find(|t| t.id == id) {
        (StatusCode::OK, Json(topic.clone())).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Topic not found",
                "id": id
            })),
        )
            .into_response()
    }
}

/// Create a new topic
async fn create_topic(Json(payload): Json<Topic>) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(json!({
            "message": "Topic created successfully",
            "topic": payload
        })),
    )
}

/// Update an existing topic
async fn update_topic(Path(id): Path<u32>, Json(payload): Json<Topic>) -> impl IntoResponse {
    let topics = get_dummy_topics();

    if topics.iter().any(|t| t.id == id) {
        (
            StatusCode::OK,
            Json(json!({
                "message": "Topic updated successfully",
                "topic": payload
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Topic not found",
                "id": id
            })),
        )
            .into_response()
    }
}

/// Delete a topic
async fn delete_topic(Path(id): Path<u32>) -> impl IntoResponse {
    let topics = get_dummy_topics();

    if topics.iter().any(|t| t.id == id) {
        (
            StatusCode::OK,
            Json(json!({
                "message": "Topic deleted successfully",
                "id": id
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "Topic not found",
                "id": id
            })),
        )
            .into_response()
    }
}
