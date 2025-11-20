use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, post},
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::{
    ApiState,
    auth::{
        self, AuthUser, jwt,
        routes::{AuthResponse, UserResponse},
    },
    error::ApiError,
    user::{email_verification, password_reset},
};

use mms_db::models::{ActivityDay, UserStats};

/// Create the user routes
pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/users/register", post(create_user))
        .route("/users/login", post(login_user))
        .route("/users/{user_id}/dashboard", get(get_user_dashboard))
        .route("/users/{user_id}", delete(delete_user))
        .route("/users/verify-email", get(verify_email))
        .route(
            "/users/resend-verification",
            post(resend_verification_email),
        )
        .route(
            "/users/request-password-reset",
            post(request_password_reset),
        )
        .route("/users/reset-password", post(reset_password))
}

#[derive(Serialize)]
struct UserDashboard {
    stats: UserStats,
    heatmap: Vec<ActivityDay>,
}

async fn get_user_dashboard(
    auth: AuthUser,
    State(state): State<ApiState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserDashboard>, ApiError> {
    // Verify the authenticated user matches the requested user
    if auth.user_id != user_id {
        return Err(ApiError::Auth(
            "You are not authorized to access this dashboard".to_string(),
        ));
    }

    let stats = sqlx::query_as::<_, UserStats>(
        // language=PostgreSQL
        r#"
            SELECT current_streak_days, longest_streak_days, total_reviews, total_cards_learned, last_review_date
            FROM user_stats WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    let heatmap = sqlx::query_as::<_, ActivityDay>(
        // language=PostgreSQL
        r#"
            SELECT activity_date, reviews_count
            FROM user_activity
            WHERE user_id = $1 AND activity_date >= CURRENT_DATE - 365
            ORDER BY activity_date
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(UserDashboard { stats, heatmap }))
}

#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

async fn create_user(
    State(state): State<ApiState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validate input
    auth::validation::validate_email(&request.email)?;
    auth::validation::validate_password(&request.password)?;
    auth::validation::validate_username(&request.username)?;

    // Check if user already exists
    let existing_user = sqlx::query_as::<_, (Uuid, bool)>(
        // language=PostgreSQL
        r#"
            SELECT id, email_verified
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(&request.email)
    .fetch_optional(&state.pool)
    .await?;

    // If user exists and is verified, tell them to login
    if let Some((_, true)) = existing_user {
        return Err(ApiError::Validation(
            "An account with this email already exists. Please log in.".to_string(),
        ));
    }

    // If user exists but is not verified, resend verification email
    if let Some((user_id, false)) = existing_user {
        let verification_token =
            email_verification::create_verification_token(&state.pool, user_id, 24).await?;

        if let Some(email_service) = &state.email_service {
            email_service.send_verification_email(
                &request.email,
                &request.username,
                &verification_token,
            )?;
        } else {
            eprintln!(
                "Email service not configured. Verification token for user {}: {}",
                user_id, verification_token
            );
        }

        return Ok(Json(serde_json::json!({
            "message": "A verification email has been resent. Please check your email to verify your account.",
            "email": request.email
        })));
    }

    // Start a transaction for user creation
    let mut tx = state.pool.begin().await?;

    // Hash the password
    let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;

    // Insert user into database
    let user_id = sqlx::query_scalar::<_, Uuid>(
        // language=PostgreSQL
        r#"
            INSERT INTO users (username, email, password_hash, auth_provider)
            VALUES ($1, $2, $3, 'email')
            RETURNING id
        "#,
    )
    .bind(&request.username)
    .bind(&request.email)
    .bind(&password_hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        // Handle unique constraint violations gracefully
        if e.to_string().contains("duplicate key") {
            if e.to_string().contains("username") {
                ApiError::Validation("Username is already taken.".to_string())
            } else {
                ApiError::Validation("An account with this email already exists.".to_string())
            }
        } else {
            ApiError::Database(e)
        }
    })?;

    // Create user_stats entry
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO user_stats (user_id)
            VALUES ($1)
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // Generate verification token (24 hour expiry)
    let verification_token =
        email_verification::create_verification_token(&state.pool, user_id, 24).await?;

    // Commit the transaction before sending email
    tx.commit().await?;

    // Send verification email if email service is configured
    // Note: If this fails, user is created but email not sent
    // They can use the resend endpoint or re-register
    if let Some(email_service) = &state.email_service {
        if let Err(e) = email_service.send_verification_email(
            &request.email,
            &request.username,
            &verification_token,
        ) {
            eprintln!("Failed to send verification email: {}", e);
            // Don't fail the request, user can resend later
        }
    } else {
        // If email service is not configured, log the verification URL to the console
        eprintln!(
            "Email service not configured. Verification token for user {}: {}",
            user_id, verification_token
        );
    }

    Ok(Json(serde_json::json!({
        "message": "Registration successful. Please check your email to verify your account.",
        "email": request.email
    })))
}

async fn login_user(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Json(request): Json<LoginRequest>,
) -> Result<(PrivateCookieJar, Json<AuthResponse>), ApiError> {
    // Fetch user from database
    let user = sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>, bool)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, password_hash, profile_picture_url, email_verified
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(&request.email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::Auth("Invalid email or password".to_string()))?;

    let (id, username, email, password_hash, profile_picture_url, email_verified) = user;

    // Verify password exists and matches
    let password_hash =
        password_hash.ok_or_else(|| ApiError::Auth("Invalid email or password".to_string()))?;

    if !bcrypt::verify(&request.password, &password_hash)? {
        return Err(ApiError::Auth("Invalid email or password".to_string()));
    }

    // Check if email is verified
    if !email_verified {
        return Err(ApiError::Auth(
            "Please verify your email address before logging in. Check your inbox for the verification link.".to_string()
        ));
    }

    // Generate JWT token
    let token = jwt::generate_jwt_token(id, email.clone(), &state.jwt_secret)?;

    // Set auth cookie with JWT
    let auth_cookie = jwt::create_auth_cookie(token.clone(), &state.environment);
    let jar = jar.add(auth_cookie);

    Ok((
        jar,
        Json(AuthResponse {
            token,
            user: UserResponse {
                id,
                username,
                email,
                profile_picture_url,
            },
        }),
    ))
}

#[derive(Debug, Deserialize)]
struct RequestPasswordResetRequest {
    email: String,
}

#[derive(Debug, Serialize)]
struct RequestPasswordResetResponse {
    message: String,
}

async fn request_password_reset(
    State(state): State<ApiState>,
    Json(request): Json<RequestPasswordResetRequest>,
) -> Result<Json<RequestPasswordResetResponse>, ApiError> {
    // Validate email format
    auth::validation::validate_email(&request.email)?;

    // Find user by email (only for email auth provider)
    let user = sqlx::query_as::<_, (Uuid, String)>(
        // language=PostgreSQL
        r#"
            SELECT id, username
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(&request.email)
    .fetch_optional(&state.pool)
    .await?;

    // If user exists, create token and send email
    // Note: We don't reveal if the email exists or not for security
    if let Some((user_id, username)) = user {
        // Create reset token (expires in 1 hour)
        let token = password_reset::create_reset_token(&state.pool, user_id, 1).await?;

        // Send password reset email
        // Note: If this fails, we don't return error to prevent email enumeration
        if let Some(email_service) = &state.email_service {
            if let Err(e) =
                email_service.send_password_reset_email(&request.email, &username, &token)
            {
                eprintln!("Failed to send password reset email: {}", e);
                // Don't fail the request to prevent revealing user existence
            }
        } else {
            // Email service not configured - log the token for development
            eprintln!(
                "Email service not configured. Password reset token for {}: {}",
                request.email, token
            );
        }
    }

    // Always return success to prevent email enumeration
    Ok(Json(RequestPasswordResetResponse {
        message: "If an account exists with that email, a password reset link has been sent."
            .to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct ResetPasswordRequest {
    token: String,
    new_password: String,
}

#[derive(Debug, Serialize)]
struct ResetPasswordResponse {
    message: String,
}

async fn reset_password(
    State(state): State<ApiState>,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Json<ResetPasswordResponse>, ApiError> {
    // Validate new password
    auth::validation::validate_password(&request.new_password)?;

    // Hash the new password
    let password_hash = bcrypt::hash(&request.new_password, bcrypt::DEFAULT_COST)?;

    // Verify token and reset password in a single transaction
    // This prevents token burn without password update
    let (email, username) =
        password_reset::verify_and_reset_password(&state.pool, &request.token, &password_hash)
            .await
            .map_err(|_| {
                // Return generic error to prevent enumeration
                ApiError::Auth(
                    "Password reset failed. The token may be invalid or expired.".to_string(),
                )
            })?;

    // Send password change confirmation email
    // Note: We don't fail the request if email fails - password was already changed
    if let Some(email_service) = &state.email_service
        && let Err(e) = email_service.send_password_changed_email(&email, &username)
    {
        eprintln!("Failed to send password change confirmation email: {}", e);
        // Don't fail - password was already successfully changed
    }

    Ok(Json(ResetPasswordResponse {
        message: "Password has been reset successfully. You can now log in with your new password."
            .to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct VerifyEmailQuery {
    token: String,
}

async fn verify_email(
    State(state): State<ApiState>,
    Query(query): Query<VerifyEmailQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify the token and mark the user's email as verified
    let newly_verified = email_verification::verify_email_token(&state.pool, &query.token)
        .await
        .unwrap_or(false); // Return generic success even on error to prevent enumeration

    let message = if newly_verified {
        "Email verified successfully. You can now log in to your account."
    } else {
        "Email verification processed successfully."
    };

    Ok(Json(serde_json::json!({
        "message": message
    })))
}

#[derive(Debug, Deserialize)]
struct ResendVerificationRequest {
    email: String,
}

async fn resend_verification_email(
    State(state): State<ApiState>,
    Json(request): Json<ResendVerificationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validate email format
    auth::validation::validate_email(&request.email)?;

    // Find user by email (only for email auth provider)
    let user = sqlx::query_as::<_, (Uuid, String, bool)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email_verified
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(&request.email)
    .fetch_optional(&state.pool)
    .await?;

    // If user exists and is not verified, send verification email
    // Note: We don't reveal if the email exists or not for security
    if let Some((user_id, username, email_verified)) = user {
        // If already verified, don't send email but return success
        if !email_verified {
            // Create verification token (24 hour expiry)
            let token =
                email_verification::create_verification_token(&state.pool, user_id, 24).await?;

            // Send verification email
            if let Some(email_service) = &state.email_service {
                email_service
                    .send_verification_email(&request.email, &username, &token)
                    .map_err(|e| {
                        eprintln!("Failed to send verification email: {}", e);
                        ApiError::Email("Failed to send verification email".to_string())
                    })?;
            } else {
                // Email service not configured - log the token for development
                eprintln!(
                    "Email service not configured. Verification token for {}: {}",
                    request.email, token
                );
            }
        }
    }

    // Always return success to prevent email enumeration
    Ok(Json(serde_json::json!({
        "message": "If an unverified account exists with that email, a verification link has been sent."
    })))
}

#[derive(Debug, Serialize)]
struct DeleteUserResponse {
    message: String,
}

async fn delete_user(
    auth: AuthUser,
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Path(user_id): Path<Uuid>,
) -> Result<(PrivateCookieJar, Json<DeleteUserResponse>), ApiError> {
    // Verify the authenticated user matches the user to delete
    if auth.user_id != user_id {
        return Err(ApiError::Auth(
            "You are not authorized to delete this account".to_string(),
        ));
    }

    // Delete the user - cascade will handle all related data
    let result = sqlx::query(
        // language=PostgreSQL
        r#"
            DELETE FROM users WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(ApiError::Database)?;

    // Check if user was actually deleted
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("User not found".to_string()));
    }

    // Clear the auth cookie
    let cookie = Cookie::build(("auth_token", "")).path("/").build();
    let jar = jar.remove(cookie);

    Ok((
        jar,
        Json(DeleteUserResponse {
            message: "Account deleted successfully".to_string(),
        }),
    ))
}
