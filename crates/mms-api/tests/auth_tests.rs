use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
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
    // No cleanup needed - no data created
}

#[tokio::test]
async fn test_auth_me_without_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client.get("/v1/auth/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert!(body["error"].is_string(), "Should have error message");

    // No cleanup needed - no data created
}

#[tokio::test]
async fn test_auth_me_with_valid_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a test user
    let user_id =
        common::db::create_verified_user(&state.pool, "test_valid@example.com", "testuser_valid")
            .await
            .expect("Failed to create test user");

    // Generate a valid JWT token
    let token =
        common::jwt::create_test_token(user_id, "test_valid@example.com", &state.jwt_secret);

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Use the simplified method
    let response = client
        .get_with_auth("/v1/auth/me", &token, &state.cookie_key)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"].as_str().unwrap(), "test_valid@example.com");
    assert_eq!(body["username"].as_str().unwrap(), "testuser_valid");

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "test_valid@example.com")
        .await
        .expect("Failed to cleanup test user");
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
    let response = client
        .get_with_auth("/v1/auth/me", "invalid_token", &state.cookie_key)
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert!(body["error"].is_string(), "Should have error message");

    // No cleanup needed - no data created
}

#[tokio::test]
async fn test_auth_me_with_expired_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a test user
    let user_id = common::db::create_verified_user(
        &state.pool,
        "test_expired@example.com",
        "testuser_expired",
    )
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
    let response = client
        .get_with_auth("/v1/auth/me", &expired_token, &state.cookie_key)
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = response.json();
    assert!(body["error"].is_string(), "Should have error message");

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "test_expired@example.com")
        .await
        .expect("Failed to cleanup test user");
}

#[tokio::test]
async fn test_logout() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create a test user
    let user_id =
        common::db::create_verified_user(&state.pool, "test_logout@example.com", "testuser_logout")
            .await
            .expect("Failed to create test user");

    // Generate a valid JWT token
    let token =
        common::jwt::create_test_token(user_id, "test_logout@example.com", &state.jwt_secret);

    // Create a refresh token in the database
    let refresh_token = uuid::Uuid::new_v4().to_string();
    let token_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(refresh_token.as_bytes());
        hex::encode(hasher.finalize())
    };

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, NOW() + INTERVAL '30 days')
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .execute(&state.pool)
    .await
    .expect("Failed to create refresh token");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Use the get_with_auth_and_refresh method for logout (needs both cookies)
    let response = client
        .get_with_auth_and_refresh("/v1/auth/logout", &token, &refresh_token, &state.cookie_key)
        .await;

    response.assert_status(StatusCode::OK);

    // Verify refresh token was deleted from database
    let token_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to query refresh tokens");

    assert_eq!(token_count, 0, "All refresh tokens should be deleted");

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "test_logout@example.com")
        .await
        .expect("Failed to cleanup test user");
}

#[tokio::test]
async fn test_google_auth_redirect() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client.get("/v1/auth/google").await;

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

    // Should set OIDC flow cookie
    let oidc_cookie = response.get_cookie("oidc_flow");
    assert!(oidc_cookie.is_some(), "Should set OIDC flow cookie");

    // No cleanup needed - no data created
}

#[tokio::test]
async fn test_google_callback_csrf_validation() {
    // This test verifies that the callback validates CSRF tokens correctly
    // We test the path up to the OAuth token exchange (which would require mocking Google's servers)

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Step 1: Initiate OAuth flow to get OIDC cookie
    let init_response = client.get("/v1/auth/google").await;

    // Verify we got an OIDC cookie
    assert!(
        init_response.get_cookie("oidc_flow").is_some(),
        "Should set OIDC flow cookie"
    );

    // Step 2: Try callback without the cookie - should fail
    let response_no_cookie = client.get("/v1/auth/callback?code=test&state=test").await;

    assert!(
        response_no_cookie.status == StatusCode::BAD_REQUEST
            || response_no_cookie.status == StatusCode::INTERNAL_SERVER_ERROR,
        "Should reject callback without OIDC cookie"
    );

    // Note: Testing the full callback flow with proper CSRF validation would require
    // either mocking the Google OAuth server or dependency injection for the OIDC client.
    // The service layer tests below cover the user creation logic thoroughly.
}

#[tokio::test]
async fn test_google_callback_without_oidc_cookie() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try to call callback without initiating OAuth flow (no OIDC cookie)
    let response = client
        .get("/v1/auth/callback?code=mock_code&state=mock_state")
        .await;

    // Should return error due to missing OIDC cookie
    assert!(
        response.status == StatusCode::BAD_REQUEST
            || response.status == StatusCode::UNAUTHORIZED
            || response.status == StatusCode::INTERNAL_SERVER_ERROR,
        "Should reject request without OIDC cookie. Status: {}",
        response.status
    );
}

#[tokio::test]
async fn test_find_or_create_google_user_new_user() {
    use mms_api::auth::google::service::find_or_create_google_user;

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let test_email = "google_new@example.com";
    let test_google_id = "google_new_123";

    // Create user via Google auth
    let user = find_or_create_google_user(
        &state.pool,
        test_google_id,
        test_email,
        Some("Google User"),
        Some("https://example.com/pic.jpg"),
    )
    .await
    .expect("Should create user");

    assert_eq!(user.email, test_email);
    assert_eq!(user.username, "Google User");
    assert_eq!(
        user.profile_picture_url,
        Some("https://example.com/pic.jpg".to_string())
    );

    // Verify user was created in database
    let db_user = common::db::get_user_by_email(&state.pool, test_email)
        .await
        .expect("Should query user")
        .expect("User should exist");

    assert_eq!(db_user, user.id);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, test_email)
        .await
        .expect("Failed to cleanup user");
}

#[tokio::test]
async fn test_find_or_create_google_user_existing_google_user() {
    use mms_api::auth::google::service::find_or_create_google_user;

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let test_email = "google_existing@example.com";
    let test_google_id = "google_existing_123";

    // Create user first time
    let user1 = find_or_create_google_user(
        &state.pool,
        test_google_id,
        test_email,
        Some("Original Name"),
        Some("https://example.com/pic1.jpg"),
    )
    .await
    .expect("Should create user");

    // Try to create same user again (should find existing)
    let user2 = find_or_create_google_user(
        &state.pool,
        test_google_id,
        test_email,
        Some("Updated Name"),
        Some("https://example.com/pic2.jpg"),
    )
    .await
    .expect("Should find existing user");

    // Should be the same user
    assert_eq!(user1.id, user2.id);
    assert_eq!(user1.username, user2.username); // Username shouldn't change
    assert_eq!(user2.email, test_email);

    // Profile picture should be updated
    assert_eq!(
        user2.profile_picture_url,
        Some("https://example.com/pic2.jpg".to_string())
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, test_email)
        .await
        .expect("Failed to cleanup user");
}

#[tokio::test]
async fn test_find_or_create_google_user_links_existing_email_user() {
    use mms_api::auth::google::service::find_or_create_google_user;

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let test_email = "email_then_google@example.com";
    let test_google_id = "google_link_123";

    // Create user with email/password first
    let user_id = common::db::create_verified_user(&state.pool, test_email, "emailuser")
        .await
        .expect("Should create email user");

    // Now try to login with Google using same email
    let user = find_or_create_google_user(
        &state.pool,
        test_google_id,
        test_email,
        Some("Google Name"),
        Some("https://example.com/pic.jpg"),
    )
    .await
    .expect("Should link Google account");

    // Should be the same user
    assert_eq!(user.id, user_id);
    assert_eq!(user.email, test_email);

    // Verify google_id was added
    let google_id_result: Option<String> = sqlx::query_scalar(
        r#"
        SELECT google_id FROM users WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .expect("Should query google_id");

    assert_eq!(google_id_result, Some(test_google_id.to_string()));

    // Cleanup
    common::db::delete_user_by_email(&state.pool, test_email)
        .await
        .expect("Failed to cleanup user");
}

#[tokio::test]
async fn test_find_or_create_google_user_handles_username_conflict() {
    use mms_api::auth::google::service::find_or_create_google_user;

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let test_email1 = "user1@example.com";
    let test_email2 = "user2@example.com";
    let username = "SameName";

    // Create first user with this username
    let user1 =
        find_or_create_google_user(&state.pool, "google_1", test_email1, Some(username), None)
            .await
            .expect("Should create first user");

    assert_eq!(user1.username, username);

    // Create second user with same name (should get numbered suffix)
    let user2 =
        find_or_create_google_user(&state.pool, "google_2", test_email2, Some(username), None)
            .await
            .expect("Should create second user");

    // Second user should have different username
    assert_ne!(user1.username, user2.username);
    assert!(
        user2.username.starts_with(username),
        "Username should start with original name"
    );
    assert!(
        user2.username.len() > username.len(),
        "Username should have suffix"
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, test_email1)
        .await
        .expect("Failed to cleanup user1");
    common::db::delete_user_by_email(&state.pool, test_email2)
        .await
        .expect("Failed to cleanup user2");
}
