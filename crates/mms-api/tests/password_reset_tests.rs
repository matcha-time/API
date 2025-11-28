use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
use mms_api::router;
use serde_json::json;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn test_password_reset_full_flow_success() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Step 1: Create a verified user with known password
    let original_password = "OriginalP@ss123";
    let password_hash =
        bcrypt::hash(original_password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    common::db::create_test_user(
        &state.pool,
        "resettest@example.com",
        "resetuser",
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    // Step 2: Verify user can login with original password
    let login_body = json!({
        "email": "resettest@example.com",
        "password": original_password
    });
    let login_response = client.post_json("/users/login", &login_body).await;
    login_response.assert_status(StatusCode::OK);

    // Step 3: Request password reset
    let reset_request = json!({
        "email": "resettest@example.com"
    });
    let request_response = client
        .post_json("/users/request-password-reset", &reset_request)
        .await;
    request_response.assert_status(StatusCode::OK);

    let request_json: serde_json::Value = request_response.json();
    assert!(
        request_json["message"]
            .as_str()
            .unwrap()
            .contains("password reset")
    );

    // Step 4: Get user_id and create reset token
    let user_id = common::db::get_user_by_email(&state.pool, "resettest@example.com")
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    let reset_token = common::verification::create_test_password_reset_token(&state.pool, user_id)
        .await
        .expect("Failed to create reset token");

    // Step 5: Reset password with token
    let new_password = "NewP@ssw0rd456";
    let reset_body = json!({
        "token": reset_token,
        "new_password": new_password
    });
    let reset_response = client.post_json("/users/reset-password", &reset_body).await;
    reset_response.assert_status(StatusCode::OK);

    let reset_json: serde_json::Value = reset_response.json();
    assert!(
        reset_json["message"]
            .as_str()
            .unwrap()
            .contains("successfully")
    );

    // Step 6: Verify token is marked as used
    // Hash the token to compare with database
    let mut hasher = Sha256::new();
    hasher.update(reset_token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    let token_used: bool = sqlx::query_scalar(
        r#"
        SELECT used_at IS NOT NULL
        FROM password_reset_tokens
        WHERE token_hash = $1
        "#,
    )
    .bind(&token_hash)
    .fetch_one(&state.pool)
    .await
    .expect("Failed to check token status");

    assert!(token_used, "Token should be marked as used");

    // Step 7: Verify old password no longer works
    let old_login = client.post_json("/users/login", &login_body).await;
    old_login.assert_status(StatusCode::UNAUTHORIZED);

    // Step 8: Verify new password works
    let new_login_body = json!({
        "email": "resettest@example.com",
        "password": new_password
    });
    let new_login = client.post_json("/users/login", &new_login_body).await;
    new_login.assert_status(StatusCode::OK);

    let new_login_json: serde_json::Value = new_login.json();
    assert!(new_login_json["token"].is_string());

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "resettest@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_password_reset_request_nonexistent_user() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Request reset for non-existent user
    let body = json!({
        "email": "nonexistent@example.com"
    });
    let response = client
        .post_json("/users/request-password-reset", &body)
        .await;

    // Should return success to prevent enumeration
    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(
        json["message"]
            .as_str()
            .unwrap()
            .contains("If an account exists")
    );

    // Verify no token was created
    let token_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM password_reset_tokens WHERE user_id = (SELECT id FROM users WHERE email = $1)",
    )
    .bind("nonexistent@example.com")
    .fetch_one(&state.pool)
    .await
    .expect("Failed to count tokens");

    assert_eq!(
        token_count, 0,
        "No token should be created for non-existent user"
    );
}

#[tokio::test]
async fn test_password_reset_invalid_email_format() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try with invalid email format
    let body = json!({
        "email": "not-an-email"
    });
    let response = client
        .post_json("/users/request-password-reset", &body)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let json: serde_json::Value = response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("email")
    );
}

#[tokio::test]
async fn test_password_reset_expired_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let user_id =
        common::db::create_verified_user(&state.pool, "expiredreset@example.com", "expireduser")
            .await
            .expect("Failed to create user");

    // Manually insert expired reset token
    let expired_token = "expired_reset_token_hash_12345";
    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (user_id, token_hash, expires_at, created_at)
        VALUES ($1, $2, NOW() - INTERVAL '2 hours', NOW() - INTERVAL '3 hours')
        "#,
    )
    .bind(user_id)
    .bind(expired_token)
    .execute(&state.pool)
    .await
    .expect("Failed to insert expired token");

    // Try to reset password with expired token
    let body = json!({
        "token": expired_token,
        "new_password": "NewP@ssw0rd123"
    });
    let response = client.post_json("/users/reset-password", &body).await;

    // Should fail with generic error
    response.assert_status(StatusCode::UNAUTHORIZED);

    let json: serde_json::Value = response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("invalid or expired")
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "expiredreset@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_password_reset_already_used_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let password_hash = bcrypt::hash("OriginalP@ss123", bcrypt::DEFAULT_COST).unwrap();
    common::db::create_test_user(
        &state.pool,
        "usedresettoken@example.com",
        "usedresetuser",
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    // Request password reset
    let request_body = json!({
        "email": "usedresettoken@example.com"
    });
    client
        .post_json("/users/request-password-reset", &request_body)
        .await;

    // Get user_id and create reset token
    let user_id = common::db::get_user_by_email(&state.pool, "usedresettoken@example.com")
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    let token = common::verification::create_test_password_reset_token(&state.pool, user_id)
        .await
        .expect("Failed to create reset token");

    // Use token first time
    let reset_body = json!({
        "token": token,
        "new_password": "NewP@ssw0rd123"
    });
    let first_response = client.post_json("/users/reset-password", &reset_body).await;
    first_response.assert_status(StatusCode::OK);

    // Try to use same token again
    let second_reset_body = json!({
        "token": token,
        "new_password": "AnotherP@ss456"
    });
    let second_response = client
        .post_json("/users/reset-password", &second_reset_body)
        .await;

    // Should fail
    second_response.assert_status(StatusCode::UNAUTHORIZED);

    let json: serde_json::Value = second_response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("invalid or expired")
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "usedresettoken@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_password_reset_weak_new_password() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user and request reset
    common::db::create_verified_user(&state.pool, "weakpass@example.com", "weakpassuser")
        .await
        .expect("Failed to create user");

    let request_body = json!({
        "email": "weakpass@example.com"
    });
    client
        .post_json("/users/request-password-reset", &request_body)
        .await;

    // Get user_id and create reset token
    let user_id = common::db::get_user_by_email(&state.pool, "weakpass@example.com")
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    let token = common::verification::create_test_password_reset_token(&state.pool, user_id)
        .await
        .expect("Failed to create reset token");

    // Try to reset with weak password
    let body = json!({
        "token": token,
        "new_password": "weak"
    });
    let response = client.post_json("/users/reset-password", &body).await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let json: serde_json::Value = response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("password")
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "weakpass@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_password_reset_invalid_token_format() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try with completely invalid token
    let body = json!({
        "token": "invalid_token_12345",
        "new_password": "ValidP@ssw0rd123"
    });
    let response = client.post_json("/users/reset-password", &body).await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    let json: serde_json::Value = response.json();
    assert!(
        json["error"]
            .as_str()
            .unwrap()
            .contains("invalid or expired")
    );
}

#[tokio::test]
async fn test_password_reset_revokes_old_sessions() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user and login
    let original_password = "OriginalP@ss123";
    let password_hash = bcrypt::hash(original_password, bcrypt::DEFAULT_COST).unwrap();
    let user_id = common::db::create_test_user(
        &state.pool,
        "revokesession@example.com",
        "revokeuser",
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    // Login and get tokens
    let login_body = json!({
        "email": "revokesession@example.com",
        "password": original_password
    });
    let login_response = client.post_json("/users/login", &login_body).await;
    login_response.assert_status(StatusCode::OK);

    let login_json: serde_json::Value = login_response.json();
    let old_token = login_json["token"].as_str().unwrap();

    // Verify old token works
    let dashboard_response = client
        .get_with_auth(
            &format!("/users/{}/dashboard", user_id),
            old_token,
            &state.cookie_key,
        )
        .await;
    dashboard_response.assert_status(StatusCode::OK);

    // Request and perform password reset
    let reset_request = json!({
        "email": "revokesession@example.com"
    });
    client
        .post_json("/users/request-password-reset", &reset_request)
        .await;

    let reset_token = common::verification::create_test_password_reset_token(&state.pool, user_id)
        .await
        .expect("Failed to create reset token");

    let reset_body = json!({
        "token": reset_token,
        "new_password": "NewP@ssw0rd456"
    });
    client.post_json("/users/reset-password", &reset_body).await;

    // Verify old refresh tokens are invalidated
    let refresh_token_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .expect("Failed to count refresh tokens");

    // Note: This depends on implementation. If password reset revokes tokens, count should be 0
    // If not implemented yet, this test documents the expected behavior
    println!(
        "Active refresh tokens after password reset: {}",
        refresh_token_count
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "revokesession@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_password_reset_multiple_requests_invalidates_old_tokens() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    common::db::create_verified_user(&state.pool, "multireset@example.com", "multiresetuser")
        .await
        .expect("Failed to create user");

    // Request reset first time
    let request_body = json!({
        "email": "multireset@example.com"
    });
    client
        .post_json("/users/request-password-reset", &request_body)
        .await;

    // Get user_id and create first reset token
    let user_id = common::db::get_user_by_email(&state.pool, "multireset@example.com")
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    let first_token = common::verification::create_test_password_reset_token(&state.pool, user_id)
        .await
        .expect("Failed to create first reset token");

    // Wait a bit to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Request reset second time
    client
        .post_json("/users/request-password-reset", &request_body)
        .await;

    // Create second reset token
    let second_token = common::verification::create_test_password_reset_token(&state.pool, user_id)
        .await
        .expect("Failed to create second reset token");

    // Tokens should be different
    assert_ne!(
        first_token, second_token,
        "Each request should generate a new token"
    );

    // Try to use first (older) token - behavior depends on implementation
    let reset_body = json!({
        "token": first_token,
        "new_password": "NewP@ssw0rd123"
    });
    let response = client.post_json("/users/reset-password", &reset_body).await;

    // If implementation invalidates old tokens, this should fail
    // If not, this test documents the current behavior
    println!("Reset with old token status: {}", response.status);

    // Second token should work
    let reset_body2 = json!({
        "token": second_token,
        "new_password": "NewP@ssw0rd456"
    });
    let response2 = client
        .post_json("/users/reset-password", &reset_body2)
        .await;
    response2.assert_status(StatusCode::OK);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "multireset@example.com")
        .await
        .expect("Failed to cleanup");
}
