use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use sqlx::types::Uuid;

use crate::{ApiState, auth::AuthUser, error::ApiError};

use mms_db::models::PracticeCard;
use mms_db::repositories::deck as deck_repo;

/// Create the deck routes
pub fn routes() -> Router<ApiState> {
    Router::new().route(
        "/decks/{deck_id}/practice",
        get(get_practice_session),
    )
}

async fn get_practice_session(
    auth_user: AuthUser,
    State(state): State<ApiState>,
    Path(deck_id): Path<Uuid>,
) -> Result<Json<Vec<PracticeCard>>, ApiError> {
    let cards = deck_repo::get_practice_cards(&state.pool, deck_id, auth_user.user_id)
        .await
        .map_err(ApiError::Database)?;

    Ok(Json(cards))
}
