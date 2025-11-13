use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Serialize;
use sqlx::types::Uuid;

use crate::{ApiState, auth::AuthUser};

/// Create the deck routes
pub fn routes() -> Router<ApiState> {
    Router::new().route(
        "/decks/{deck_id}/practice/{user_id}",
        get(get_practice_session),
    )
}

// NOTE: This structure can also be replaced by the original DTO if needed
#[derive(Serialize, sqlx::FromRow)]
struct PracticeCard {
    id: Uuid,
    term: String,
    translation: String,
    times_correct: i32,
    times_wrong: i32,
}

async fn get_practice_session(
    AuthUser {
        user_id: auth_user_id,
        ..
    }: AuthUser,
    State(state): State<ApiState>,
    Path((deck_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<PracticeCard>>, StatusCode> {
    // Verify the authenticated user matches the requested user
    if auth_user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    let cards = sqlx::query_as::<_, PracticeCard>(
        // language=PostgreSQL
        r#"
            SELECT 
                f.id,
                f.term,
                f.translation,
                COALESCE(ucp.times_correct, 0) as times_correct,
                COALESCE(ucp.times_wrong, 0) as times_wrong
            FROM deck_flashcards df
            JOIN flashcards f ON f.id = df.flashcard_id
            LEFT JOIN user_card_progress ucp 
                ON ucp.flashcard_id = f.id AND ucp.user_id = $2
            WHERE df.deck_id = $1
                AND (ucp.next_review_at IS NULL OR ucp.next_review_at <= NOW())
            ORDER BY ucp.next_review_at NULLS FIRST
        "#,
        // LIMIT 20 ?
    )
    .bind(deck_id)
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(cards))
}
