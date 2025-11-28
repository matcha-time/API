use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use serde::Serialize;

use crate::{auth, deck, practice, roadmap, state::ApiState, user};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/health", get(health))
        .route("/health/ready", get(readiness))
        .merge(user::routes())
        .merge(deck::routes())
        .merge(auth::routes())
        .merge(roadmap::routes())
        .merge(practice::routes())
        .fallback(handler_404)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    database: &'static str,
    version: &'static str,
}

/// Simple liveness check - returns 200 if the server is running
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Readiness check - verifies database connectivity
async fn readiness(State(state): State<ApiState>) -> Result<Json<ReadinessResponse>, StatusCode> {
    // Check database connectivity
    let db_status = sqlx::query("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .map(|_| "connected")
        .unwrap_or("disconnected");

    if db_status == "disconnected" {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    Ok(Json(ReadinessResponse {
        status: "ready",
        database: db_status,
        version: env!("CARGO_PKG_VERSION"),
    }))
}

async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "The requested resource was not found",
    )
}
