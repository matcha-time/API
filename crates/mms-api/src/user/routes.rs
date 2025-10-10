use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;

use super::model::{User, get_dummy_users};

/// Create the user routes
pub fn routes() -> Router {
    Router::new()
        .route("/users", get(get_all_users))
        .route("/users/:id", get(get_user_by_id))
        .route("/users", post(create_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
}

/// Get all users
async fn get_all_users() -> impl IntoResponse {
    let users = get_dummy_users();
    Json(users)
}

/// Get user by ID
async fn get_user_by_id(Path(id): Path<u32>) -> impl IntoResponse {
    let users = get_dummy_users();

    if let Some(user) = users.iter().find(|u| u.id == id) {
        (StatusCode::OK, Json(user.clone())).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "User not found",
                "id": id
            })),
        )
            .into_response()
    }
}

/// Create a new user
async fn create_user(Json(payload): Json<User>) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(json!({
            "message": "User created successfully",
            "user": payload
        })),
    )
}

/// Update an existing user
async fn update_user(Path(id): Path<u32>, Json(payload): Json<User>) -> impl IntoResponse {
    let users = get_dummy_users();

    if users.iter().any(|u| u.id == id) {
        (
            StatusCode::OK,
            Json(json!({
                "message": "User updated successfully",
                "user": payload
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "User not found",
                "id": id
            })),
        )
            .into_response()
    }
}

/// Delete a user
async fn delete_user(Path(id): Path<u32>) -> impl IntoResponse {
    let users = get_dummy_users();

    if users.iter().any(|u| u.id == id) {
        (
            StatusCode::OK,
            Json(json!({
                "message": "User deleted successfully",
                "id": id
            })),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "User not found",
                "id": id
            })),
        )
            .into_response()
    }
}
