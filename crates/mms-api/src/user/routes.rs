use axum::{
    // Json,
    Router,
    // extract::Path,
    // http::StatusCode,
    // response::IntoResponse,
    // routing::{delete, get, post, put},
};

use crate::ApiState;

/// Create the user routes
pub fn routes() -> Router<ApiState> {
    Router::new()
    // .route("/users", get(get_all_users))
    // .route("/users/{id}", get(get_user_by_id))
    // .route("/users", post(create_user))
    // .route("/users/{id}", put(update_user))
    // .route("/users/{id}", delete(delete_user))
    // .with_state(state)
}
