use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use chrono::{DateTime, Utc};
use mms_srs::compute_next_review;
use serde::Deserialize;
use sqlx::types::Uuid;
use unicode_normalization::UnicodeNormalization;

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
    user_answer: String,
    deck_id: Uuid,
}

/// Normalize a string for comparison: remove accents, lowercase, remove special characters
fn normalize_for_comparison(s: &str) -> String {
    s.nfd()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

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

    // Fetch the flashcard's correct translation
    let flashcard: (String,) = sqlx::query_as(
        // language=PostgreSQL
        r#"
        SELECT translation
        FROM flashcards
        WHERE id = $1
        "#,
    )
    .bind(flashcard_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    let correct_translation = flashcard.0;

    // Validate the user's answer by normalizing both strings
    let normalized_user_answer = normalize_for_comparison(&payload.user_answer);
    let normalized_correct_answer = normalize_for_comparison(&correct_translation);
    let is_correct = normalized_user_answer == normalized_correct_answer;

    // Fetch current progress to check if we should update
    let current_progress: Option<(DateTime<Utc>, i32, i32)> = sqlx::query_as(
        // language=PostgreSQL
        r#"
        SELECT next_review_at, times_correct, times_wrong
        FROM user_card_progress
        WHERE user_id = $1 AND flashcard_id = $2
        "#,
    )
    .bind(user_id)
    .bind(flashcard_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    // Check if the practice is too early
    let should_update = if let Some((next_review_at, _, _)) = current_progress {
        // Only update if current time is past or equal to next_review_at
        Utc::now() >= next_review_at
    } else {
        // First time practicing this card, always update
        true
    };

    // Only update if it's not too early
    if should_update {
        let (new_times_correct, new_times_wrong) = match current_progress {
            Some((_, times_correct, times_wrong)) => {
                if is_correct {
                    (times_correct + 1, times_wrong)
                } else {
                    (times_correct, times_wrong + 1)
                }
            }
            None => {
                if is_correct {
                    (1, 0)
                } else {
                    (0, 1)
                }
            }
        };

        // Compute the next review date based on the new score
        let next_review_at = compute_next_review(new_times_correct, new_times_wrong);

        // Update the progress
        sqlx::query(
            // language=PostgreSQL
            r#"
            INSERT INTO user_card_progress (user_id, flashcard_id, next_review_at, last_review_at, times_correct, times_wrong)
            VALUES ($1, $2, $3, NOW(), $4, $5)
            ON CONFLICT (user_id, flashcard_id)
            DO UPDATE SET
                next_review_at = $3,
                last_review_at = NOW(),
                times_correct = $4,
                times_wrong = $5,
                updated_at = NOW()
            "#
        )
        .bind(user_id)
        .bind(flashcard_id)
        .bind(next_review_at)
        .bind(new_times_correct)
        .bind(new_times_wrong)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        // Refresh deck progress
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
    }

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(StatusCode::OK)
}
