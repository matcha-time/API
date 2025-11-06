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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Oidc(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Cookie(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Jwt(e) => (StatusCode::UNAUTHORIZED, e.to_string()),
            ApiError::InvalidIdToken(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let error = Json(serde_json::json!({ "error": message }));
        (status, error).into_response()
    }
}
