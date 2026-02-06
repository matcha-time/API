use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::{RefreshTokenRecord, UserProfile, UserWithGoogleId};

pub async fn find_by_google_id<'e, E>(
    executor: E,
    google_id: &str,
) -> Result<Option<UserProfile>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, profile_picture_url, native_language, learning_language
            FROM users
            WHERE google_id = $1
        "#,
    )
    .bind(google_id)
    .fetch_optional(executor)
    .await
}

pub async fn find_by_email_with_google_id<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserWithGoogleId>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, google_id, profile_picture_url, native_language, learning_language
            FROM users
            WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await
}

pub async fn link_google_account<'e, E>(
    executor: E,
    user_id: Uuid,
    google_id: &str,
    picture: Option<&str>,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET google_id = $1, profile_picture_url = COALESCE($2, profile_picture_url)
            WHERE id = $3
        "#,
    )
    .bind(google_id)
    .bind(picture)
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_profile_picture<'e, E>(
    executor: E,
    user_id: Uuid,
    picture_url: &str,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET profile_picture_url = $1
            WHERE id = $2
        "#,
    )
    .bind(picture_url)
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn create_google_user<'e, E>(
    executor: E,
    username: &str,
    email: &str,
    google_id: &str,
    picture: Option<&str>,
) -> Result<Uuid, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
        // language=PostgreSQL
        r#"
            INSERT INTO users (username, email, google_id, auth_provider, profile_picture_url)
            VALUES ($1, $2, $3, 'google', $4)
            RETURNING id
        "#,
    )
    .bind(username)
    .bind(email)
    .bind(google_id)
    .bind(picture)
    .fetch_one(executor)
    .await
}

pub async fn store_refresh_token<'e, E>(
    executor: E,
    user_id: Uuid,
    token_hash: &str,
    device_info: Option<&str>,
    ip_address: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
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
    .fetch_one(executor)
    .await
}

pub async fn find_refresh_token_by_hash<'e, E>(
    executor: E,
    token_hash: &str,
) -> Result<Option<RefreshTokenRecord>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, user_id, expires_at, device_info, ip_address
            FROM refresh_tokens
            WHERE token_hash = $1
            FOR UPDATE
        "#,
    )
    .bind(token_hash)
    .fetch_optional(executor)
    .await
}

pub async fn delete_refresh_token<'e, E>(executor: E, token_id: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
        .bind(token_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn delete_refresh_token_by_hash<'e, E>(
    executor: E,
    token_hash: &str,
) -> Result<u64, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
        .bind(token_hash)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

pub async fn delete_all_user_refresh_tokens<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<u64, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

pub async fn cleanup_expired_refresh_tokens<'e, E>(executor: E) -> Result<u64, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query("SELECT cleanup_expired_refresh_tokens()")
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}
