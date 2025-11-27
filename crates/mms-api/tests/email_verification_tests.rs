use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
use mms_api::router;
use serde_json::json;

#[tokio::test]
async fn test_email_verification_full_flow_success() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Step 1: Register a new user
    let body = json!({
        "username": "emailtest",
        "email": "emailtest@example.com",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/users/register", &body).await;
    response.assert_status(StatusCode::OK);

    // Step 2: Get user_id and create a verification token
    let user_id = common::db::get_user_by_email(&state.pool, "emailtest@example.com")
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    // Create verification token using the helper
    let token = common::verification::create_test_verification_token(&state.pool, user_id)
        .await
        .expect("Failed to create verification token");

    // Step 3: Check if user is verified in database (should be false after registration)
    let email_verified_before: bool = sqlx::query_scalar(
        "SELECT email_verified FROM users WHERE email = $1",
    )
    .bind("emailtest@example.com")
    .fetch_one(&state.pool)
    .await
    .expect("Failed to check email_verified status");

    println!("Email verified status after registration: {}", email_verified_before);

    // Try to login before email verification
    let login_body = json!({
        "email": "emailtest@example.com",
        "password": "SecureP@ssw0rd123"
    });
    let login_response = client.post_json("/users/login", &login_body).await;

    // If user is not verified, login should fail
    if !email_verified_before {
        assert!(
            login_response.status == StatusCode::UNAUTHORIZED
                || login_response.status == StatusCode::FORBIDDEN,
            "User should not be able to login before email verification. Got status: {}, body: {}",
            login_response.status,
            login_response.text()
        );
    }

    // Step 4: Verify email with token
    let verify_response = client
        .get(&format!("/users/verify-email?token={}", token))
        .await;
    verify_response.assert_status(StatusCode::OK);

    let verify_json: serde_json::Value = verify_response.json();
    assert!(verify_json["message"]
        .as_str()
        .unwrap()
        .contains("verified successfully"));

    // Step 5: Verify user's email_verified status in database
    let email_verified: bool = sqlx::query_scalar(
        "SELECT email_verified FROM users WHERE email = $1",
    )
    .bind("emailtest@example.com")
    .fetch_one(&state.pool)
    .await
    .expect("Failed to check email_verified status");

    assert!(email_verified, "User's email should be marked as verified");

    // Step 6: Verify user can now login successfully
    let final_login = client.post_json("/users/login", &login_body).await;
    final_login.assert_status(StatusCode::OK);

    let login_json: serde_json::Value = final_login.json();
    assert!(login_json["token"].is_string());
    assert!(login_json["refresh_token"].is_string());

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "emailtest@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_email_verification_expired_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create a user manually
    let user_id = common::db::create_verified_user(
        &state.pool,
        "expiredtoken@example.com",
        "expireduser",
    )
    .await
    .expect("Failed to create user");

    // Manually insert an expired verification token
    let expired_token = "expired_token_hash_12345678";
    sqlx::query(
        r#"
        INSERT INTO email_verification_tokens (user_id, token_hash, expires_at, created_at)
        VALUES ($1, decode($2, 'hex'), NOW() - INTERVAL '1 day', NOW() - INTERVAL '2 days')
        "#,
    )
    .bind(user_id)
    .bind(expired_token)
    .execute(&state.pool)
    .await
    .expect("Failed to insert expired token");

    // Try to verify with expired token
    let response = client
        .get(&format!("/users/verify-email?token={}", expired_token))
        .await;

    // Should still return success to prevent enumeration
    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    // Generic message to prevent enumeration
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("processed successfully"));

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "expiredtoken@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_email_verification_already_used_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Register user
    let body = json!({
        "username": "usedtoken",
        "email": "usedtoken@example.com",
        "password": "SecureP@ssw0rd123"
    });
    client.post_json("/users/register", &body).await;

    // Get user_id and create verification token
    let user_id = common::db::get_user_by_email(&state.pool, "usedtoken@example.com")
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    let token = common::verification::create_test_verification_token(&state.pool, user_id)
        .await
        .expect("Failed to create verification token");

    // Use token first time
    let first_response = client
        .get(&format!("/users/verify-email?token={}", token))
        .await;
    first_response.assert_status(StatusCode::OK);

    // Try to use same token again
    let second_response = client
        .get(&format!("/users/verify-email?token={}", token))
        .await;

    // Should return generic success to prevent enumeration
    second_response.assert_status(StatusCode::OK);

    let json: serde_json::Value = second_response.json();
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("processed successfully"));

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "usedtoken@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_email_verification_invalid_token_format() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try with completely invalid token
    let response = client
        .get("/users/verify-email?token=invalid_token_12345")
        .await;

    // Should return generic success to prevent enumeration
    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("processed successfully"));
}

#[tokio::test]
async fn test_resend_verification_email_success() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Register user
    let body = json!({
        "username": "resenduser",
        "email": "resenduser@example.com",
        "password": "SecureP@ssw0rd123"
    });
    client.post_json("/users/register", &body).await;

    // Mark user as unverified (in case registration auto-verifies in tests)
    sqlx::query("UPDATE users SET email_verified = false WHERE email = $1")
        .bind("resenduser@example.com")
        .execute(&state.pool)
        .await
        .expect("Failed to mark user as unverified");

    // Get count of verification tokens before resend
    let tokens_before: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM email_verification_tokens
        WHERE user_id = (SELECT id FROM users WHERE email = $1)
        "#,
    )
    .bind("resenduser@example.com")
    .fetch_one(&state.pool)
    .await
    .expect("Failed to count tokens");

    // Resend verification email
    let resend_body = json!({
        "email": "resenduser@example.com"
    });
    let response = client
        .post_json("/users/resend-verification", &resend_body)
        .await;
    response.assert_status(StatusCode::OK);

    // Verify new token was created
    let tokens_after: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM email_verification_tokens
        WHERE user_id = (SELECT id FROM users WHERE email = $1)
        "#,
    )
    .bind("resenduser@example.com")
    .fetch_one(&state.pool)
    .await
    .expect("Failed to count tokens");

    assert!(
        tokens_after > tokens_before,
        "New verification token should be created"
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "resenduser@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_resend_verification_already_verified_user() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create already verified user
    common::db::create_verified_user(&state.pool, "verified@example.com", "verifieduser")
        .await
        .expect("Failed to create verified user");

    // Try to resend verification to already verified user
    let body = json!({
        "email": "verified@example.com"
    });
    let response = client.post_json("/users/resend-verification", &body).await;

    // Should return success to prevent enumeration
    response.assert_status(StatusCode::OK);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "verified@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_resend_verification_nonexistent_user() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Try to resend to non-existent user
    let body = json!({
        "email": "nonexistent@example.com"
    });
    let response = client.post_json("/users/resend-verification", &body).await;

    // Should return success to prevent enumeration
    response.assert_status(StatusCode::OK);

    let json: serde_json::Value = response.json();
    assert!(json["message"].as_str().is_some());
}

#[tokio::test]
async fn test_resend_verification_invalid_email_format() {
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
    let response = client.post_json("/users/resend-verification", &body).await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let json: serde_json::Value = response.json();
    assert!(json["error"].as_str().unwrap().to_lowercase().contains("email"));
}
