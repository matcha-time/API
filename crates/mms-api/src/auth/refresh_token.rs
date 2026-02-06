use base64::Engine;
use chrono::Utc;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, types::Uuid};

use crate::error::ApiError;

use mms_db::repositories::auth as auth_repo;

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
    expiry_days: i64,
) -> Result<Uuid, ApiError> {
    let expires_at = Utc::now() + chrono::Duration::days(expiry_days);

    let token_id = auth_repo::store_refresh_token(pool, user_id, token_hash, device_info, ip_address, expires_at)
        .await
        .map_err(ApiError::Database)?;

    Ok(token_id)
}

/// Verify a refresh token and return the user_id if valid
/// Also updates the last_used_at timestamp and rotates the token
pub async fn verify_and_rotate_refresh_token(
    pool: &PgPool,
    token: &str,
    expiry_days: i64,
) -> Result<(Uuid, String, String), ApiError> {
    // Hash the incoming token
    let mut hasher = Sha256::new();
    hasher.update(token);
    let token_hash = format!("{:x}", hasher.finalize());

    // Start a transaction for atomic token rotation
    let mut tx = pool.begin().await?;

    // Fetch and verify the token
    let record = auth_repo::find_refresh_token_by_hash(&mut *tx, &token_hash)
        .await?
        .ok_or_else(|| ApiError::Auth("Invalid refresh token".to_string()))?;

    // Check if token is expired
    if record.expires_at < Utc::now() {
        // Delete expired token
        auth_repo::delete_refresh_token(&mut *tx, record.id).await?;
        tx.commit().await?;
        return Err(ApiError::Auth("Refresh token expired".to_string()));
    }

    // Token is valid - delete the old token
    auth_repo::delete_refresh_token(&mut *tx, record.id).await?;

    // Generate a new refresh token
    let (new_token, new_token_hash) = generate_refresh_token();
    let new_expires_at = Utc::now() + chrono::Duration::days(expiry_days);

    // Store the new refresh token
    auth_repo::store_refresh_token(
        &mut *tx,
        record.user_id,
        &new_token_hash,
        record.device_info.as_deref(),
        record.ip_address.as_deref(),
        new_expires_at,
    )
    .await?;

    tx.commit().await?;

    Ok((record.user_id, new_token, new_token_hash))
}

/// Revoke a specific refresh token
pub async fn revoke_refresh_token(pool: &PgPool, token: &str) -> Result<(), ApiError> {
    // Hash the token
    let mut hasher = Sha256::new();
    hasher.update(token);
    let token_hash = format!("{:x}", hasher.finalize());

    let rows = auth_repo::delete_refresh_token_by_hash(pool, &token_hash).await?;

    if rows == 0 {
        return Err(ApiError::Auth("Refresh token not found".to_string()));
    }

    Ok(())
}

/// Revoke all refresh tokens for a user (logout from all devices)
pub async fn revoke_all_user_tokens(pool: &PgPool, user_id: Uuid) -> Result<u64, ApiError> {
    let rows = auth_repo::delete_all_user_refresh_tokens(pool, user_id).await?;
    Ok(rows)
}

/// Clean up expired refresh tokens (should be run periodically)
/// Note: As of migration 0006, this is automatically handled by database triggers
/// when new tokens are created. This function can still be called manually if needed.
pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, ApiError> {
    let rows = auth_repo::cleanup_expired_refresh_tokens(pool).await?;
    Ok(rows)
}
