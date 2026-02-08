use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;
use sqlx::types::Uuid;

use crate::{ApiState, auth::AuthUser, error::ApiError, validation};

use mms_db::models::{Roadmap, RoadmapWithProgress};
use mms_db::repositories::roadmap as roadmap_repo;

const DEFAULT_PAGE_LIMIT: i64 = 50;
const MAX_PAGE_LIMIT: i64 = 100;

#[derive(Deserialize)]
struct PaginationQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
}

impl PaginationQuery {
    fn limit(&self) -> i64 {
        self.limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT)
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

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
            "/roadmaps/{roadmap_id}/progress",
            get(get_roadmap_with_progress),
        )
}

async fn list_roadmaps(
    State(state): State<ApiState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<Roadmap>>, ApiError> {
    let roadmaps =
        roadmap_repo::list_all(&state.pool, pagination.limit(), pagination.offset()).await?;

    Ok(Json(roadmaps))
}

async fn get_roadmaps_by_language(
    State(state): State<ApiState>,
    Path((language_from, language_to)): Path<(String, String)>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<Roadmap>>, ApiError> {
    // Validate language codes
    validation::validate_language_code(&language_from)?;
    validation::validate_language_code(&language_to)?;

    let roadmaps = roadmap_repo::list_by_language(
        &state.pool,
        &language_from,
        &language_to,
        pagination.limit(),
        pagination.offset(),
    )
    .await?;

    Ok(Json(roadmaps))
}

async fn get_roadmap_nodes(
    State(state): State<ApiState>,
    Path(roadmap_id): Path<Uuid>,
) -> Result<Json<RoadmapWithProgress>, ApiError> {
    // Fetch roadmap metadata (public - no user-specific progress)
    let roadmap_metadata = roadmap_repo::get_metadata(&state.pool, roadmap_id).await?;

    // Fetch all nodes (public - no user-specific progress)
    let nodes = roadmap_repo::get_nodes(&state.pool, roadmap_id).await?;

    Ok(Json(RoadmapWithProgress {
        roadmap: roadmap_metadata,
        nodes,
    }))
}

async fn get_roadmap_with_progress(
    auth_user: AuthUser,
    State(state): State<ApiState>,
    Path(roadmap_id): Path<Uuid>,
) -> Result<Json<RoadmapWithProgress>, ApiError> {
    let user_id = auth_user.user_id;

    // Fetch roadmap metadata with progress statistics
    let roadmap_metadata =
        roadmap_repo::get_metadata_with_progress(&state.pool, roadmap_id, user_id).await?;

    // Fetch all nodes with progress
    let nodes = roadmap_repo::get_nodes_with_progress(&state.pool, roadmap_id, user_id).await?;

    Ok(Json(RoadmapWithProgress {
        roadmap: roadmap_metadata,
        nodes,
    }))
}
