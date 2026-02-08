use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::models::{
    ActivityDay, EmailVerifiedStatus, UserCredentials, UserEmailAndName, UserExistenceCheck,
    UserIdAndName, UserPasswordInfo, UserProfile, UserStats, UserVerificationInfo,
};

pub async fn find_profile_by_id<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<Option<UserProfile>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, profile_picture_url, native_language, learning_language
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await
}

pub async fn find_credentials_by_email<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserCredentials>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, password_hash, profile_picture_url, email_verified, native_language, learning_language
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await
}

pub async fn find_existence_by_email<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserExistenceCheck>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, email_verified
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await
}

pub async fn create_email_user<'e, E>(
    executor: E,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<Uuid, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
        // language=PostgreSQL
        r#"
            INSERT INTO users (username, email, password_hash, auth_provider)
            VALUES ($1, $2, $3, 'email')
            RETURNING id
        "#,
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(executor)
    .await
}

pub async fn create_user_stats<'e, E>(executor: E, user_id: Uuid) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO user_stats (user_id)
            VALUES ($1)
        "#,
    )
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn find_id_and_name_by_email<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserIdAndName>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, username
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await
}

pub async fn find_verification_info_by_email<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserVerificationInfo>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT id, username, email_verified
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(email)
    .fetch_optional(executor)
    .await
}

pub async fn find_email_verified_status<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<Option<EmailVerifiedStatus>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT email, email_verified
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await
}

pub async fn find_password_info<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<Option<UserPasswordInfo>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT email, username, password_hash, auth_provider::text
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await
}

pub async fn update_language_preferences<'e, E>(
    executor: E,
    user_id: Uuid,
    native_language: &str,
    learning_language: &str,
) -> Result<UserProfile, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET native_language = $1, learning_language = $2
            WHERE id = $3
            RETURNING id, username, email, profile_picture_url, native_language, learning_language
        "#,
    )
    .bind(native_language)
    .bind(learning_language)
    .bind(user_id)
    .fetch_one(executor)
    .await
}

pub async fn update_password_for_email_user<'e, E>(
    executor: E,
    user_id: Uuid,
    password_hash: &str,
) -> Result<bool, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET password_hash = $1
            WHERE id = $2 AND auth_provider = 'email'
        "#,
    )
    .bind(password_hash)
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_username<'e, E>(
    executor: E,
    user_id: Uuid,
    username: &str,
) -> Result<String, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET username = $1
            WHERE id = $2
            RETURNING username
        "#,
    )
    .bind(username)
    .bind(user_id)
    .fetch_one(executor)
    .await
}

pub async fn mark_email_verified<'e, E>(executor: E, user_id: Uuid) -> Result<bool, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET email_verified = TRUE
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_user<'e, E>(executor: E, user_id: Uuid) -> Result<u64, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            DELETE FROM users WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_user_stats<'e, E>(executor: E, user_id: Uuid) -> Result<UserStats, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT current_streak_days, longest_streak_days, total_reviews, total_cards_learned, last_review_date
            FROM user_stats WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(executor)
    .await
}

pub async fn get_user_activity<'e, E>(
    executor: E,
    user_id: Uuid,
    days: i32,
) -> Result<Vec<ActivityDay>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT activity_date, reviews_count
            FROM user_activity
            WHERE user_id = $1 AND activity_date >= CURRENT_DATE - $2
            ORDER BY activity_date
        "#,
    )
    .bind(user_id)
    .bind(days)
    .fetch_all(executor)
    .await
}

pub async fn find_email_and_name<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<UserEmailAndName, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        // language=PostgreSQL
        r#"
            SELECT email, username
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(executor)
    .await
}
