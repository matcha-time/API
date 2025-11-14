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
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Password hashing error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Oidc(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Cookie(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Jwt(e) => (StatusCode::UNAUTHORIZED, e.to_string()),
            ApiError::InvalidIdToken(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Bcrypt(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::Database(e) => {
                // Handle specific database errors
                if let sqlx::Error::Database(db_err) = &e
                    && db_err.constraint().is_some()
                {
                    return (
                        StatusCode::CONFLICT,
                        Json(serde_json::json!({ "error": "User already exists" })),
                    )
                        .into_response();
                }
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        };

        let error = Json(serde_json::json!({ "error": message }));
        (status, error).into_response()
    }
}
