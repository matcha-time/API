use axum::Router;

use crate::ApiState;

/// Create the flashcard routes
pub fn routes() -> Router<ApiState> {
    Router::new()
}
