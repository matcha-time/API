use crate::error::ApiError;
use sqlx::{PgPool, types::Uuid};

/// Represents a user in the database
#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
}

/// Find or create a user from Google OAuth
///
/// This function will:
/// 1. Check if a user exists with this Google ID
/// 2. If not, check if a user exists with this email
/// 3. If not, create a new user
///
/// Returns the user's ID, username, and email
pub async fn find_or_create_google_user(
    pool: &PgPool,
    google_id: &str,
    email: &str,
    name: Option<&str>,
) -> Result<User, ApiError> {
    // First, try to find existing user by Google ID
    if let Some(user) = sqlx::query_as::<_, (Uuid, String, String)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email
            FROM users
            WHERE google_id = $1
        "#,
    )
    .bind(google_id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(User {
            id: user.0,
            username: user.1,
            email: user.2,
        });
    }

    // If not found by Google ID, check if user exists with this email
    // This handles the case where user registered with email/password first
    if let Some(user) = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, google_id
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
                    SET google_id = $1, auth_provider = 'google'
                    WHERE id = $2
                "#,
            )
            .bind(google_id)
            .bind(user.0)
            .execute(pool)
            .await?;
        }

        return Ok(User {
            id: user.0,
            username: user.1,
            email: user.2,
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
                INSERT INTO users (username, email, google_id, auth_provider)
                VALUES ($1, $2, $3, 'google')
                RETURNING id
            "#,
        )
        .bind(&final_username)
        .bind(email)
        .bind(google_id)
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
