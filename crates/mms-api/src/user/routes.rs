use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use axum_extra::extract::PrivateCookieJar;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::{
    ApiState,
    auth::{self, AuthUser, jwt},
    error::ApiError,
};

use mms_db::models::{ActivityDay, UserStats};

/// Create the user routes
pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/users/register", post(create_user))
        .route("/users/login", post(login_user))
        .route("/users/{user_id}/dashboard", get(get_user_dashboard))
}

#[derive(Serialize)]
struct UserDashboard {
    stats: UserStats,
    heatmap: Vec<ActivityDay>,
}

// TODO: make this two database calls concurrent or on two different routes
async fn get_user_dashboard(
    AuthUser {
        user_id: auth_user_id,
        ..
    }: AuthUser,
    State(state): State<ApiState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserDashboard>, StatusCode> {
    // Verify the authenticated user matches the requested user
    if auth_user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
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
    .map_err(|_| StatusCode::NOT_FOUND)?;

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
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    jar: PrivateCookieJar,
    Json(request): Json<CreateUserRequest>,
) -> Result<(PrivateCookieJar, Json<auth::routes::AuthResponse>), ApiError> {
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

    // Generate JWT token
    let token = jwt::generate_jwt_token(user_id, request.email.clone(), &state.jwt_secret)?;

    // Set auth cookie with JWT
    let auth_cookie = jwt::create_auth_cookie(token.clone());
    let jar = jar.add(auth_cookie);

    Ok((
        jar,
        Json(auth::routes::AuthResponse {
            token,
            user: auth::routes::UserResponse {
                id: user_id,
                username: request.username,
                email: request.email,
                profile_picture_url: None,
            },
        }),
    ))
}

async fn login_user(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Json(request): Json<LoginRequest>,
) -> Result<(PrivateCookieJar, Json<auth::routes::AuthResponse>), ApiError> {
    // Fetch user from database
    let user = sqlx::query_as::<_, (Uuid, String, String, Option<String>, Option<String>)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, password_hash, profile_picture_url
            FROM users
            WHERE email = $1 AND auth_provider = 'email'
        "#,
    )
    .bind(&request.email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::Auth("Invalid email or password".to_string()))?;

    let (user_id, username, email, password_hash, profile_picture_url) = user;

    // Verify password exists and matches
    let password_hash =
        password_hash.ok_or_else(|| ApiError::Auth("Invalid email or password".to_string()))?;

    if !bcrypt::verify(&request.password, &password_hash)? {
        return Err(ApiError::Auth("Invalid email or password".to_string()));
    }

    // Generate JWT token
    let token = jwt::generate_jwt_token(user_id, email.clone(), &state.jwt_secret)?;

    // Set auth cookie with JWT
    let auth_cookie = jwt::create_auth_cookie(token.clone());
    let jar = jar.add(auth_cookie);

    Ok((
        jar,
        Json(auth::routes::AuthResponse {
            token,
            user: auth::routes::UserResponse {
                id: user_id,
                username,
                email,
                profile_picture_url,
            },
        }),
    ))
}
