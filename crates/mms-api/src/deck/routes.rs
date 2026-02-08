use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;
use sqlx::types::Uuid;

use crate::{ApiState, auth::AuthUser, error::ApiError};

use mms_db::models::PracticeCard;
use mms_db::repositories::deck as deck_repo;

const DEFAULT_PRACTICE_LIMIT: i64 = 20;
const MAX_PRACTICE_LIMIT: i64 = 50;

/// Create the deck routes
pub fn routes() -> Router<ApiState> {
    Router::new().route("/decks/{deck_id}/practice", get(get_practice_session))
}

#[derive(Deserialize)]
struct PracticeQuery {
    #[serde(default)]
    limit: Option<i64>,
}

async fn get_practice_session(
    auth_user: AuthUser,
    State(state): State<ApiState>,
    Path(deck_id): Path<Uuid>,
    Query(query): Query<PracticeQuery>,
) -> Result<Json<Vec<PracticeCard>>, ApiError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PRACTICE_LIMIT)
        .clamp(1, MAX_PRACTICE_LIMIT);

    let cards =
        deck_repo::get_practice_cards(&state.pool, deck_id, auth_user.user_id, limit).await?;

    Ok(Json(cards))
}
