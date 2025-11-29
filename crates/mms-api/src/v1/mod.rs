use axum::Router;

use crate::{auth, deck, practice, roadmap, state::ApiState, user};

/// V1 API routes
pub fn routes() -> Router<ApiState> {
    Router::new()
        .merge(user::routes())
        .merge(deck::routes())
        .merge(auth::routes())
        .merge(roadmap::routes())
        .merge(practice::routes())
}
