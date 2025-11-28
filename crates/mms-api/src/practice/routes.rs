use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use serde::Deserialize;
use sqlx::types::Uuid;

use crate::{ApiState, auth::middleware::AuthUser, error::ApiError};

/// Create the practice routes
pub fn routes() -> Router<ApiState> {
    Router::new().route(
        "/practice/{user_id}/{flashcard_id}/review",
        post(submit_review),
    )
}

#[derive(Deserialize)]
struct ReviewSubmission {
    correct: bool,
    next_review_at: chrono::DateTime<chrono::Utc>,
    deck_id: Uuid,
}

// NOTE:
// Here we can change the flow and validate the translation
// We can also compute & set the SRS datas when on a correct submition
async fn submit_review(
    auth_user: AuthUser,
    State(state): State<ApiState>,
    Path((user_id, flashcard_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<ReviewSubmission>,
) -> Result<StatusCode, ApiError> {
    // Authorization check: ensure the authenticated user matches the user_id in the path
    if auth_user.user_id != user_id {
        return Err(ApiError::Auth(
            "You are not authorized to submit reviews for this user".to_string(),
        ));
    }
    // Single transaction for atomicity
    let mut tx = state.pool.begin().await.map_err(ApiError::Database)?;

    sqlx::query(
        // language=PostgreSQL
        r#"
        INSERT INTO user_card_progress (user_id, flashcard_id, next_review_at, last_review_at, times_correct, times_wrong)
        VALUES ($1, $2, $3, NOW(), $4, $5)
        ON CONFLICT (user_id, flashcard_id) 
        DO UPDATE SET
            next_review_at = EXCLUDED.next_review_at,
            last_review_at = NOW(),
            times_correct = user_card_progress.times_correct + EXCLUDED.times_correct,
            times_wrong = user_card_progress.times_wrong + EXCLUDED.times_wrong
        "#
    )
    .bind(user_id)
    .bind(flashcard_id)
    .bind(payload.next_review_at)
    .bind(if payload.correct { 1 } else { 0 })
    .bind(if payload.correct { 0 } else { 1 })
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        // language=PostgreSQL
        r#"
            SELECT refresh_deck_progress($1, $2)
        "#,
    )
    .bind(user_id)
    .bind(payload.deck_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    // Record activity
    sqlx::query(
        // language=PostgreSQL
        r#"
        INSERT INTO user_activity (user_id, activity_date, reviews_count)
        VALUES ($1, CURRENT_DATE, 1)
        ON CONFLICT (user_id, activity_date)
        DO UPDATE SET reviews_count = user_activity.reviews_count + 1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    // Update user stats
    sqlx::query(
        // language=PostgreSQL
        r#"
        UPDATE user_stats
        SET total_reviews = total_reviews + 1,
            last_review_date = CURRENT_DATE,
            updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(StatusCode::OK)
}
