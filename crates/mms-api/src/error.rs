use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("OIDC error: {0}")]
    Oidc(String),
    #[error("Cookie error: {0}")]
    Cookie(String),
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("Invalid ID token: {0}")]
    InvalidIdToken(String),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Password hashing error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("Email error: {0}")]
    Email(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Oidc(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Cookie(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Jwt(e) => {
                tracing::error!(error = %e, "JWT error occurred");
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid or expired token".to_string(),
                )
            }
            ApiError::InvalidIdToken(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Bcrypt(e) => {
                tracing::error!(error = %e, "Password hashing error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal error occurred. Please try again later.".to_string(),
                )
            }
            ApiError::Email(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Database(e) => {
                // Log the actual error for debugging
                tracing::error!(error = %e, "Database error occurred");

                // Never expose internal database errors to users
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal error occurred. Please try again later.".to_string(),
                )
            }
        };

        let error = Json(serde_json::json!({ "error": message }));
        (status, error).into_response()
    }
}
