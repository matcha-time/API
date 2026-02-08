use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::CardProgress;

/// Verify that a flashcard belongs to a given deck.
pub async fn flashcard_belongs_to_deck<'e, E>(
    executor: E,
    deck_id: Uuid,
    flashcard_id: Uuid,
) -> Result<bool, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let exists: bool = sqlx::query_scalar(
        // language=PostgreSQL
        r#"
            SELECT EXISTS(
                SELECT 1 FROM deck_flashcards
                WHERE deck_id = $1 AND flashcard_id = $2
            )
        "#,
    )
    .bind(deck_id)
    .bind(flashcard_id)
    .fetch_one(executor)
    .await?;
    Ok(exists)
}

pub async fn get_flashcard_translation<'e, E>(
    executor: E,
    flashcard_id: Uuid,
) -> Result<String, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
        // language=PostgreSQL
        r#"
            SELECT translation
            FROM flashcards
            WHERE id = $1
        "#,
    )
    .bind(flashcard_id)
    .fetch_one(executor)
    .await
}

pub async fn get_card_progress<'e, E>(
    executor: E,
    user_id: Uuid,
    flashcard_id: Uuid,
) -> Result<Option<CardProgress>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT next_review_at, times_correct, times_wrong
            FROM user_card_progress
            WHERE user_id = $1 AND flashcard_id = $2
        "#,
    )
    .bind(user_id)
    .bind(flashcard_id)
    .fetch_optional(executor)
    .await
}

pub async fn upsert_card_progress<'e, E>(
    executor: E,
    user_id: Uuid,
    flashcard_id: Uuid,
    next_review_at: DateTime<Utc>,
    times_correct: i32,
    times_wrong: i32,
    mastered: bool,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO user_card_progress (user_id, flashcard_id, next_review_at, last_review_at, times_correct, times_wrong, mastered_at)
            VALUES ($1, $2, $3, NOW(), $4, $5, CASE WHEN $6 THEN NOW() ELSE NULL END)
            ON CONFLICT (user_id, flashcard_id)
            DO UPDATE SET
                next_review_at = $3,
                last_review_at = NOW(),
                times_correct = $4,
                times_wrong = $5,
                mastered_at = CASE WHEN $6 THEN COALESCE(user_card_progress.mastered_at, NOW()) ELSE NULL END,
                updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(flashcard_id)
    .bind(next_review_at)
    .bind(times_correct)
    .bind(times_wrong)
    .bind(mastered)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn refresh_deck_progress<'e, E>(
    executor: E,
    user_id: Uuid,
    deck_id: Uuid,
    mastery_threshold: i32,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            SELECT refresh_deck_progress($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(deck_id)
    .bind(mastery_threshold)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn record_activity<'e, E>(executor: E, user_id: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
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
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn increment_review_stats<'e, E>(
    executor: E,
    user_id: Uuid,
    newly_mastered: bool,
) -> Result<bool, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE user_stats
            SET total_reviews = total_reviews + 1,
                total_cards_learned = total_cards_learned + CASE WHEN $2 THEN 1 ELSE 0 END,
                last_review_date = CURRENT_DATE,
                updated_at = NOW()
            WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(newly_mastered)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_streak<'e, E>(executor: E, user_id: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            SELECT calculate_and_update_streak($1)
        "#,
    )
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(())
}
