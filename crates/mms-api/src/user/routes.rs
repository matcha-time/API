use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{delete, get, patch, post},
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use serde::{Deserialize, Serialize};

use crate::{
    ApiState,
    auth::{
        self, AuthUser, cookies, jwt,
        routes::{AuthResponse, UserResponse},
    },
    error::ApiError,
    middleware::rate_limit,
    user::{email_verification, password_reset},
};

use mms_db::models::{ActivityDay, UserStats};
use mms_db::repositories::user as user_repo;

/// Check if a SQLx error is a PostgreSQL unique constraint violation (error code 23505).
fn is_unique_violation(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = e {
        db_err.code().as_deref() == Some("23505")
    } else {
        false
    }
}

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
        .route("/users/me/dashboard", get(get_user_dashboard))
        .route("/users/me/password", patch(change_password))
        .route("/users/me/username", patch(change_username))
        .route("/users/me", delete(delete_user))
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
) -> Result<Json<UserDashboard>, ApiError> {
    let user_id = auth.user_id;

    let stats = user_repo::get_user_stats(&state.pool, user_id)
        .await
        .map_err(ApiError::Database)?;

    let heatmap = user_repo::get_user_activity(&state.pool, user_id)
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
    let existing_user = user_repo::find_existence_by_email(&state.pool, &request.email).await?;

    // If user exists (verified or not), resend verification email
    // This prevents email enumeration by always returning the same response
    if let Some(existing) = existing_user {
        // If verified, don't send email but return same message
        if !existing.email_verified {
            let verification_token =
                email_verification::create_verification_token(&state.pool, existing.id, 24).await?;

            crate::user::email::send_verification_email_if_available(
                &state.email_tx,
                existing.id,
                &request.email,
                &request.username,
                &verification_token,
            );
        }

        // Return generic message regardless of verification status to prevent enumeration
        return Ok(Json(serde_json::json!({
            "message": "Registration successful. Please check your email to verify your account.",
            "email": request.email
        })));
    }

    // Start a transaction for user creation
    let mut tx = state.pool.begin().await?;

    // Hash the password (CPU-intensive, run off the async runtime)
    let password = request.password.clone();
    let cost = state.auth.bcrypt_cost;
    let password_hash = tokio::task::spawn_blocking(move || bcrypt::hash(password, cost))
        .await
        .map_err(|_| ApiError::Auth("Hashing failed".into()))?
        .map_err(ApiError::Bcrypt)?;

    // Insert user into database
    let user_id =
        user_repo::create_email_user(&mut *tx, &request.username, &request.email, &password_hash)
            .await
            .map_err(|e| {
                // Handle unique constraint violations gracefully (PostgreSQL error code 23505)
                if is_unique_violation(&e) {
                    ApiError::Conflict(
                        "Registration failed. This username or email may already be in use."
                            .to_string(),
                    )
                } else {
                    ApiError::Database(e)
                }
            })?;

    // Create user_stats entry
    user_repo::create_user_stats(&mut *tx, user_id).await?;

    // Generate verification token (24 hour expiry)
    // Use the transaction version to respect foreign key constraints
    let verification_token =
        email_verification::create_verification_token_tx(&mut tx, user_id, 24).await?;

    // Commit the transaction before sending email
    tx.commit().await?;

    // Send verification email via background worker if configured
    // Note: If this fails, user is created but email not sent
    // They can use the resend endpoint or re-register
    crate::user::email::send_verification_email_if_available(
        &state.email_tx,
        user_id,
        &request.email,
        &request.username,
        &verification_token,
    );

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
    let user = user_repo::find_credentials_by_email(&state.pool, &request.email)
        .await?
        .ok_or_else(|| ApiError::Auth("Invalid email or password".to_string()))?;

    // Verify password exists and matches
    let password_hash = user
        .password_hash
        .ok_or_else(|| ApiError::Auth("Invalid email or password".to_string()))?;

    let password = request.password.clone();
    let hash = password_hash.clone();
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash))
        .await
        .map_err(|_| ApiError::Auth("Verification failed".into()))?
        .map_err(ApiError::Bcrypt)?;
    if !valid {
        return Err(ApiError::Auth("Invalid email or password".to_string()));
    }

    // Check if email is verified
    if !user.email_verified {
        return Err(ApiError::Auth(
            "Please verify your email address before logging in. Check your inbox for the verification link.".to_string()
        ));
    }

    // Generate JWT access token
    let token = jwt::generate_jwt_token(
        user.id,
        user.email.clone(),
        &state.auth.jwt_secret,
        state.auth.jwt_expiry_hours,
    )?;

    // Generate refresh token
    let (refresh_token, refresh_token_hash) = auth::refresh_token::generate_refresh_token();
    auth::refresh_token::store_refresh_token(
        &state.pool,
        user.id,
        &refresh_token_hash,
        None,
        None,
        state.auth.refresh_token_expiry_days,
    )
    .await?;

    // Set cookies with JWT and refresh token
    let auth_cookie = cookies::create_auth_cookie(
        token.clone(),
        &state.cookie.environment,
        state.auth.jwt_expiry_hours,
        &state.cookie.cookie_domain,
    );
    let refresh_cookie = cookies::create_refresh_token_cookie(
        refresh_token.clone(),
        &state.cookie.environment,
        state.auth.refresh_token_expiry_days,
        &state.cookie.cookie_domain,
    );
    let jar = jar.add(auth_cookie).add(refresh_cookie);

    Ok((
        jar,
        Json(AuthResponse {
            token,
            refresh_token,
            user: UserResponse {
                id: user.id,
                username: user.username,
                email: user.email,
                profile_picture_url: user.profile_picture_url,
                native_language: user.native_language,
                learning_language: user.learning_language,
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
    let user = user_repo::find_id_and_name_by_email(&state.pool, &request.email).await?;

    // If user exists, create token and send email
    // Note: We don't reveal if the email exists or not for security
    if let Some(user) = user {
        // Create reset token (expires in 1 hour)
        let token = password_reset::create_reset_token(&state.pool, user.id, 1).await?;

        // Send password reset email via background worker
        // Note: If this fails, we don't return error to prevent email enumeration
        if let Some(email_tx) = &state.email_tx {
            let job = crate::user::email::EmailJob::PasswordReset {
                to_email: request.email.clone(),
                username: user.username.clone(),
                reset_token: token,
            };

            if let Err(e) = email_tx.send(job) {
                tracing::error!(error = %e, "Failed to queue password reset email");
                // Don't fail the request to prevent revealing user existence
            }
        } else {
            // Email worker not configured - log the token for development
            tracing::info!(
                email = %request.email,
                token = %token,
                "Email worker not configured - password reset token generated"
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

    // Hash the new password (CPU-intensive, run off the async runtime)
    let new_password = request.new_password.clone();
    let cost = state.auth.bcrypt_cost;
    let password_hash = tokio::task::spawn_blocking(move || bcrypt::hash(new_password, cost))
        .await
        .map_err(|_| ApiError::Auth("Hashing failed".into()))?
        .map_err(ApiError::Bcrypt)?;

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

    // Send password change confirmation email via background worker
    // Note: We don't fail the request if email fails - password was already changed
    if let Some(email_tx) = &state.email_tx {
        let job = crate::user::email::EmailJob::PasswordChanged {
            to_email: email.clone(),
            username: username.clone(),
        };

        if let Err(e) = email_tx.send(job) {
            tracing::error!(error = %e, "Failed to queue password change confirmation email");
            // Don't fail - password was already successfully changed
        }
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
    let (email, newly_verified) =
        email_verification::verify_email_token(&state.pool, &query.token).await?; // Propagate the error to return proper error codes

    let message = if newly_verified {
        "Email verified successfully. You can now log in to your account."
    } else {
        "Email verification processed successfully."
    };

    Ok(Json(serde_json::json!({
        "message": message,
        "email": email
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
    let user = user_repo::find_verification_info_by_email(&state.pool, &request.email).await?;

    // If user exists and is not verified, send verification email
    // Note: We don't reveal if the email exists or not for security
    if let Some(user) = user {
        // If already verified, don't send email but return success
        if !user.email_verified {
            // Create verification token (24 hour expiry)
            let token =
                email_verification::create_verification_token(&state.pool, user.id, 24).await?;

            // Send verification email via background worker
            // Note: If this fails, we don't return error to prevent email enumeration
            if let Some(email_tx) = &state.email_tx {
                let job = crate::user::email::EmailJob::Verification {
                    to_email: request.email.clone(),
                    username: user.username.clone(),
                    verification_token: token,
                };

                if let Err(e) = email_tx.send(job) {
                    tracing::error!(error = %e, "Failed to queue verification email");
                    // Don't fail the request - user can try resending again
                }
            } else {
                // Email worker not configured - log the token for development
                tracing::info!(
                    email = %request.email,
                    token = %token,
                    "Email worker not configured - verification token generated"
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
) -> Result<(PrivateCookieJar, Json<DeleteUserResponse>), ApiError> {
    let user_id = auth.user_id;

    // Revoke all refresh tokens for this user
    let _ = auth::refresh_token::revoke_all_user_tokens(&state.pool, user_id).await;

    // Delete the user - cascade will handle all related data
    let rows = user_repo::delete_user(&state.pool, user_id)
        .await
        .map_err(ApiError::Database)?;

    // Check if user was actually deleted
    if rows == 0 {
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
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Debug, Serialize)]
struct ChangePasswordResponse {
    message: String,
}

async fn change_password(
    auth: AuthUser,
    State(state): State<ApiState>,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Json<ChangePasswordResponse>, ApiError> {
    let user_id = auth.user_id;

    // Get current user data
    let user_info = user_repo::find_password_info(&state.pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    // Ensure this is an email auth user
    if user_info.auth_provider != "email" {
        return Err(ApiError::Validation(
            "Password changes are only available for email authentication users".to_string(),
        ));
    }

    // Verify current password
    let password_hash_value = user_info.password_hash.ok_or_else(|| {
        ApiError::Auth("Password authentication not available for this account".to_string())
    })?;

    let current_password = request.current_password.clone();
    let hash = password_hash_value.clone();
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(current_password, &hash))
        .await
        .map_err(|_| ApiError::Auth("Verification failed".into()))?
        .map_err(ApiError::Bcrypt)?;
    if !valid {
        return Err(ApiError::Auth("Current password is incorrect".to_string()));
    }

    // Ensure new password is different from current password
    if request.current_password == request.new_password {
        return Err(ApiError::Validation(
            "New password must be different from current password".to_string(),
        ));
    }

    // Validate new password
    auth::validation::validate_password(&request.new_password)?;

    // Hash the new password (CPU-intensive, run off the async runtime)
    let new_password = request.new_password.clone();
    let cost = state.auth.bcrypt_cost;
    let new_password_hash = tokio::task::spawn_blocking(move || bcrypt::hash(new_password, cost))
        .await
        .map_err(|_| ApiError::Auth("Hashing failed".into()))?
        .map_err(ApiError::Bcrypt)?;

    // Update the password
    user_repo::update_password_for_email_user(&state.pool, user_id, &new_password_hash)
        .await
        .map_err(ApiError::Database)?;

    // Send password change confirmation email via background worker
    if let Some(email_tx) = &state.email_tx {
        let job = crate::user::email::EmailJob::PasswordChanged {
            to_email: user_info.email,
            username: user_info.username,
        };

        if let Err(e) = email_tx.send(job) {
            tracing::error!(error = %e, "Failed to queue password change confirmation email");
        }
    }

    Ok(Json(ChangePasswordResponse {
        message: "Password changed successfully".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct ChangeUsernameRequest {
    username: String,
}

#[derive(Debug, Serialize)]
struct ChangeUsernameResponse {
    message: String,
    username: String,
}

async fn change_username(
    auth: AuthUser,
    State(state): State<ApiState>,
    Json(request): Json<ChangeUsernameRequest>,
) -> Result<Json<ChangeUsernameResponse>, ApiError> {
    let user_id = auth.user_id;

    // Validate username
    auth::validation::validate_username(&request.username)?;

    // Update the username
    let username = user_repo::update_username(&state.pool, user_id, &request.username)
        .await
        .map_err(|e| {
            if is_unique_violation(&e) {
                ApiError::Conflict("Username is already taken".to_string())
            } else {
                ApiError::Database(e)
            }
        })?;

    Ok(Json(ChangeUsernameResponse {
        message: "Username changed successfully".to_string(),
        username,
    }))
}
