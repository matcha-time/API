use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Serialize;
use sqlx::types::Uuid;

use crate::ApiState;

use mms_db::models::{ActivityDay, UserStats};

/// Create the user routes
pub fn routes() -> Router<ApiState> {
    Router::new().route("/users/{user_id}/dashboard", get(get_user_dashboard))
}

#[derive(Serialize)]
struct UserDashboard {
    stats: UserStats,
    heatmap: Vec<ActivityDay>,
}

// TODO: make this two database calls concurrent or on two different routes
async fn get_user_dashboard(
    State(state): State<ApiState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserDashboard>, StatusCode> {
    let stats = sqlx::query_as::<_, UserStats>(
        // language=PostgreSQL
        r#"
            SELECT current_streak_days, longest_streak_days, total_reviews, total_cards_learned, last_review_date
            FROM user_stats WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::NOT_FOUND)?;

    let heatmap = sqlx::query_as::<_, ActivityDay>(
        // language=PostgreSQL
        r#"
            SELECT activity_date, reviews_count
            FROM user_activity
            WHERE user_id = $1 AND activity_date >= CURRENT_DATE - 365
            ORDER BY activity_date
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UserDashboard { stats, heatmap }))
}
