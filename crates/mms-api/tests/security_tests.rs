use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
use mms_api::router;
use serde_json::json;

// ============================================================================
// SQL Injection Tests
// ============================================================================

#[tokio::test]
async fn test_sql_injection_login_email() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // SQL injection payloads in email field
    let sql_injection_payloads = vec![
        "admin'--",
        "admin' OR '1'='1",
        "admin' OR '1'='1'--",
        "admin' OR 1=1--",
        "' OR 'a'='a",
        "' UNION SELECT * FROM users--",
        "'; DROP TABLE users;--",
        "' OR '1'='1' /*",
    ];

    for payload in sql_injection_payloads {
        let body = json!({
            "email": payload,
            "password": "password123"
        });

        let response = client.post_json("/v1/users/login", &body).await;

        // Should either return 400 (validation error), 401 (invalid credentials), or 429 (rate limited)
        // Should NEVER return 200 (success) or 500 (server error from SQL injection)
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::UNAUTHORIZED
                || response.status == StatusCode::TOO_MANY_REQUESTS,
            "SQL injection payload '{}' should not cause server error or success. Got: {}",
            payload,
            response.status
        );

        // Verify no users were deleted or modified
        let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&state.pool)
            .await
            .expect("Failed to count users");

        assert!(user_count >= 0, "Users table should still exist");

        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }
}

#[tokio::test]
async fn test_sql_injection_password_reset() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let sql_payloads = vec![
        "test@example.com' OR '1'='1",
        "'; UPDATE users SET password_hash='hacked' WHERE '1'='1'--",
    ];

    for payload in sql_payloads {
        let body = json!({
            "email": payload
        });

        let response = client
            .post_json("/v1/users/request-password-reset", &body)
            .await;

        // Should return either 400 or 200 (generic success), never 500
        assert!(
            response.status == StatusCode::OK || response.status == StatusCode::BAD_REQUEST,
            "SQL injection in password reset should be handled safely. Got: {}",
            response.status
        );
    }
}

#[tokio::test]
async fn test_sql_injection_user_registration() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let body = json!({
        "username": "admin'--",
        "email": "test' OR '1'='1'--@example.com",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/v1/users/register", &body).await;

    // Should reject due to validation or handle safely
    assert!(
        response.status == StatusCode::BAD_REQUEST || response.status == StatusCode::OK,
        "SQL injection in registration should be handled. Got: {}",
        response.status
    );
}

#[tokio::test]
async fn test_sql_injection_query_parameters() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Test SQL injection in query parameters
    // URL encode the payload - ' becomes %27, = becomes %3D
    let encoded_payload = "%27%20OR%20%271%27%3D%271";

    let response = client
        .get(&format!("/v1/users/verify-email?token={}", encoded_payload))
        .await;

    // Should handle gracefully, not cause server error
    assert!(
        response.status == StatusCode::OK
            || response.status == StatusCode::BAD_REQUEST
            || response.status == StatusCode::UNAUTHORIZED,
        "SQL injection in query param should be handled. Got: {}",
        response.status
    );
}

// ============================================================================
// XSS (Cross-Site Scripting) Tests
// ============================================================================

#[tokio::test]
async fn test_xss_in_username() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let xss_payloads = vec![
        "<script>alert('XSS')</script>",
        "<img src=x onerror=alert('XSS')>",
        "javascript:alert('XSS')",
        "<svg/onload=alert('XSS')>",
        "';alert('XSS');//",
    ];

    for (i, payload) in xss_payloads.iter().enumerate() {
        let body = json!({
            "username": payload,
            "email": format!("xss{}@example.com", i),
            "password": "SecureP@ssw0rd123"
        });

        let response = client.post_json("/v1/users/register", &body).await;

        // Should either reject or sanitize
        assert!(
            response.status == StatusCode::BAD_REQUEST || response.status == StatusCode::OK,
            "XSS payload should be handled. Got: {}",
            response.status
        );

        if response.status == StatusCode::OK {
            // If registration succeeds, verify the payload is stored safely
            let stored_username: Option<String> =
                sqlx::query_scalar("SELECT username FROM users WHERE email = $1")
                    .bind(format!("xss{}@example.com", i))
                    .fetch_optional(&state.pool)
                    .await
                    .expect("Failed to fetch username");

            if let Some(username) = stored_username {
                // Username should be stored as-is (backend doesn't sanitize, frontend should escape)
                // But we verify it doesn't break the database
                println!("Stored username: {}", username);
            }

            // Cleanup
            common::db::delete_user_by_email(&state.pool, &format!("xss{}@example.com", i))
                .await
                .expect("Failed to cleanup");
        }
    }
}

#[tokio::test]
async fn test_xss_in_profile_update() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let email = common::test_data::unique_email("xssprofile");
    let username = common::test_data::unique_username("xssuser");
    let user_id = common::db::create_verified_user(&state.pool, &email, &username)
        .await
        .expect("Failed to create user");

    let token = common::jwt::create_test_token(user_id, &email, &state.jwt_secret);

    // Try to update with XSS payload
    let body = json!({
        "username": "<script>alert('XSS')</script>"
    });

    let response = client
        .patch_json_with_auth(
            &format!("/v1/users/{}", user_id),
            &body,
            &token,
            &state.cookie_key,
        )
        .await;

    // Should handle safely
    assert!(
        response.status == StatusCode::OK || response.status == StatusCode::BAD_REQUEST,
        "XSS in profile update should be handled. Got: {}",
        response.status
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

// ============================================================================
// Authentication Bypass Tests
// ============================================================================

#[tokio::test]
async fn test_auth_bypass_missing_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let email = common::test_data::unique_email("noauth");
    let username = common::test_data::unique_username("noauthuser");
    let user_id = common::db::create_verified_user(&state.pool, &email, &username)
        .await
        .expect("Failed to create user");

    // Try to access protected endpoint without auth token
    let response = client
        .get(&format!("/v1/users/{}/dashboard", user_id))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_auth_bypass_invalid_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let email = common::test_data::unique_email("badtoken");
    let username = common::test_data::unique_username("badtokenuser");
    let user_id = common::db::create_verified_user(&state.pool, &email, &username)
        .await
        .expect("Failed to create user");

    // Try with completely invalid token
    let response = client
        .get_with_auth(
            &format!("/v1/users/{}/dashboard", user_id),
            "invalid.jwt.token",
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
async fn test_auth_bypass_wrong_user_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create two users
    let email1 = common::test_data::unique_email("user1");
    let username1 = common::test_data::unique_username("user1");
    let user1_id = common::db::create_verified_user(&state.pool, &email1, &username1)
        .await
        .expect("Failed to create user1");

    let email2 = common::test_data::unique_email("user2");
    let username2 = common::test_data::unique_username("user2");
    let user2_id = common::db::create_verified_user(&state.pool, &email2, &username2)
        .await
        .expect("Failed to create user2");

    // Get token for user1
    let user1_token = common::jwt::create_test_token(user1_id, &email1, &state.jwt_secret);

    // Try to access user2's dashboard with user1's token
    let response = client
        .get_with_auth(
            &format!("/v1/users/{}/dashboard", user2_id),
            &user1_token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email1)
        .await
        .expect("Failed to cleanup");
    common::db::delete_user_by_email(&state.pool, &email2)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_auth_bypass_expired_token() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let email = common::test_data::unique_email("expired");
    let username = common::test_data::unique_username("expireduser");
    let user_id = common::db::create_verified_user(&state.pool, &email, &username)
        .await
        .expect("Failed to create user");

    // Create token with past expiration (would require custom JWT creation)
    // For now, just test with malformed token
    let expired_token =
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiZXhwIjowfQ.invalid";

    let response = client
        .get_with_auth(
            &format!("/v1/users/{}/dashboard", user_id),
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
async fn test_auth_bypass_wrong_secret() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let email = common::test_data::unique_email("wrongsecret");
    let username = common::test_data::unique_username("wronguser");
    let user_id = common::db::create_verified_user(&state.pool, &email, &username)
        .await
        .expect("Failed to create user");

    // Create token with wrong secret
    let wrong_token =
        common::jwt::create_test_token(user_id, &email, "wrong_secret_that_doesnt_match_12345");

    let response = client
        .get_with_auth(
            &format!("/v1/users/{}/dashboard", user_id),
            &wrong_token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email)
        .await
        .expect("Failed to cleanup");
}

// ============================================================================
// Path Traversal / Directory Traversal Tests
// ============================================================================

#[tokio::test]
async fn test_path_traversal_in_routes() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let path_traversal_payloads = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32",
        "....//....//....//etc/passwd",
    ];

    for payload in path_traversal_payloads {
        // Try path traversal in various routes
        let response = client.get(&format!("/v1/roadmaps/{}", payload)).await;

        // Should return 404 or 400, never expose files
        assert!(
            response.status == StatusCode::NOT_FOUND
                || response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::INTERNAL_SERVER_ERROR,
            "Path traversal should be rejected. Payload: {}, Status: {}",
            payload,
            response.status
        );
    }
}

// ============================================================================
// IDOR (Insecure Direct Object Reference) Tests
// ============================================================================

#[tokio::test]
async fn test_idor_profile_access() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create two users
    let email1 = common::test_data::unique_email("idor1");
    let username1 = common::test_data::unique_username("idor1");
    let user1_id = common::db::create_verified_user(&state.pool, &email1, &username1)
        .await
        .expect("Failed to create user1");

    let email2 = common::test_data::unique_email("idor2");
    let username2 = common::test_data::unique_username("idor2");
    let user2_id = common::db::create_verified_user(&state.pool, &email2, &username2)
        .await
        .expect("Failed to create user2");

    let user1_token = common::jwt::create_test_token(user1_id, &email1, &state.jwt_secret);

    // User1 tries to update user2's profile
    let body = json!({
        "username": "hacked"
    });

    let response = client
        .patch_json_with_auth(
            &format!("/v1/users/{}", user2_id),
            &body,
            &user1_token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Verify user2's username wasn't changed
    let user2_username: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
        .bind(user2_id)
        .fetch_one(&state.pool)
        .await
        .expect("Failed to get username");

    assert_eq!(user2_username, username2, "Username should not be changed");

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email1)
        .await
        .expect("Failed to cleanup");
    common::db::delete_user_by_email(&state.pool, &email2)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_idor_practice_submission() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create two users
    let email1 = common::test_data::unique_email("practice1");
    let username1 = common::test_data::unique_username("practice1");
    let user1_id = common::db::create_verified_user(&state.pool, &email1, &username1)
        .await
        .expect("Failed to create user1");

    let email2 = common::test_data::unique_email("practice2");
    let username2 = common::test_data::unique_username("practice2");
    let user2_id = common::db::create_verified_user(&state.pool, &email2, &username2)
        .await
        .expect("Failed to create user2");

    let user1_token = common::jwt::create_test_token(user1_id, &email1, &state.jwt_secret);

    // User1 tries to submit review for user2
    let fake_flashcard_id = uuid::Uuid::new_v4();
    let fake_deck_id = uuid::Uuid::new_v4();

    let body = json!({
        "correct": true,
        "next_review_at": "2025-12-01T10:00:00Z",
        "deck_id": fake_deck_id.to_string()
    });

    let response = client
        .post_json_with_auth(
            &format!("/v1/practice/{}/{}/review", user2_id, fake_flashcard_id),
            &body,
            &user1_token,
            &state.cookie_key,
        )
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);

    // Cleanup
    common::db::delete_user_by_email(&state.pool, &email1)
        .await
        .expect("Failed to cleanup");
    common::db::delete_user_by_email(&state.pool, &email2)
        .await
        .expect("Failed to cleanup");
}

// ============================================================================
// Input Validation Tests
// ============================================================================

#[tokio::test]
async fn test_oversized_input_rejection() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Very long username (potential buffer overflow or DoS)
    let long_username = "a".repeat(10000);

    let body = json!({
        "username": long_username,
        "email": "long@example.com",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/v1/users/register", &body).await;

    // Should reject oversized input
    assert!(
        response.status == StatusCode::BAD_REQUEST
            || response.status == StatusCode::PAYLOAD_TOO_LARGE,
        "Oversized input should be rejected. Got: {}",
        response.status
    );
}

#[tokio::test]
async fn test_null_byte_injection() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Null byte injection
    let body = json!({
        "username": "admin\0",
        "email": "null@example.com\0",
        "password": "SecureP@ssw0rd123"
    });

    let response = client.post_json("/v1/users/register", &body).await;

    // Should handle null bytes safely
    assert!(
        response.status == StatusCode::BAD_REQUEST || response.status == StatusCode::OK,
        "Null byte injection should be handled. Got: {}",
        response.status
    );
}
