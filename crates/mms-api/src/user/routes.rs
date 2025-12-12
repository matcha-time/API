use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
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
    middleware::rate_limit,
    user::{email_verification, password_reset},
};

use mms_db::models::{ActivityDay, UserStats};

/// Create the user routes
pub fn routes() -> Router<ApiState> {
    use crate::make_rate_limit_layer;

    // Sensitive routes with very strict rate limiting and timing-safe middleware
    let sensitive_routes = Router::new()
        .route(
            "/users/request-password-reset",
            post(request_password_reset),
        )
        .route(
            "/users/resend-verification",
            post(resend_verification_email),
        )
        .layer(make_rate_limit_layer!(
            rate_limit::SENSITIVE_RATE_PER_SECOND,
            rate_limit::SENSITIVE_BURST_SIZE
        ))
        .route_layer(axum::middleware::from_fn(
            rate_limit::timing_safe_middleware,
        ));

    // Auth routes with strict rate limiting and timing-safe middleware
    let auth_routes = Router::new()
        .route("/users/register", post(create_user))
        .route("/users/login", post(login_user))
        .route("/users/reset-password", post(reset_password))
        .layer(make_rate_limit_layer!(
            rate_limit::AUTH_RATE_PER_SECOND,
            rate_limit::AUTH_BURST_SIZE
        ))
        .route_layer(axum::middleware::from_fn(
            rate_limit::timing_safe_middleware,
        ));

    // General authenticated routes with moderate rate limiting
    let general_routes = Router::new()
        .route("/users/{user_id}/dashboard", get(get_user_dashboard))
        .route("/users/{user_id}", patch(update_user_profile))
        .route("/users/{user_id}", delete(delete_user))
        .route("/users/verify-email", get(verify_email))
        .layer(make_rate_limit_layer!(
            rate_limit::GENERAL_RATE_PER_SECOND,
            rate_limit::GENERAL_BURST_SIZE
        ));

    // Merge all route groups
    Router::new()
        .merge(sensitive_routes)
        .merge(auth_routes)
        .merge(general_routes)
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

    // If user exists (verified or not), resend verification email
    // This prevents email enumeration by always returning the same response
    if let Some((user_id, email_verified)) = existing_user {
        // If verified, don't send email but return same message
        if !email_verified {
            let verification_token =
                email_verification::create_verification_token(&state.pool, user_id, 24).await?;

            if let Some(email_service) = &state.email_service {
                let _ = email_service.send_verification_email(
                    &request.email,
                    &request.username,
                    &verification_token,
                );
            } else {
                tracing::info!(
                    user_id = %user_id,
                    token = %verification_token,
                    "Email service not configured - verification token generated"
                );
            }
        }

        // Return generic message regardless of verification status to prevent enumeration
        return Ok(Json(serde_json::json!({
            "message": "Registration successful. Please check your email to verify your account.",
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
                ApiError::Conflict("Username is already taken.".to_string())
            } else {
                ApiError::Conflict("An account with this email already exists.".to_string())
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
    // Use the transaction version to respect foreign key constraints
    let verification_token =
        email_verification::create_verification_token_tx(&mut tx, user_id, 24).await?;

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
            tracing::error!(error = %e, "Failed to send verification email");
            // Don't fail the request, user can resend later
        }
    } else {
        // If email service is not configured, log the verification URL to the console
        tracing::info!(
            user_id = %user_id,
            token = %verification_token,
            "Email service not configured - verification token generated"
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

    // Generate JWT access token
    let token =
        jwt::generate_jwt_token(id, email.clone(), &state.jwt_secret, state.jwt_expiry_hours)?;

    // Generate refresh token
    let (refresh_token, refresh_token_hash) = auth::refresh_token::generate_refresh_token();
    auth::refresh_token::store_refresh_token(
        &state.pool,
        id,
        &refresh_token_hash,
        None,
        None,
        state.refresh_token_expiry_days,
    )
    .await?;

    // Set cookies with JWT and refresh token
    let auth_cookie =
        jwt::create_auth_cookie(token.clone(), &state.environment, state.jwt_expiry_hours);
    let refresh_cookie = create_refresh_token_cookie(
        refresh_token.clone(),
        &state.environment,
        state.refresh_token_expiry_days,
    );
    let jar = jar.add(auth_cookie).add(refresh_cookie);

    Ok((
        jar,
        Json(AuthResponse {
            token,
            refresh_token,
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
                tracing::error!(error = %e, "Failed to send password reset email");
                // Don't fail the request to prevent revealing user existence
            }
        } else {
            // Email service not configured - log the token for development
            tracing::info!(
                email = %request.email,
                token = %token,
                "Email service not configured - password reset token generated"
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
        tracing::error!(error = %e, "Failed to send password change confirmation email");
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
    let newly_verified = email_verification::verify_email_token(&state.pool, &query.token).await?; // Propagate the error to return proper error codes

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
                        tracing::error!(error = %e, "Failed to send verification email");
                        ApiError::Email("Failed to send verification email".to_string())
                    })?;
            } else {
                // Email service not configured - log the token for development
                tracing::info!(
                    email = %request.email,
                    token = %token,
                    "Email service not configured - verification token generated"
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

    // Revoke all refresh tokens for this user
    let _ = auth::refresh_token::revoke_all_user_tokens(&state.pool, user_id).await;

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

    // Clear both auth and refresh token cookies
    let auth_cookie = Cookie::build(("auth_token", "")).path("/").build();
    let refresh_cookie = Cookie::build(("refresh_token", "")).path("/").build();
    let jar = jar.remove(auth_cookie).remove(refresh_cookie);

    Ok((
        jar,
        Json(DeleteUserResponse {
            message: "Account deleted successfully".to_string(),
        }),
    ))
}

#[derive(Debug, Deserialize)]
struct UpdateUserProfileRequest {
    username: Option<String>,
    email: Option<String>,
    current_password: Option<String>,
    new_password: Option<String>,
    profile_picture_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateUserProfileResponse {
    message: String,
    user: UserResponse,
}

// TODO: refactor this giant
async fn update_user_profile(
    auth: AuthUser,
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserProfileRequest>,
) -> Result<(PrivateCookieJar, Json<UpdateUserProfileResponse>), ApiError> {
    // Verify the authenticated user matches the user to update
    if auth.user_id != user_id {
        return Err(ApiError::Auth(
            "You are not authorized to update this profile".to_string(),
        ));
    }

    // Get current user data
    let user = sqlx::query_as::<_, (String, String, Option<String>, String)>(
        // language=PostgreSQL
        r#"
            SELECT username, email, password_hash, auth_provider::text
            FROM users
            WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let (current_username, current_email, password_hash, auth_provider) = user;

    // Validate password change request
    if let Some(new_password) = &request.new_password {
        // Ensure this is an email auth user
        if auth_provider != "email" {
            return Err(ApiError::Validation(
                "Password changes are only available for email authentication users".to_string(),
            ));
        }

        // Require current password for password changes
        let current_password = request.current_password.as_ref().ok_or_else(|| {
            ApiError::Validation("Current password is required to set a new password".to_string())
        })?;

        // Verify current password
        let password_hash = password_hash.ok_or_else(|| {
            ApiError::Auth("Password authentication not available for this account".to_string())
        })?;

        if !bcrypt::verify(current_password, &password_hash)? {
            return Err(ApiError::Auth("Current password is incorrect".to_string()));
        }

        // Validate new password
        auth::validation::validate_password(new_password)?;
    }

    // Validate optional fields
    if let Some(username) = &request.username {
        auth::validation::validate_username(username)?;
    }

    if let Some(email) = &request.email {
        auth::validation::validate_email(email)?;
    }

    if let Some(profile_pic) = &request.profile_picture_url {
        auth::validation::validate_profile_picture_url(profile_pic)?;
    }

    // Check if there are any updates to make
    let has_updates = request.username.is_some()
        || request.email.is_some()
        || request.new_password.is_some()
        || request.profile_picture_url.is_some();

    if !has_updates {
        let user_response = UserResponse {
            id: user_id,
            username: current_username,
            email: current_email,
            profile_picture_url: None,
        };

        return Ok((
            jar,
            Json(UpdateUserProfileResponse {
                message: "No changes were made".to_string(),
                user: user_response,
            }),
        ));
    }

    // Build safe dynamic update query using QueryBuilder
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE users SET ");
    let mut separated = query_builder.separated(", ");

    if let Some(new_username) = &request.username {
        separated.push("username = ");
        separated.push_bind_unseparated(new_username);
    }

    if let Some(new_email) = &request.email {
        separated.push("email = ");
        separated.push_bind_unseparated(new_email);
        // When email changes, mark as unverified
        separated.push_unseparated("email_verified = FALSE");
    }

    if let Some(new_pwd) = &request.new_password {
        let new_password_hash = bcrypt::hash(new_pwd, bcrypt::DEFAULT_COST)?;
        separated.push("password_hash = ");
        separated.push_bind_unseparated(new_password_hash);
    }

    if let Some(profile_pic) = &request.profile_picture_url {
        separated.push("profile_picture_url = ");
        separated.push_bind_unseparated(profile_pic);
    }

    query_builder.push(" WHERE id = ");
    query_builder.push_bind(user_id);
    query_builder.push(" RETURNING id, username, email, profile_picture_url");

    let (id, username, email, profile_picture_url) = query_builder
        .build_query_as::<(Uuid, String, String, Option<String>)>()
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                if e.to_string().contains("username") {
                    ApiError::Conflict("Username is already taken".to_string())
                } else {
                    ApiError::Conflict("Email is already in use".to_string())
                }
            } else {
                ApiError::Database(e)
            }
        })?;

    // If email changed, send verification email
    if request.email.is_some() && request.email.as_ref() != Some(&current_email) {
        let verification_token =
            email_verification::create_verification_token(&state.pool, user_id, 24).await?;

        if let Some(email_service) = &state.email_service {
            if let Err(e) =
                email_service.send_verification_email(&email, &username, &verification_token)
            {
                tracing::error!(error = %e, "Failed to send verification email");
            }
        } else {
            tracing::info!(
                user_id = %user_id,
                token = %verification_token,
                "Email service not configured - verification token generated"
            );
        }
    }

    let user_response = UserResponse {
        id,
        username: username.clone(),
        email: email.clone(),
        profile_picture_url,
    };

    // Generate new JWT if email changed
    let jar = if request.email.is_some() && request.email.as_ref() != Some(&current_email) {
        let token = jwt::generate_jwt_token(id, email, &state.jwt_secret, state.jwt_expiry_hours)?;
        let auth_cookie =
            jwt::create_auth_cookie(token, &state.environment, state.jwt_expiry_hours);
        jar.add(auth_cookie)
    } else {
        jar
    };

    Ok((
        jar,
        Json(UpdateUserProfileResponse {
            message: "Profile updated successfully".to_string(),
            user: user_response,
        }),
    ))
}

/// Create a refresh token cookie
///
/// Cookies are secure (HTTPS-only) by default in production.
/// In development mode, cookies can be used over HTTP.
fn create_refresh_token_cookie(
    token: String,
    environment: &crate::config::Environment,
    expiry_days: i64,
) -> Cookie<'static> {
    let is_development = environment.is_development();
    let same_site = if is_development {
        axum_extra::extract::cookie::SameSite::Lax
    } else {
        axum_extra::extract::cookie::SameSite::Strict
    };

    Cookie::build(("refresh_token", token))
        .path("/")
        .max_age(time::Duration::days(expiry_days))
        .http_only(true)
        .same_site(same_site)
        .secure(!is_development)
        .build()
}
