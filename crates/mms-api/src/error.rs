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
