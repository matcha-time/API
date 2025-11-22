use base64::Engine;
use chrono::Utc;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, types::Uuid};

use crate::error::ApiError;

/// Duration in days for refresh token expiry
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 30;

/// Generate a cryptographically secure random refresh token
/// Returns the token string (to send to client) and its SHA-256 hash (to store in DB)
pub fn generate_refresh_token() -> (String, String) {
    // Generate 32 random bytes (256 bits)
    let mut token_bytes = [0u8; 32];
    rand::thread_rng().fill(&mut token_bytes);

    // Encode as base64 for safe transmission
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes);

    // Hash the token for storage
    let mut hasher = Sha256::new();
    hasher.update(&token);
    let token_hash = format!("{:x}", hasher.finalize());

    (token, token_hash)
}

/// Store a refresh token in the database
pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    device_info: Option<&str>,
    ip_address: Option<&str>,
) -> Result<Uuid, ApiError> {
    let expires_at = Utc::now() + chrono::Duration::days(REFRESH_TOKEN_EXPIRY_DAYS);

    let token_id = sqlx::query_scalar::<_, Uuid>(
        // language=PostgreSQL
        r#"
            INSERT INTO refresh_tokens (user_id, token_hash, device_info, ip_address, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(device_info)
    .bind(ip_address)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(token_id)
}

/// Verify a refresh token and return the user_id if valid
/// Also updates the last_used_at timestamp and rotates the token
pub async fn verify_and_rotate_refresh_token(
    pool: &PgPool,
    token: &str,
) -> Result<(Uuid, String, String), ApiError> {
    // Hash the incoming token
    let mut hasher = Sha256::new();
    hasher.update(token);
    let token_hash = format!("{:x}", hasher.finalize());

    // Start a transaction for atomic token rotation
    let mut tx = pool.begin().await?;

    // Fetch and verify the token
    let result = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            chrono::DateTime<Utc>,
            Option<String>,
            Option<String>,
        ),
    >(
        // language=PostgreSQL
        r#"
            SELECT id, user_id, expires_at, device_info, ip_address
            FROM refresh_tokens
            WHERE token_hash = $1
            FOR UPDATE
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let (token_id, user_id, expires_at, device_info, ip_address) =
        result.ok_or_else(|| ApiError::Auth("Invalid refresh token".to_string()))?;

    // Check if token is expired
    if expires_at < Utc::now() {
        // Delete expired token
        sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
            .bind(token_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        return Err(ApiError::Auth("Refresh token expired".to_string()));
    }

    // Token is valid - delete the old token
    sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
        .bind(token_id)
        .execute(&mut *tx)
        .await?;

    // Generate a new refresh token
    let (new_token, new_token_hash) = generate_refresh_token();
    let new_expires_at = Utc::now() + chrono::Duration::days(REFRESH_TOKEN_EXPIRY_DAYS);

    // Store the new refresh token
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO refresh_tokens (user_id, token_hash, device_info, ip_address, expires_at)
            VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(user_id)
    .bind(&new_token_hash)
    .bind(device_info)
    .bind(ip_address)
    .bind(new_expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((user_id, new_token, new_token_hash))
}

/// Revoke a specific refresh token
pub async fn revoke_refresh_token(pool: &PgPool, token: &str) -> Result<(), ApiError> {
    // Hash the token
    let mut hasher = Sha256::new();
    hasher.update(token);
    let token_hash = format!("{:x}", hasher.finalize());

    let result = sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::Auth("Refresh token not found".to_string()));
    }

    Ok(())
}

/// Revoke all refresh tokens for a user (logout from all devices)
pub async fn revoke_all_user_tokens(pool: &PgPool, user_id: Uuid) -> Result<u64, ApiError> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Clean up expired refresh tokens (should be run periodically)
pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, ApiError> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < NOW()")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
