mod common;

use axum::http::StatusCode;
use common::{TestClient, TestStateBuilder};
use mms_api::router;
use serde_json::json;

#[tokio::test]
async fn test_user_registration_success() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "username": "newuser",
        "email": "newuser@example.com",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/users/register", &body).await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(
        json["message"]
            .as_str()
            .unwrap()
            .contains("Registration successful")
    );
    assert_eq!(json["email"].as_str().unwrap(), "newuser@example.com");

    // Verify user was created in database
    let user_exists = common::db::get_user_by_email(&state.pool, "newuser@example.com")
        .await
        .expect("Failed to query user");
    assert!(user_exists.is_some(), "User should exist in database");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_user_registration_duplicate_email() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a verified user first
    common::db::create_verified_user(&state.pool, "existing@example.com", "existinguser")
        .await
        .expect("Failed to create test user");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "username": "newuser",
        "email": "existing@example.com",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/users/register", &body).await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let json: serde_json::Value = response.json();
    assert!(json["error"].as_str().unwrap().contains("already exists"));

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_user_registration_invalid_email() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "username": "newuser",
        "email": "invalid-email",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/users/register", &body).await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let json: serde_json::Value = response.json();
    assert!(json["error"].as_str().unwrap().contains("email"));

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_user_registration_weak_password() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "username": "newuser",
        "email": "newuser@example.com",
        "password": "weak"
    });

    let response = client.post_json("/users/register", &body).await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let json: serde_json::Value = response.json();
    assert!(json["error"].as_str().unwrap().contains("password"));

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_user_login_success() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a verified user with known password
    let password_hash =
        bcrypt::hash("password123", bcrypt::DEFAULT_COST).expect("Failed to hash password");

    common::db::create_test_user(
        &state.pool,
        "testuser@example.com",
        "testuser",
        &password_hash,
    )
    .await
    .expect("Failed to create test user");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "email": "testuser@example.com",
        "password": "password123"
    });

    let response = client.post_json("/users/login", &body).await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json["token"].is_string(), "Should return JWT token");
    assert!(
        json["refresh_token"].is_string(),
        "Should return refresh token"
    );
    assert_eq!(
        json["user"]["email"].as_str().unwrap(),
        "testuser@example.com"
    );
    assert_eq!(json["user"]["username"].as_str().unwrap(), "testuser");

    // Verify auth cookie was set
    let auth_cookie = response.get_cookie("auth_token");
    assert!(auth_cookie.is_some(), "Auth cookie should be set");

    // Verify refresh cookie was set
    let refresh_cookie = response.get_cookie("refresh_token");
    assert!(
        refresh_cookie.is_some(),
        "Refresh token cookie should be set"
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_user_login_invalid_credentials() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a verified user
    common::db::create_verified_user(&state.pool, "testuser@example.com", "testuser")
        .await
        .expect("Failed to create test user");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "email": "testuser@example.com",
        "password": "wrongpassword"
    });

    let response = client.post_json("/users/login", &body).await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let json: serde_json::Value = response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("Invalid email or password")
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_user_login_nonexistent_user() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "email": "nonexistent@example.com",
        "password": "password123"
    });

    let response = client.post_json("/users/login", &body).await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let json: serde_json::Value = response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("Invalid email or password")
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_get_user_dashboard() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a verified user
    let user_id = common::db::create_verified_user(&state.pool, "testuser@example.com", "testuser")
        .await
        .expect("Failed to create test user");

    // Generate auth token
    let token = common::jwt::create_test_token(user_id, "testuser@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create request with auth cookie
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/users/{}/dashboard", user_id))
        .header("cookie", format!("auth_token={}", token))
        .body(axum::body::Body::empty())
        .expect("Failed to build request");

    let response = client.with_auth_cookie(request, &token, &state.cookie_key).await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json["stats"].is_object(), "Should have stats");
    assert!(json["heatmap"].is_array(), "Should have heatmap");

    // Verify stats structure
    let stats = &json["stats"];
    assert!(stats["current_streak_days"].is_number());
    assert!(stats["longest_streak_days"].is_number());
    assert!(stats["total_reviews"].is_number());
    assert!(stats["total_cards_learned"].is_number());

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_get_dashboard_unauthorized() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create two users
    let user1_id = common::db::create_verified_user(&state.pool, "user1@example.com", "user1")
        .await
        .expect("Failed to create user1");

    let user2_id = common::db::create_verified_user(&state.pool, "user2@example.com", "user2")
        .await
        .expect("Failed to create user2");

    // Generate auth token for user1
    let token = common::jwt::create_test_token(user1_id, "user1@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try to access user2's dashboard with user1's token
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/users/{}/dashboard", user2_id))
        .header("cookie", format!("auth_token={}", token))
        .body(axum::body::Body::empty())
        .expect("Failed to build request");

    let response = client.with_auth_cookie(request, &token, &state.cookie_key).await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let json: serde_json::Value = response.json();
    assert!(json["error"].as_str().unwrap().contains("not authorized"));

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_update_user_profile() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a verified user
    let user_id = common::db::create_verified_user(&state.pool, "testuser@example.com", "testuser")
        .await
        .expect("Failed to create test user");

    // Generate auth token
    let token = common::jwt::create_test_token(user_id, "testuser@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Update username
    let body = json!({
        "username": "updateduser"
    });

    let request = axum::http::Request::builder()
        .method("PATCH")
        .uri(format!("/users/{}", user_id))
        .header("content-type", "application/json")
        .header("cookie", format!("auth_token={}", token))
        .body(axum::body::Body::from(
            serde_json::to_string(&body).unwrap(),
        ))
        .expect("Failed to build request");

    let response = client.request(request).await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert_eq!(json["username"].as_str().unwrap(), "updateduser");

    // Verify the update in database
    let updated_username =
        sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .expect("Failed to fetch username");

    assert_eq!(updated_username, "updateduser");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_delete_user() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a verified user
    let user_id = common::db::create_verified_user(&state.pool, "testuser@example.com", "testuser")
        .await
        .expect("Failed to create test user");

    // Generate auth token
    let token = common::jwt::create_test_token(user_id, "testuser@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Delete user
    let request = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/users/{}", user_id))
        .header("cookie", format!("auth_token={}", token))
        .body(axum::body::Body::empty())
        .expect("Failed to build request");

    let response = client.request(request).await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json["message"].as_str().unwrap().contains("deleted"));

    // Verify user was deleted from database
    let user_exists = common::db::get_user_by_email(&state.pool, "testuser@example.com")
        .await
        .expect("Failed to query user");

    assert!(
        user_exists.is_none(),
        "User should be deleted from database"
    );

    // Cleanup (already deleted, but cleanup other tables)
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_user_registration_creates_stats() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "username": "statsuser",
        "email": "statsuser@example.com",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/users/register", &body).await;

    response.assert_status(StatusCode::OK);

    // Get the user_id
    let user_id = common::db::get_user_by_email(&state.pool, "statsuser@example.com")
        .await
        .expect("Failed to query user")
        .expect("User should exist");

    // Verify user_stats entry was created
    let stats_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM user_stats WHERE user_id = $1)")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .expect("Failed to check stats");

    assert!(stats_exists, "User stats should be created automatically");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}
