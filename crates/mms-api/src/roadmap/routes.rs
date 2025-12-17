use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use sqlx::types::Uuid;

use crate::{ApiState, auth::AuthUser, error::ApiError, validation};

use mms_db::models::{Roadmap, RoadmapMetadata, RoadmapNodeWithProgress, RoadmapWithProgress};

/// Create the roadmap routes
pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/roadmaps", get(list_roadmaps))
        .route(
            "/roadmaps/{language_from}/{language_to}",
            get(get_roadmaps_by_language),
        )
        .route("/roadmaps/{roadmap_id}/nodes", get(get_roadmap_nodes))
        .route(
            "/roadmaps/{roadmap_id}/progress/{user_id}",
            get(get_roadmap_with_progress),
        )
}

async fn list_roadmaps(State(state): State<ApiState>) -> Result<Json<Vec<Roadmap>>, ApiError> {
    let roadmaps = sqlx::query_as::<_, Roadmap>(
        // language=PostgreSQL
        r#"
            SELECT id, title, description, language_from, language_to
            FROM roadmaps
            ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(roadmaps))
}

async fn get_roadmaps_by_language(
    State(state): State<ApiState>,
    Path((language_from, language_to)): Path<(String, String)>,
) -> Result<Json<Vec<Roadmap>>, ApiError> {
    // Validate language codes
    validation::validate_language_code(&language_from)?;
    validation::validate_language_code(&language_to)?;

    let roadmaps = sqlx::query_as::<_, Roadmap>(
        // language=PostgreSQL
        r#"
            SELECT id, title, description, language_from, language_to
            FROM roadmaps
            WHERE language_from = $1 AND language_to = $2
        "#,
    )
    .bind(language_from)
    .bind(language_to)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(roadmaps))
}

async fn get_roadmap_nodes(
    State(state): State<ApiState>,
    Path(roadmap_id): Path<Uuid>,
) -> Result<Json<RoadmapWithProgress>, ApiError> {
    // Fetch roadmap metadata (public - no user-specific progress)
    let roadmap_metadata = sqlx::query_as::<_, RoadmapMetadata>(
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
    .fetch_one(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    // Fetch all nodes (public - no user-specific progress)
    let nodes = sqlx::query_as::<_, RoadmapNodeWithProgress>(
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
                NULL::timestamptz as last_practiced_at
            FROM roadmap_nodes rn
            JOIN decks d ON d.id = rn.deck_id
            WHERE rn.roadmap_id = $1
            ORDER BY rn.pos_y, rn.pos_x
        "#,
    )
    .bind(roadmap_id)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(RoadmapWithProgress {
        roadmap: roadmap_metadata,
        nodes,
    }))
}

async fn get_roadmap_with_progress(
    AuthUser {
        user_id: auth_user_id,
        ..
    }: AuthUser,
    State(state): State<ApiState>,
    Path((roadmap_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RoadmapWithProgress>, ApiError> {
    // Verify the authenticated user matches the requested user
    if auth_user_id != user_id {
        return Err(ApiError::Auth(
            "You are not authorized to access this roadmap progress".to_string(),
        ));
    }

    // Fetch roadmap metadata with progress statistics
    let roadmap_metadata = sqlx::query_as::<_, RoadmapMetadata>(
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
    .fetch_one(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    // Fetch all nodes with progress
    let nodes = sqlx::query_as::<_, RoadmapNodeWithProgress>(
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
                udp.last_practiced_at
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
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(RoadmapWithProgress {
        roadmap: roadmap_metadata,
        nodes,
    }))
}
