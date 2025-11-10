use axum::{
    // Json,
    Router,
    // extract::Path,
    // http::StatusCode,
    // response::IntoResponse,
    // routing::{delete, get, post, put},
};

use crate::ApiState;

/// Create the roadmap routes
pub fn routes() -> Router<ApiState> {
    Router::new()
    // .route("/roadmaps", get(get_all_roadmaps))
    // .route("/roadmaps/{id}", get(get_roadmap_by_id))
    // .route("/roadmaps", post(create_roadmap))
    // .route("/roadmaps/{id}", put(update_roadmap))
    // .route("/roadmaps/{id}", delete(delete_roadmap))
    //.with_state(state)
}
