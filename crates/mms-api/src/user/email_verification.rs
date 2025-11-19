use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use sqlx::types::Uuid;

use crate::error::ApiError;
use super::token::{generate_token, hash_token};

/// Create an email verification token in the database
pub async fn create_verification_token(
    pool: &PgPool,
    user_id: Uuid,
    expires_in_hours: i64,
) -> Result<String, ApiError> {
    // Generate the token
    let token = generate_token();
    let token_hash = hash_token(&token);

    // Calculate expiration time
    let expires_at: DateTime<Utc> = Utc::now() + Duration::hours(expires_in_hours);

    // Invalidate any existing unused tokens for this user
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE email_verification_tokens
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
            INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
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

/// Verify an email verification token and mark the user's email as verified
pub async fn verify_email_token(pool: &PgPool, token: &str) -> Result<Uuid, ApiError> {
    let token_hash = hash_token(token);

    // Start a transaction to ensure both operations succeed or fail together
    let mut tx = pool.begin().await?;

    // Find the token and mark it as used
    let result = sqlx::query_as::<_, (Uuid,)>(
        // language=PostgreSQL
        r#"
            UPDATE email_verification_tokens
            SET used_at = NOW()
            WHERE token_hash = $1
                AND used_at IS NULL
                AND expires_at > NOW()
            RETURNING user_id
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let user_id = result
        .map(|(user_id,)| user_id)
        .ok_or_else(|| ApiError::Auth("Invalid or expired verification token".to_string()))?;

    // Mark the user's email as verified
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET email_verified = TRUE
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // Commit the transaction
    tx.commit().await?;

    Ok(user_id)
}

/// Clean up expired tokens (can be run periodically)
pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, ApiError> {
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            DELETE FROM email_verification_tokens
            WHERE expires_at < NOW() OR used_at IS NOT NULL
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
