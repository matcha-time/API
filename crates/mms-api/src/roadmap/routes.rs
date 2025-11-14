use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use sqlx::types::Uuid;

use crate::{ApiState, auth::AuthUser, error::ApiError, validation};

use mms_db::models::{Roadmap, RoadmapNodeWithProgress};

/// Create the roadmap routes
pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/roadmaps", get(list_roadmaps))
        .route(
            "/roadmaps/{language_from}/{language_to}",
            get(get_roadmaps_by_language),
        )
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

async fn get_roadmap_with_progress(
    AuthUser {
        user_id: auth_user_id,
        ..
    }: AuthUser,
    State(state): State<ApiState>,
    Path((roadmap_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<RoadmapNodeWithProgress>>, ApiError> {
    // Verify the authenticated user matches the requested user
    if auth_user_id != user_id {
        return Err(ApiError::Auth(
            "You are not authorized to access this roadmap progress".to_string(),
        ));
    }

    let nodes = sqlx::query_as::<_, RoadmapNodeWithProgress>(
        // language=PostgreSQL
        r#"
            SELECT 
                rn.id as node_id,
                rn.pos_x,
                rn.pos_y,
                d.id as deck_id,
                d.title as deck_title,
                COALESCE(udp.total_cards, 0) as total_cards,
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

    Ok(Json(nodes))
}
