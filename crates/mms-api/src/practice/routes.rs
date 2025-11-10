use axum::{
    // Json,
    Router,
    // extract::Path,
    // http::StatusCode,
    // response::IntoResponse,
    // routing::{delete, get, post, put},
};

use crate::ApiState;

/// Create the practice routes
pub fn routes() -> Router<ApiState> {
    Router::new()
    // .route("/practices", get(get_all_practices))
    // .route("/practices/{id}", get(get_practice_by_id))
    // .route("/practices", post(create_practice))
    // .route("/practices/{id}", put(update_practice))
    // .route("/practices/{id}", delete(delete_practice))
    //.with_state(state)
}
