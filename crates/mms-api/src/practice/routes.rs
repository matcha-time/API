use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use unicode_normalization::UnicodeNormalization;

use crate::{ApiState, auth::middleware::AuthUser, error::ApiError};

use mms_db::repositories::practice as practice_repo;

/// Create the practice routes
pub fn routes() -> Router<ApiState> {
    Router::new().route("/practice/{flashcard_id}/review", post(submit_review))
}

#[derive(Deserialize)]
struct ReviewSubmission {
    user_answer: String,
    deck_id: Uuid,
}

#[derive(Serialize)]
struct ReviewResponse {
    is_correct: bool,
    correct_answer: String,
}

/// Normalize a string for comparison: remove accents, lowercase, remove special characters
fn normalize_for_comparison(s: &str) -> String {
    s.to_lowercase()
        .replace('ß', "ss")
        .nfd()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

async fn submit_review(
    auth_user: AuthUser,
    State(state): State<ApiState>,
    Path(flashcard_id): Path<Uuid>,
    Json(payload): Json<ReviewSubmission>,
) -> Result<Json<ReviewResponse>, ApiError> {
    let user_id = auth_user.user_id;

    // Single transaction for atomicity
    let mut tx = state.pool.begin().await.map_err(ApiError::Database)?;

    // Fetch the flashcard's correct translation
    let correct_translation = practice_repo::get_flashcard_translation(&mut *tx, flashcard_id)
        .await
        .map_err(ApiError::Database)?;

    // Validate the user's answer by normalizing both strings
    let normalized_user_answer = normalize_for_comparison(&payload.user_answer);
    let normalized_correct_answer = normalize_for_comparison(&correct_translation);
    let is_correct = normalized_user_answer == normalized_correct_answer;

    // Fetch current progress to check if we should update
    let current_progress = practice_repo::get_card_progress(&mut *tx, user_id, flashcard_id)
        .await
        .map_err(ApiError::Database)?;

    // Check if the practice is too early
    let should_update = if let Some(progress) = &current_progress {
        // Only update if current time is past or equal to next_review_at
        Utc::now() >= progress.next_review_at
    } else {
        // First time practicing this card, always update
        true
    };

    // Only update if it's not too early
    if should_update {
        let (mut new_times_correct, mut new_times_wrong) = current_progress
            .map(|p| (p.times_correct, p.times_wrong))
            .unwrap_or((0, 0));

        if is_correct {
            new_times_correct += 1;
        } else {
            new_times_wrong += 1;
        }

        // Compute the next review date based on the new score
        let next_review_at = mms_srs::compute_next_review(new_times_correct, new_times_wrong);

        // Update the progress
        practice_repo::upsert_card_progress(
            &mut *tx,
            user_id,
            flashcard_id,
            next_review_at,
            new_times_correct,
            new_times_wrong,
        )
        .await
        .map_err(ApiError::Database)?;

        // Refresh deck progress
        practice_repo::refresh_deck_progress(&mut *tx, user_id, payload.deck_id)
            .await
            .map_err(ApiError::Database)?;

        // Record activity
        practice_repo::record_activity(&mut *tx, user_id)
            .await
            .map_err(ApiError::Database)?;

        // Update user stats
        let stats_updated = practice_repo::increment_review_stats(&mut *tx, user_id)
            .await
            .map_err(ApiError::Database)?;
        if !stats_updated {
            tracing::warn!(user_id = %user_id, "user_stats row missing for authenticated user");
        }

        // Update streak (must run after record_activity so today's entry exists)
        practice_repo::update_streak(&mut *tx, user_id)
            .await
            .map_err(ApiError::Database)?;
    }

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(ReviewResponse {
        is_correct,
        correct_answer: correct_translation,
    }))
}
