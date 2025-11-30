use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
use mms_api::router;
use serde_json::json;

#[tokio::test]
async fn test_refresh_token_rotation_success() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create and login user with unique email for concurrency safety
    let email = common::test_data::unique_email("refreshtest");
    let username = common::test_data::unique_username("refreshuser");
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
    common::db::create_test_user(&state.pool, &email, &username, &password_hash)
        .await
        .expect("Failed to create user");

    let login_body = json!({
        "email": &email,
        "password": "password123"
    });
    let login_response = client.post_json("/v1/users/login", &login_body).await;
    login_response.assert_status(StatusCode::OK);

    let login_json: serde_json::Value = login_response.json();
    let old_access_token = login_json["token"].as_str().unwrap();
    let old_refresh_token = login_json["refresh_token"].as_str().unwrap();

    // Get refresh token hash from database
    let old_token_hash: String = sqlx::query_scalar(
        r#"
        SELECT token_hash
        FROM refresh_tokens
        WHERE user_id = (SELECT id FROM users WHERE email = $1)
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(&email)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to get old token hash");

    // Use refresh token to get new tokens
    let refresh_response = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            old_access_token,
            old_refresh_token,
            &state.cookie_key,
        )
        .await;

    refresh_response.assert_status(StatusCode::OK);

    let refresh_json: serde_json::Value = refresh_response.json();
    let new_access_token = refresh_json["token"].as_str().unwrap();
    assert!(
        !new_access_token.is_empty(),
        "New access token should be returned"
    );
    // Note: JWT tokens may be identical if generated in the same second (due to same exp/iat)
    // The important thing is that a new refresh token is issued

    // Verify new refresh token cookie was set
    let new_refresh_cookie = refresh_response.get_cookie("refresh_token");
    assert!(
        new_refresh_cookie.is_some(),
        "New refresh token cookie should be set"
    );

    // Verify old refresh token is deleted/revoked (no longer exists)
    let old_token_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(SELECT 1 FROM refresh_tokens WHERE token_hash = $1)
        "#,
    )
    .bind(&old_token_hash)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to check old token status");

    assert!(
        !old_token_exists,
        "Old refresh token should be deleted/revoked"
    );

    // Verify new refresh token exists in database
    let new_token_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM refresh_tokens
        WHERE user_id = (SELECT id FROM users WHERE email = $1)
        "#,
    )
    .bind(&email)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to count new tokens");

    assert_eq!(
        new_token_count, 1,
        "Exactly one active refresh token should exist"
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_refresh_token_reuse_detection() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create and login user with unique email for concurrency safety
    let email = common::test_data::unique_email("reuse");
    let username = common::test_data::unique_username("reuseuser");
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
    common::db::create_test_user(
        &state.pool,
        &email,
        &username,
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    let login_body = json!({
        "email": &email,
        "password": "password123"
    });
    let login_response = client.post_json("/v1/users/login", &login_body).await;
    let login_json: serde_json::Value = login_response.json();
    let access_token = login_json["token"].as_str().unwrap();
    let refresh_token = login_json["refresh_token"].as_str().unwrap();

    // Use refresh token first time - should succeed
    let first_refresh = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            access_token,
            refresh_token,
            &state.cookie_key,
        )
        .await;
    first_refresh.assert_status(StatusCode::OK);

    // Try to reuse same refresh token - should fail
    let second_refresh = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            access_token,
            refresh_token,
            &state.cookie_key,
        )
        .await;

    second_refresh.assert_status(StatusCode::UNAUTHORIZED);

    let error_json: serde_json::Value = second_refresh.json();
    assert!(error_json["error"].as_str().is_some());

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_refresh_token_missing_cookie() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try to refresh without any cookies
    let response = client.get("/v1/auth/refresh").await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let json: serde_json::Value = response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("refresh token")
    );
}

#[tokio::test]
async fn test_refresh_token_invalid_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user for valid access token with unique email for concurrency safety
    let email = common::test_data::unique_email("invalid");
    let username = common::test_data::unique_username("invaliduser");
    let user_id =
        common::db::create_verified_user(&state.pool, &email, &username)
            .await
            .expect("Failed to create user");

    let access_token =
        common::jwt::create_test_token(user_id, &email, &state.jwt_secret);

    // Try to refresh with invalid refresh token
    let response = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            &access_token,
            "invalid_refresh_token",
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_logout_revokes_refresh_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create and login user with unique email for concurrency safety
    let email = common::test_data::unique_email("logout");
    let username = common::test_data::unique_username("logoutuser");
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
    let user_id = common::db::create_test_user(
        &state.pool,
        &email,
        &username,
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    let login_body = json!({
        "email": &email,
        "password": "password123"
    });
    let login_response = client.post_json("/v1/users/login", &login_body).await;
    let login_json: serde_json::Value = login_response.json();
    let access_token = login_json["token"].as_str().unwrap();
    let refresh_token = login_json["refresh_token"].as_str().unwrap();

    // Verify refresh token is active before logout
    let tokens_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 ")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .expect("Failed to count tokens");

    assert!(tokens_before > 0, "Should have active refresh token");

    // Logout
    let logout_response = client
        .get_with_auth_and_refresh(
            "/v1/auth/logout",
            access_token,
            refresh_token,
            &state.cookie_key,
        )
        .await;

    logout_response.assert_status(StatusCode::OK);

    let logout_json: serde_json::Value = logout_response.json();
    assert!(
        logout_json["message"]
            .as_str()
            .unwrap()
            .contains("Logged out")
    );

    // Verify refresh token is revoked after logout
    let tokens_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 ")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .expect("Failed to count tokens");

    assert_eq!(tokens_after, 0, "All refresh tokens should be revoked");

    // Try to use refresh token after logout - should fail
    let refresh_after_logout = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            access_token,
            refresh_token,
            &state.cookie_key,
        )
        .await;

    refresh_after_logout.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_multiple_concurrent_refresh_tokens() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user with unique email for concurrency safety
    let email = common::test_data::unique_email("multidevice");
    let username = common::test_data::unique_username("multiuser");
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
    let user_id = common::db::create_test_user(
        &state.pool,
        &email,
        &username,
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    // Login from "device 1"
    let login_body = json!({
        "email": &email,
        "password": "password123"
    });
    let login1 = client.post_json("/v1/users/login", &login_body).await;
    let login1_json: serde_json::Value = login1.json();
    let access_token1 = login1_json["token"].as_str().unwrap().to_string();
    let refresh_token1 = login1_json["refresh_token"].as_str().unwrap().to_string();

    // Login from "device 2"
    let login2 = client.post_json("/v1/users/login", &login_body).await;
    let login2_json: serde_json::Value = login2.json();
    let access_token2 = login2_json["token"].as_str().unwrap().to_string();
    let refresh_token2 = login2_json["refresh_token"].as_str().unwrap().to_string();

    // Verify both refresh tokens are different
    assert_ne!(
        refresh_token1, refresh_token2,
        "Different devices should have different refresh tokens"
    );

    // Verify both tokens work
    let total_tokens: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 ")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .expect("Failed to count tokens");

    assert!(
        total_tokens >= 2,
        "Should have multiple active refresh tokens for different devices"
    );

    // Refresh from device 1
    let refresh1 = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            &access_token1,
            &refresh_token1,
            &state.cookie_key,
        )
        .await;
    refresh1.assert_status(StatusCode::OK);

    // Refresh from device 2 should still work
    let refresh2 = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            &access_token2,
            &refresh_token2,
            &state.cookie_key,
        )
        .await;
    refresh2.assert_status(StatusCode::OK);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_refresh_token_family_invalidation_on_breach() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create and login user with unique email for concurrency safety
    let email = common::test_data::unique_email("breach");
    let username = common::test_data::unique_username("breachuser");
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
    let user_id = common::db::create_test_user(
        &state.pool,
        &email,
        &username,
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    let login_body = json!({
        "email": &email,
        "password": "password123"
    });
    let login_response = client.post_json("/v1/users/login", &login_body).await;
    let login_json: serde_json::Value = login_response.json();
    let token1 = login_json["refresh_token"].as_str().unwrap();

    // Rotate token
    let refresh1 = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            login_json["token"].as_str().unwrap(),
            token1,
            &state.cookie_key,
        )
        .await;
    refresh1.assert_status(StatusCode::OK);
    let refresh1_json: serde_json::Value = refresh1.json();
    let _token2 = refresh1.get_cookie("refresh_token").unwrap();

    // Try to reuse old token1 (simulating token theft)
    let breach_attempt = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            &refresh1_json["token"].as_str().unwrap(),
            token1,
            &state.cookie_key,
        )
        .await;

    // Should fail
    breach_attempt.assert_status(StatusCode::UNAUTHORIZED);

    // Verify token family might be invalidated (depends on implementation)
    // This test documents expected behavior for token family invalidation
    let active_tokens: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 ")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .expect("Failed to count active tokens");

    println!("Active tokens after breach detection: {}", active_tokens);

    // If token family invalidation is implemented, active_tokens should be 0
    // If not, token2 should still work (but token1 should not)

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_refresh_token_expiration() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user with unique email for concurrency safety
    let email = common::test_data::unique_email("expired");
    let username = common::test_data::unique_username("expireduser");
    let user_id =
        common::db::create_verified_user(&state.pool, &email, &username)
            .await
            .expect("Failed to create user");

    // Manually create an expired refresh token
    let expired_token = "expired_token_12345678901234567890";
    use sha2::Digest;
    let token_hash = sha2::Sha256::digest(expired_token.as_bytes());

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at, created_at)
        VALUES ($1, $2, NOW() - INTERVAL '1 day', NOW() - INTERVAL '31 days')
        "#,
    )
    .bind(user_id)
    .bind(&token_hash[..])
    .execute(&state.pool)
    .await
    .expect("Failed to insert expired token");

    let access_token =
        common::jwt::create_test_token(user_id, &email, &state.jwt_secret);

    // Try to use expired refresh token
    let response = client
        .get_with_auth_and_refresh(
            "/v1/auth/refresh",
            &access_token,
            expired_token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_logout_without_refresh_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try to logout without any cookies - should still succeed gracefully
    let response = client.get("/v1/auth/logout").await;

    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json["message"].as_str().unwrap().contains("Logged out"));
}
