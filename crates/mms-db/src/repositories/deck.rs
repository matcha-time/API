use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::PracticeCard;

pub async fn get_practice_cards<'e, E>(
    executor: E,
    deck_id: Uuid,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<PracticeCard>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
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
            LIMIT $3
        "#,
    )
    .bind(deck_id)
    .bind(user_id)
    .bind(limit)
    .fetch_all(executor)
    .await
}
