use crate::error::ApiError;
use mms_db::models::UserProfile;
use sqlx::PgPool;

use mms_db::repositories::auth as auth_repo;
use mms_db::repositories::user as user_repo;

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
) -> Result<UserProfile, ApiError> {
    // First, try to find existing user by Google ID
    if let Some(user) = auth_repo::find_by_google_id(pool, google_id).await? {
        // Update profile picture if it has changed
        if picture.is_some() && picture != user.profile_picture_url.as_deref() {
            if let Some(pic) = picture {
                auth_repo::update_profile_picture(pool, user.id, pic).await?;
            }
        }

        return Ok(UserProfile {
            profile_picture_url: picture.map(|p| p.to_string()).or(user.profile_picture_url),
            ..user
        });
    }

    // If not found by Google ID, check if user exists with this email
    // This handles the case where user registered with email/password first
    if let Some(user) = auth_repo::find_by_email_with_google_id(pool, email).await? {
        // If user exists but doesn't have google_id, link the Google account
        if user.google_id.is_none() {
            auth_repo::link_google_account(pool, user.id, google_id, picture).await?;
        } else if picture.is_some() && picture != user.profile_picture_url.as_deref() {
            // Update profile picture if it has changed
            if let Some(pic) = picture {
                auth_repo::update_profile_picture(pool, user.id, pic).await?;
            }
        }

        return Ok(UserProfile {
            id: user.id,
            username: user.username,
            email: user.email,
            profile_picture_url: picture.map(|p| p.to_string()).or(user.profile_picture_url),
            native_language: user.native_language,
            learning_language: user.learning_language,
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
        match auth_repo::create_google_user(pool, &final_username, email, google_id, picture).await
        {
            Ok(user_id) => {
                // Create user_stats entry
                user_repo::create_user_stats(pool, user_id).await?;

                return Ok(UserProfile {
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
