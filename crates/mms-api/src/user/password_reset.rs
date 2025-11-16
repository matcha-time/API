use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::types::Uuid;
use sqlx::PgPool;

use crate::error::ApiError;

/// Generate a secure random token for password reset
pub fn generate_reset_token() -> String {
    let mut rng = rand::thread_rng();
    let token_bytes: [u8; 32] = rng.r#gen();
    hex::encode(token_bytes)
}

/// Hash a token for secure storage in the database
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Create a password reset token in the database
pub async fn create_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    expires_in_hours: i64,
) -> Result<String, ApiError> {
    // Generate the token
    let token = generate_reset_token();
    let token_hash = hash_token(&token);

    // Calculate expiration time
    let expires_at: DateTime<Utc> = Utc::now() + Duration::hours(expires_in_hours);

    // Invalidate any existing unused tokens for this user
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE user_id = $1 AND used_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    // Insert new token
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(token)
}

/// Verify a reset token and return the associated user_id
pub async fn verify_reset_token(pool: &PgPool, token: &str) -> Result<Uuid, ApiError> {
    let token_hash = hash_token(token);

    // Find the token and mark it as used
    let result = sqlx::query_as::<_, (Uuid,)>(
        // language=PostgreSQL
        r#"
            UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE token_hash = $1
                AND used_at IS NULL
                AND expires_at > NOW()
            RETURNING user_id
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    result
        .map(|(user_id,)| user_id)
        .ok_or_else(|| ApiError::Auth("Invalid or expired reset token".to_string()))
}

/// Clean up expired tokens (can be run periodically)
pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, ApiError> {
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            DELETE FROM password_reset_tokens
            WHERE expires_at < NOW() OR used_at IS NOT NULL
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
