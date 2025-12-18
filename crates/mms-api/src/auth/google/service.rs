use crate::error::ApiError;
use sqlx::{PgPool, types::Uuid};

/// Represents a user in the database
#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub profile_picture_url: Option<String>,
    pub native_language: Option<String>,
    pub learning_language: Option<String>,
}

// TODO: Refacto this whole ass thing

/// Find or create a user from Google OAuth
///
/// This function will:
/// 1. Check if a user exists with this Google ID
/// 2. If not, check if a user exists with this email
/// 3. If not, create a new user
///
/// Returns the user's ID, username, email, and profile picture URL
pub async fn find_or_create_google_user(
    pool: &PgPool,
    google_id: &str,
    email: &str,
    name: Option<&str>,
    picture: Option<&str>,
) -> Result<User, ApiError> {
    // First, try to find existing user by Google ID
    if let Some(user) = sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, Option<String>)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, profile_picture_url, native_language, learning_language
            FROM users
            WHERE google_id = $1
        "#,
    )
    .bind(google_id)
    .fetch_optional(pool)
    .await?
    {
        // Update profile picture if it has changed
        if picture.is_some() && picture != user.3.as_deref() {
            sqlx::query(
                // language=PostgreSQL
                r#"
                    UPDATE users
                    SET profile_picture_url = $1
                    WHERE id = $2
                "#,
            )
            .bind(picture)
            .bind(user.0)
            .execute(pool)
            .await?;
        }

        return Ok(User {
            id: user.0,
            username: user.1,
            email: user.2,
            profile_picture_url: picture.map(|p| p.to_string()).or(user.3),
            native_language: user.4,
            learning_language: user.5,
        });
    }

    // If not found by Google ID, check if user exists with this email
    // This handles the case where user registered with email/password first
    if let Some(user) = sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, Option<String>, Option<String>)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, google_id, profile_picture_url, native_language, learning_language
            FROM users
            WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?
    {
        // If user exists but doesn't have google_id, link the Google account
        if user.3.is_none() {
            sqlx::query(
                // language=PostgreSQL
                r#"
                    UPDATE users
                    SET google_id = $1, auth_provider = 'google', profile_picture_url = $2
                    WHERE id = $3
                "#,
            )
            .bind(google_id)
            .bind(picture)
            .bind(user.0)
            .execute(pool)
            .await?;
        } else if picture.is_some() && picture != user.4.as_deref() {
            // Update profile picture if it has changed
            sqlx::query(
                // language=PostgreSQL
                r#"
                    UPDATE users
                    SET profile_picture_url = $1
                    WHERE id = $2
                "#,
            )
            .bind(picture)
            .bind(user.0)
            .execute(pool)
            .await?;
        }

        return Ok(User {
            id: user.0,
            username: user.1,
            email: user.2,
            profile_picture_url: picture.map(|p| p.to_string()).or(user.4),
            native_language: user.5,
            learning_language: user.6,
        });
    }

    // User doesn't exist, create a new one
    // Generate username from name or email
    let username = name.map(|n| n.to_string()).unwrap_or_else(|| {
        // Extract username from email (part before @)
        email.split('@').next().unwrap_or(email).to_string()
    });

    // Handle potential username conflicts by appending a number
    let mut final_username = username.clone();
    let mut counter = 1;

    loop {
        match sqlx::query_scalar::<_, Uuid>(
            // language=PostgreSQL
            r#"
                INSERT INTO users (username, email, google_id, auth_provider, profile_picture_url)
                VALUES ($1, $2, $3, 'google', $4)
                RETURNING id
            "#,
        )
        .bind(&final_username)
        .bind(email)
        .bind(google_id)
        .bind(picture)
        .fetch_one(pool)
        .await
        {
            Ok(user_id) => {
                // Create user_stats entry
                sqlx::query(
                    // language=PostgreSQL
                    r#"
                        INSERT INTO user_stats (user_id)
                        VALUES ($1)
                    "#,
                )
                .bind(user_id)
                .execute(pool)
                .await?;

                return Ok(User {
                    id: user_id,
                    username: final_username,
                    email: email.to_string(),
                    profile_picture_url: picture.map(|p| p.to_string()),
                    native_language: None,
                    learning_language: None,
                });
            }
            Err(sqlx::Error::Database(db_err))
                if db_err.constraint() == Some("users_username_key") =>
            {
                // Username conflict, try with a number suffix
                counter += 1;
                final_username = format!("{}{}", username, counter);
            }
            Err(e) => return Err(e.into()),
        }
    }
}
