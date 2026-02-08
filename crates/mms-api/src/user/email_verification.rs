use chrono::{Duration, Utc};
use sqlx::types::Uuid;
use sqlx::{PgPool, Postgres, Transaction};

use super::token::{generate_token, hash_token};
use crate::error::ApiError;

use mms_db::repositories::token as token_repo;
use mms_db::repositories::user as user_repo;

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
    let expires_at = Utc::now() + Duration::hours(expires_in_hours);

    let mut tx = pool.begin().await?;

    // Invalidate any existing unused tokens for this user
    token_repo::invalidate_verification_tokens(&mut *tx, user_id).await?;

    // Insert new token
    token_repo::insert_verification_token(&mut *tx, user_id, &token_hash, expires_at).await?;

    tx.commit().await?;

    Ok(token)
}

/// Create an email verification token within a transaction
pub async fn create_verification_token_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    expires_in_hours: i64,
) -> Result<String, ApiError> {
    // Generate the token
    let token = generate_token();
    let token_hash = hash_token(&token);

    // Calculate expiration time
    let expires_at = Utc::now() + Duration::hours(expires_in_hours);

    // Invalidate any existing unused tokens for this user
    token_repo::invalidate_verification_tokens(&mut **tx, user_id).await?;

    // Insert new token
    token_repo::insert_verification_token(&mut **tx, user_id, &token_hash, expires_at).await?;

    Ok(token)
}

/// Verify an email verification token and mark the user's email as verified
/// Returns Ok((email, true)) if email was newly verified, Ok((email, false)) if already verified
pub async fn verify_email_token(pool: &PgPool, token: &str) -> Result<(String, bool), ApiError> {
    let token_hash = hash_token(token);

    // Start a transaction to ensure both operations succeed or fail together
    let mut tx = pool.begin().await?;

    // Find the token and mark it as used
    let user_id = token_repo::consume_verification_token(&mut *tx, &token_hash)
        .await?
        .ok_or_else(|| ApiError::Auth("Invalid or expired verification token".to_string()))?;

    // Check if user's email is already verified and get the email
    let status = user_repo::find_email_verified_status(&mut *tx, user_id)
        .await?
        .ok_or_else(|| ApiError::Auth("User not found".to_string()))?;

    // If already verified, just return success without updating
    if status.email_verified {
        tx.commit().await?;
        return Ok((status.email, false));
    }

    // Mark the user's email as verified
    let updated = user_repo::mark_email_verified(&mut *tx, user_id).await?;
    if !updated {
        return Err(ApiError::NotFound("User not found".to_string()));
    }

    // Commit the transaction
    tx.commit().await?;

    Ok((status.email, true))
}

/// Clean up expired tokens (can be run periodically)
pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, ApiError> {
    let rows = token_repo::cleanup_expired_verification_tokens(pool).await?;
    Ok(rows)
}
