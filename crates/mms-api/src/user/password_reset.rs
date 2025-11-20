use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use sqlx::types::Uuid;

use super::token::{generate_token, hash_token};
use crate::error::ApiError;

/// Create a password reset token in the database
pub async fn create_reset_token(
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

/// Verify a reset token, update password, and mark token as used (all in one transaction)
/// Returns (email, username) on success for sending confirmation email
pub async fn verify_and_reset_password(
    pool: &PgPool,
    token: &str,
    new_password_hash: &str,
) -> Result<(String, String), ApiError> {
    let token_hash = hash_token(token);

    // Start transaction to ensure atomicity
    let mut tx = pool.begin().await?;

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
    .fetch_optional(&mut *tx)
    .await?;

    let user_id = result
        .map(|(user_id,)| user_id)
        .ok_or_else(|| ApiError::Auth("Invalid or expired reset token".to_string()))?;

    // Update the user's password
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET password_hash = $1
            WHERE id = $2 AND auth_provider = 'email'
        "#,
    )
    .bind(new_password_hash)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // Get user email and username for confirmation email
    let (email, username) = sqlx::query_as::<_, (String, String)>(
        // language=PostgreSQL
        r#"
            SELECT email, username
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    // Commit the transaction
    tx.commit().await?;

    Ok((email, username))
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
