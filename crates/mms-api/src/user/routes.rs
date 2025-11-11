use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use axum_extra::extract::PrivateCookieJar;
use axum_extra::extract::cookie::Cookie;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::{ApiState, auth::routes::Claims, error::ApiError};

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
    State(state): State<ApiState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserDashboard>, StatusCode> {
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

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    user: UserResponse,
}

#[derive(Serialize)]
struct UserResponse {
    id: Uuid,
    username: String,
    email: String,
}

async fn create_user(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Json(request): Json<CreateUserRequest>,
) -> Result<(PrivateCookieJar, Json<AuthResponse>), ApiError> {
    // Hash the password
    let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;

    // Insert user into database
    let user_id = sqlx::query_scalar::<_, Uuid>(
        // language=PostgreSQL
        r#"
            INSERT INTO users (username, email, password_hash)
            VALUES ($1, $2, $3)
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
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: request.email.clone(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )?;

    // Set auth cookie with JWT
    let auth_cookie = Cookie::build(("auth_token", token.clone()))
        .path("/")
        .max_age(time::Duration::hours(24))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(false) // Set to true in production with HTTPS
        .build();

    let jar = jar.add(auth_cookie);

    Ok((
        jar,
        Json(AuthResponse {
            token,
            user: UserResponse {
                id: user_id,
                username: request.username,
                email: request.email,
            },
        }),
    ))
}

async fn login_user(
    State(state): State<ApiState>,
    jar: PrivateCookieJar,
    Json(request): Json<LoginRequest>,
) -> Result<(PrivateCookieJar, Json<AuthResponse>), ApiError> {
    // Fetch user from database
    let user = sqlx::query_as::<_, (Uuid, String, String, String)>(
        // language=PostgreSQL
        r#"
            SELECT id, username, email, password_hash
            FROM users
            WHERE email = $1
        "#,
    )
    .bind(&request.email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::Auth("Invalid email or password".to_string()))?;

    let (user_id, username, email, password_hash) = user;

    // Verify password
    if !bcrypt::verify(&request.password, &password_hash)? {
        return Err(ApiError::Auth("Invalid email or password".to_string()));
    }

    // Generate JWT token
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.clone(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )?;

    // Set auth cookie with JWT
    let auth_cookie = Cookie::build(("auth_token", token.clone()))
        .path("/")
        .max_age(time::Duration::hours(24))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(false) // Set to true in production with HTTPS
        .build();

    let jar = jar.add(auth_cookie);

    Ok((
        jar,
        Json(AuthResponse {
            token,
            user: UserResponse {
                id: user_id,
                username,
                email,
            },
        }),
    ))
}
