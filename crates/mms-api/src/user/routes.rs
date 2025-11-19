use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use axum_extra::extract::PrivateCookieJar;
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
        .route("/users/verify-email", get(verify_email))
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
    .fetch_one(&state.pool)
    .await?;

    // Create user_stats entry
    sqlx::query(
        // language=PostgreSQL
        r#"
            INSERT INTO user_stats (user_id)
            VALUES ($1)
        "#,
    )
    .bind(user_id)
    .execute(&state.pool)
    .await?;

    // Generate verification token (24 hour expiry)
    let verification_token =
        email_verification::create_verification_token(&state.pool, user_id, 24).await?;

    // Send verification email if email service is configured
    if let Some(email_service) = &state.email_service {
        email_service.send_verification_email(
            &request.email,
            &request.username,
            &verification_token,
        )?;
    } else {
        // If email service is not configured, log the verification URL to the console
        eprintln!(
            "Email service not configured. Verification token for user {}: {}",
            user_id,
            verification_token
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
        if let Some(email_service) = &state.email_service {
            email_service
                .send_password_reset_email(&request.email, &username, &token)
                .map_err(|e| {
                    eprintln!("Failed to send password reset email: {}", e);
                    ApiError::Email("Failed to send password reset email".to_string())
                })?;
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

    // Verify token and get user_id (this marks the token as used)
    let user_id = password_reset::verify_reset_token(&state.pool, &request.token).await?;

    // Hash the new password
    let password_hash = bcrypt::hash(&request.new_password, bcrypt::DEFAULT_COST)?;

    // Update user's password
    sqlx::query(
        // language=PostgreSQL
        r#"
            UPDATE users
            SET password_hash = $1
            WHERE id = $2 AND auth_provider = 'email'
        "#,
    )
    .bind(&password_hash)
    .bind(user_id)
    .execute(&state.pool)
    .await?;

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
    let user_id = email_verification::verify_email_token(&state.pool, &query.token).await?;

    Ok(Json(serde_json::json!({
        "message": "Email verified successfully. You can now log in to your account.",
        "user_id": user_id
    })))
}
