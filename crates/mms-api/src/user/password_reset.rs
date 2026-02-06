use chrono::{Duration, Utc};
use sqlx::PgPool;
use sqlx::types::Uuid;

use super::token::{generate_token, hash_token};
use crate::error::ApiError;

use mms_db::repositories::auth as auth_repo;
use mms_db::repositories::token as token_repo;
use mms_db::repositories::user as user_repo;

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
    let expires_at = Utc::now() + Duration::hours(expires_in_hours);

    let mut tx = pool.begin().await?;

    // Invalidate any existing unused tokens for this user
    token_repo::invalidate_reset_tokens(&mut *tx, user_id).await?;

    // Insert new token
    token_repo::insert_reset_token(&mut *tx, user_id, &token_hash, expires_at).await?;

    tx.commit().await?;

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
    let user_id = token_repo::consume_reset_token(&mut *tx, &token_hash)
        .await?
        .ok_or_else(|| ApiError::Auth("Invalid or expired reset token".to_string()))?;

    // Update the user's password
    user_repo::update_password_for_email_user(&mut *tx, user_id, new_password_hash).await?;

    // Revoke all existing refresh tokens for security
    // This ensures any stolen tokens cannot be used after password reset
    auth_repo::delete_all_user_refresh_tokens(&mut *tx, user_id).await?;

    // Get user email and username for confirmation email
    let user_info = user_repo::find_email_and_name(&mut *tx, user_id).await?;

    // Commit the transaction
    tx.commit().await?;

    Ok((user_info.email, user_info.username))
}

/// Clean up expired tokens (can be run periodically)
pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, ApiError> {
    let rows = token_repo::cleanup_expired_reset_tokens(pool).await?;
    Ok(rows)
}
