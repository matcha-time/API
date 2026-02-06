use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::{Roadmap, RoadmapMetadata, RoadmapNodeWithProgress};

pub async fn list_all<'e, E>(executor: E) -> Result<Vec<Roadmap>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, title, description, language_from, language_to
            FROM roadmaps
            ORDER BY created_at DESC
        "#,
    )
    .fetch_all(executor)
    .await
}

pub async fn list_by_language<'e, E>(executor: E, language_from: &str, language_to: &str) -> Result<Vec<Roadmap>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, title, description, language_from, language_to
            FROM roadmaps
            WHERE language_from = $1 AND language_to = $2
        "#,
    )
    .bind(language_from)
    .bind(language_to)
    .fetch_all(executor)
    .await
}

pub async fn get_metadata<'e, E>(executor: E, roadmap_id: Uuid) -> Result<RoadmapMetadata, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT
                r.id,
                r.title,
                r.description,
                r.language_from,
                r.language_to,
                COUNT(rn.id)::int as total_nodes,
                0::int as completed_nodes,
                0.0::float8 as progress_percentage
            FROM roadmaps r
            LEFT JOIN roadmap_nodes rn ON rn.roadmap_id = r.id
            WHERE r.id = $1
            GROUP BY r.id, r.title, r.description, r.language_from, r.language_to
        "#,
    )
    .bind(roadmap_id)
    .fetch_one(executor)
    .await
}

pub async fn get_nodes<'e, E>(executor: E, roadmap_id: Uuid) -> Result<Vec<RoadmapNodeWithProgress>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT
                rn.id as node_id,
                rn.parent_node_id,
                rn.pos_x,
                rn.pos_y,
                d.id as deck_id,
                d.title as deck_title,
                d.description as deck_description,
                (SELECT COUNT(*)::int FROM deck_flashcards df WHERE df.deck_id = d.id) as total_cards,
                0::int as mastered_cards,
                0::int as cards_due_today,
                0::int as total_practices,
                NULL::timestamptz as last_practiced_at,
                0.0::float8 as progress_percentage
            FROM roadmap_nodes rn
            JOIN decks d ON d.id = rn.deck_id
            WHERE rn.roadmap_id = $1
            ORDER BY rn.pos_y, rn.pos_x
        "#,
    )
    .bind(roadmap_id)
    .fetch_all(executor)
    .await
}

pub async fn get_metadata_with_progress<'e, E>(executor: E, roadmap_id: Uuid, user_id: Uuid) -> Result<RoadmapMetadata, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT
                r.id,
                r.title,
                r.description,
                r.language_from,
                r.language_to,
                COUNT(rn.id)::int as total_nodes,
                COUNT(rn.id) FILTER (
                    WHERE udp.mastered_cards > 0
                    AND udp.mastered_cards = udp.total_cards
                )::int as completed_nodes,
                CASE
                    WHEN COUNT(rn.id) > 0 THEN
                        (COUNT(rn.id) FILTER (
                            WHERE udp.mastered_cards > 0
                            AND udp.mastered_cards = udp.total_cards
                        )::float8 / COUNT(rn.id)::float8 * 100.0)
                    ELSE 0.0
                END as progress_percentage
            FROM roadmaps r
            LEFT JOIN roadmap_nodes rn ON rn.roadmap_id = r.id
            LEFT JOIN user_deck_progress udp
                ON udp.deck_id = rn.deck_id AND udp.user_id = $2
            WHERE r.id = $1
            GROUP BY r.id, r.title, r.description, r.language_from, r.language_to
        "#,
    )
    .bind(roadmap_id)
    .bind(user_id)
    .fetch_one(executor)
    .await
}

pub async fn get_nodes_with_progress<'e, E>(executor: E, roadmap_id: Uuid, user_id: Uuid) -> Result<Vec<RoadmapNodeWithProgress>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT
                rn.id as node_id,
                rn.parent_node_id,
                rn.pos_x,
                rn.pos_y,
                d.id as deck_id,
                d.title as deck_title,
                d.description as deck_description,
                COALESCE(udp.total_cards, (
                    SELECT COUNT(*)::int FROM deck_flashcards df WHERE df.deck_id = d.id
                )) as total_cards,
                COALESCE(udp.mastered_cards, 0) as mastered_cards,
                COALESCE(udp.cards_due_today, 0) as cards_due_today,
                COALESCE(udp.total_practices, 0) as total_practices,
                udp.last_practiced_at,
                COALESCE(udp.progress_percentage, 0.0)::float8 as progress_percentage
            FROM roadmap_nodes rn
            JOIN decks d ON d.id = rn.deck_id
            LEFT JOIN user_deck_progress udp
                ON udp.deck_id = d.id AND udp.user_id = $2
            WHERE rn.roadmap_id = $1
            ORDER BY rn.pos_y, rn.pos_x
        "#,
    )
    .bind(roadmap_id)
    .bind(user_id)
    .fetch_all(executor)
    .await
}
