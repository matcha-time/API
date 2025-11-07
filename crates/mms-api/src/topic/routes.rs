use axum::{
    // Json,
    Router,
    // extract::Path,
    // http::StatusCode,
    // response::IntoResponse,
    // routing::{delete, get, post, put},
};

use crate::ApiState;

/// Create the topic routes
pub fn routes() -> Router<ApiState> {
    Router::new()
    // .route("/topics", get(get_all_topics))
    // .route("/topics/{id}", get(get_topic_by_id))
    // .route("/topics", post(create_topic))
    // .route("/topics/{id}", put(update_topic))
    // .route("/topics/{id}", delete(delete_topic))
    // .with_state(state)
}
