mod common;

use axum::http::StatusCode;
use common::{TestClient, TestStateBuilder};
use mms_api::router;

#[tokio::test]
async fn test_health_check() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client.get("/health").await;

    response.assert_status(StatusCode::OK);
    // Health endpoint returns 200 OK status code with empty body

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_auth_me_without_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client.get("/auth/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert!(body["error"].is_string(), "Should have error message");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_auth_me_with_valid_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a test user
    let user_id = common::db::create_verified_user(&state.pool, "test_valid@example.com", "testuser_valid")
        .await
        .expect("Failed to create test user");

    // Generate a valid JWT token
    let token = common::jwt::create_test_token(user_id, "test_valid@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Use the simplified method
    let response = client.get_with_auth("/auth/me", &token, &state.cookie_key).await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"].as_str().unwrap(), "test_valid@example.com");
    assert_eq!(body["username"].as_str().unwrap(), "testuser_valid");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_auth_me_with_invalid_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Use invalid token
    let response = client.get_with_auth("/auth/me", "invalid_token", &state.cookie_key).await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert!(body["error"].is_string(), "Should have error message");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_auth_me_with_expired_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a test user
    let user_id = common::db::create_verified_user(&state.pool, "test_expired@example.com", "testuser_expired")
        .await
        .expect("Failed to create test user");

    // Create an expired token by using a token that was issued in the past
    use chrono::Utc;
    use jsonwebtoken::{EncodingKey, Header};
    use mms_api::auth::jwt::Claims;

    let expired_time = Utc::now() - chrono::Duration::hours(25);
    let claims = Claims {
        sub: user_id.to_string(),
        email: "test_expired@example.com".to_string(),
        iat: expired_time.timestamp() as usize,
        exp: (expired_time + chrono::Duration::hours(1)).timestamp() as usize, // Already expired
    };

    let expired_token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .expect("Failed to create expired token");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Use expired token
    let response = client.get_with_auth("/auth/me", &expired_token, &state.cookie_key).await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert!(body["error"].is_string(), "Should have error message");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_logout() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a test user
    let user_id = common::db::create_verified_user(&state.pool, "test_logout@example.com", "testuser_logout")
        .await
        .expect("Failed to create test user");

    // Generate a valid JWT token
    let token = common::jwt::create_test_token(user_id, "test_logout@example.com", &state.jwt_secret);

    // Create a refresh token in the database
    let refresh_token = uuid::Uuid::new_v4().to_string();
    let token_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(refresh_token.as_bytes());
        hex::encode(hasher.finalize())
    };

    sqlx::query!(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, NOW() + INTERVAL '30 days')
        "#,
        user_id,
        token_hash
    )
    .execute(&state.pool)
    .await
    .expect("Failed to create refresh token");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Use the get_with_auth_and_refresh method for logout (needs both cookies)
    let response = client.get_with_auth_and_refresh("/auth/logout", &token, &refresh_token, &state.cookie_key).await;

    response.assert_status(StatusCode::OK);

    // Verify refresh token was deleted from database
    let token_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1
        "#
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to query refresh tokens");

    assert_eq!(token_count, 0, "All refresh tokens should be deleted");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}

#[tokio::test]
async fn test_google_auth_redirect() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client.get("/auth/google").await;

    // Should redirect to Google OAuth
    assert!(
        response.status == StatusCode::SEE_OTHER || response.status == StatusCode::FOUND,
        "Expected redirect status, got {}",
        response.status
    );

    // Should redirect to Google
    let location = response
        .headers
        .get("location")
        .expect("Location header should be present")
        .to_str()
        .expect("Location should be valid string");

    assert!(
        location.contains("accounts.google.com"),
        "Should redirect to Google"
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}
