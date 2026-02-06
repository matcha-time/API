//! Background jobs for periodic maintenance tasks.
//!
//! This module provides scheduled cleanup tasks that complement the database triggers.
//! While triggers handle cleanup opportunistically on INSERT operations, these jobs
//! ensure cleanup happens even during periods of low activity.

use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::time::interval;

/// Start all background jobs
///
/// Returns a vector of join handles that can be awaited on shutdown
pub fn start_background_jobs(pool: PgPool) -> Vec<tokio::task::JoinHandle<()>> {
    vec![
        tokio::spawn(periodic_token_cleanup_job(pool.clone())),
        tokio::spawn(periodic_unverified_accounts_cleanup_job(pool)),
    ]
}

/// Run the database cleanup_all_expired_tokens() function every 6 hours
///
/// This complements the automatic triggers by ensuring cleanup happens
/// even during periods of low INSERT activity
async fn periodic_token_cleanup_job(pool: PgPool) {
    // Wait 1 hour before first run to avoid startup contention
    tokio::time::sleep(Duration::from_secs(3600)).await;

    let mut interval = interval(Duration::from_secs(21600)); // 6 hours

    loop {
        interval.tick().await;

        match run_token_cleanup(&pool).await {
            Ok((pr, ev, rt, total)) if total > 0 => {
                tracing::info!(
                    "Token cleanup complete: {} password reset, {} email verification, {} refresh tokens ({} total)",
                    pr,
                    ev,
                    rt,
                    total
                );
            }
            Ok(_) => {
                tracing::debug!("Token cleanup complete: no expired tokens found");
            }
            Err(e) => {
                tracing::error!("Failed to run periodic token cleanup: {}", e);
            }
        }
    }
}

/// Clean up unverified accounts older than 7 days, runs daily
///
/// This removes accounts where users never verified their email
async fn periodic_unverified_accounts_cleanup_job(pool: PgPool) {
    // Wait 2 hours before first run
    tokio::time::sleep(Duration::from_secs(7200)).await;

    let mut interval = interval(Duration::from_secs(86400)); // 24 hours

    loop {
        interval.tick().await;

        match cleanup_unverified_accounts(&pool).await {
            Ok(deleted) if deleted > 0 => {
                tracing::info!(
                    "Cleaned up {} unverified accounts older than 7 days",
                    deleted
                );
            }
            Ok(_) => {
                tracing::debug!("No old unverified accounts to clean up");
            }
            Err(e) => {
                tracing::error!("Failed to clean up unverified accounts: {}", e);
            }
        }
    }
}

/// Call the database function to clean up all expired tokens
///
/// Returns tuple of (password_reset, email_verification, refresh_tokens, total)
async fn run_token_cleanup(pool: &PgPool) -> Result<(i32, i32, i32, i32), sqlx::Error> {
    let result = sqlx::query(
        r#"
        SELECT
            password_reset_cleaned,
            email_verification_cleaned,
            refresh_tokens_cleaned,
            total_cleaned
        FROM cleanup_all_expired_tokens()
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok((
        result.try_get("password_reset_cleaned").unwrap_or(0),
        result.try_get("email_verification_cleaned").unwrap_or(0),
        result.try_get("refresh_tokens_cleaned").unwrap_or(0),
        result.try_get("total_cleaned").unwrap_or(0),
    ))
}

/// Delete unverified accounts older than 7 days
async fn cleanup_unverified_accounts(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        DELETE FROM users
        WHERE email_verified = false
        AND created_at < NOW() - INTERVAL '7 days'
        "#,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}