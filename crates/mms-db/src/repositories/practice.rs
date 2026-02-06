use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::CardProgress;

pub async fn get_flashcard_translation<'e, E>(
    executor: E,
    flashcard_id: Uuid,
) -> Result<String, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result: (String,) = sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT translation
            FROM flashcards
            WHERE id = $1
        "#,
    )
    .bind(flashcard_id)
    .fetch_one(executor)
    .await?;
    Ok(result.0)
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
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
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
        "#,
    )
    .bind(user_id)
    .bind(flashcard_id)
    .bind(next_review_at)
    .bind(times_correct)
    .bind(times_wrong)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn refresh_deck_progress<'e, E>(
    executor: E,
    user_id: Uuid,
    deck_id: Uuid,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            SELECT refresh_deck_progress($1, $2)
        "#,
    )
    .bind(user_id)
    .bind(deck_id)
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

pub async fn increment_review_stats<'e, E>(executor: E, user_id: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
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
    .execute(executor)
    .await?;
    Ok(())
}
