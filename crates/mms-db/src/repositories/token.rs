use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

// --- Email verification tokens ---

pub async fn invalidate_verification_tokens<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE email_verification_tokens
            SET used_at = NOW()
            WHERE user_id = $1 AND used_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn insert_verification_token<'e, E>(
    executor: E,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn consume_verification_token<'e, E>(
    executor: E,
    token_hash: &str,
) -> Result<Option<Uuid>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
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
    .bind(token_hash)
    .fetch_optional(executor)
    .await
}

pub async fn cleanup_expired_verification_tokens<'e, E>(executor: E) -> Result<u64, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            DELETE FROM email_verification_tokens
            WHERE expires_at < NOW() OR used_at IS NOT NULL
        "#,
    )
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

// --- Password reset tokens ---

pub async fn invalidate_reset_tokens<'e, E>(executor: E, user_id: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE user_id = $1 AND used_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn insert_reset_token<'e, E>(
    executor: E,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn consume_reset_token<'e, E>(
    executor: E,
    token_hash: &str,
) -> Result<Option<Uuid>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
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
    .bind(token_hash)
    .fetch_optional(executor)
    .await
}

pub async fn cleanup_expired_reset_tokens<'e, E>(executor: E) -> Result<u64, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            DELETE FROM password_reset_tokens
            WHERE expires_at < NOW() OR used_at IS NOT NULL
        "#,
    )
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}
