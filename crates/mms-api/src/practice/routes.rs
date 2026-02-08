use axum::{
    Json, Router,
    extract::{Path, State},
    routing::post,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

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

async fn submit_review(
    auth_user: AuthUser,
    State(state): State<ApiState>,
    Path(flashcard_id): Path<Uuid>,
    Json(payload): Json<ReviewSubmission>,
) -> Result<Json<ReviewResponse>, ApiError> {
    let user_id = auth_user.user_id;
    let now = Utc::now();

    // Single transaction for atomicity
    let mut tx = state.pool.begin().await?;

    // Verify the flashcard actually belongs to the submitted deck
    let belongs =
        practice_repo::flashcard_belongs_to_deck(&mut *tx, payload.deck_id, flashcard_id).await?;
    if !belongs {
        return Err(ApiError::Validation(
            "Flashcard does not belong to the specified deck".to_string(),
        ));
    }

    // Fetch the flashcard's correct translation
    let correct_translation =
        practice_repo::get_flashcard_translation(&mut *tx, flashcard_id).await?;

    // Fetch current progress to check if we should update
    let current_progress =
        practice_repo::get_card_progress(&mut *tx, user_id, flashcard_id).await?;

    // If review is too early, reject without revealing the answer
    let too_early = current_progress
        .as_ref()
        .is_some_and(|p| now < p.next_review_at);

    if too_early {
        tx.commit().await?;
        return Err(ApiError::Validation(
            "This card is not due for review yet".to_string(),
        ));
    }

    // Validate the user's answer by normalizing both strings
    let normalized_user_answer =
        crate::normalization::normalize_for_comparison(&payload.user_answer);
    let normalized_correct_answer =
        crate::normalization::normalize_for_comparison(&correct_translation);
    let is_correct = normalized_user_answer == normalized_correct_answer;

    let (mut new_times_correct, mut new_times_wrong) = current_progress
        .as_ref()
        .map(|p| (p.times_correct, p.times_wrong))
        .unwrap_or((0, 0));

    // Track whether this card was already mastered before this review
    let was_mastered = mms_srs::is_mastered(new_times_correct, new_times_wrong);

    if is_correct {
        new_times_correct += 1;
    } else {
        new_times_wrong += 1;
    }

    let mastered = mms_srs::is_mastered(new_times_correct, new_times_wrong);
    let newly_mastered = mastered && !was_mastered;

    // Compute the next review date based on the new score
    let next_review_at = mms_srs::compute_next_review(new_times_correct, new_times_wrong, now);

    // Update the progress (including mastered_at)
    practice_repo::upsert_card_progress(
        &mut *tx,
        user_id,
        flashcard_id,
        next_review_at,
        new_times_correct,
        new_times_wrong,
        mastered,
    )
    .await?;

    // Refresh deck progress (pass mastery threshold so SQL uses the same constant as the SRS crate)
    practice_repo::refresh_deck_progress(
        &mut *tx,
        user_id,
        payload.deck_id,
        mms_srs::MASTERY_THRESHOLD,
    )
    .await?;

    // Record activity
    practice_repo::record_activity(&mut *tx, user_id).await?;

    // Update user stats (increment total_cards_learned if newly mastered)
    let stats_updated =
        practice_repo::increment_review_stats(&mut *tx, user_id, newly_mastered).await?;
    if !stats_updated {
        tracing::warn!(user_id = %user_id, "user_stats row missing for authenticated user");
    }

    // Update streak (must run after record_activity so today's entry exists)
    practice_repo::update_streak(&mut *tx, user_id).await?;

    tx.commit().await?;

    Ok(Json(ReviewResponse {
        is_correct,
        correct_answer: correct_translation,
    }))
}
