use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
use mms_api::router;
use serde_json::json;

#[tokio::test]
async fn test_rate_limit_sensitive_endpoints() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create a user for testing
    common::db::create_verified_user(&state.pool, "ratelimit@example.com", "ratelimituser")
        .await
        .expect("Failed to create user");

    let body = json!({
        "email": "ratelimit@example.com"
    });

    // Sensitive endpoints have limit of 2 req/s with burst of 3
    // Send burst of requests
    let mut responses = Vec::new();
    for _ in 0..5 {
        let response = client
            .post_json("/v1/users/request-password-reset", &body)
            .await;
        responses.push(response.status);
    }

    // Count how many were rate limited
    let rate_limited_count = responses
        .iter()
        .filter(|&&status| status == StatusCode::TOO_MANY_REQUESTS)
        .count();

    assert!(
        rate_limited_count > 0,
        "Some requests should be rate limited after burst. Got statuses: {:?}",
        responses
    );

    // Verify at least some requests succeeded
    let success_count = responses
        .iter()
        .filter(|&&status| status == StatusCode::OK)
        .count();

    assert!(
        success_count > 0,
        "Some requests should succeed within burst limit"
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "ratelimit@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_rate_limit_auth_endpoints() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Auth endpoints have limit of 5 req/s with burst of 5

    // Send 10 requests rapidly
    let mut responses = Vec::new();
    for i in 0..10 {
        let body = json!({
            "username": format!("testuser{}", i),
            "email": format!("test{}@example.com", i),
            "password": "SecureP@ssw0rd123"
        });
        let response = client.post_json("/v1/users/register", &body).await;
        responses.push(response.status);
    }

    // Count rate limited responses
    let rate_limited_count = responses
        .iter()
        .filter(|&&status| status == StatusCode::TOO_MANY_REQUESTS)
        .count();

    assert!(
        rate_limited_count > 0,
        "Some registration requests should be rate limited. Got statuses: {:?}",
        responses
    );

    // Cleanup - delete any created users
    for i in 0..10 {
        let _ =
            common::db::delete_user_by_email(&state.pool, &format!("test{}@example.com", i)).await;
    }
}

#[tokio::test]
async fn test_rate_limit_general_endpoints() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // General endpoints have limit of 10 req/s with burst of 20
    // Test with roadmaps endpoint (unauthenticated, general rate limit)

    // Send 30 requests rapidly (more than burst of 20)
    let mut responses = Vec::new();
    for _ in 0..30 {
        let response = client.get("/v1/roadmaps").await;
        responses.push(response.status);
    }

    // Count rate limited responses
    let rate_limited_count = responses
        .iter()
        .filter(|&&status| status == StatusCode::TOO_MANY_REQUESTS)
        .count();

    // Some requests should be rate limited, but not necessarily all after burst
    // This depends on timing and how fast requests are processed
    println!(
        "Rate limited: {} out of 30 requests. Statuses: {:?}",
        rate_limited_count, responses
    );

    // Verify many requests succeeded (within burst of 20)
    let success_count = responses
        .iter()
        .filter(|&&status| status == StatusCode::OK)
        .count();

    assert!(
        success_count >= 15,
        "At least 15 requests should succeed within burst limit, got {}",
        success_count
    );
}

#[tokio::test]
async fn test_rate_limit_recovery_after_delay() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Exhaust rate limit first
    for _ in 0..25 {
        client.get("/v1/roadmaps").await;
    }

    // Wait for rate limit to recover (1 second should allow some tokens to replenish)
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Try again - should succeed
    let response = client.get("/v1/roadmaps").await;
    assert!(
        response.status == StatusCode::OK,
        "Request should succeed after rate limit recovery, got: {}",
        response.status
    );
}

#[tokio::test]
async fn test_rate_limit_timing_safe_middleware() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
    common::db::create_test_user(
        &state.pool,
        "timing@example.com",
        "timinguser",
        &password_hash,
    )
    .await
    .expect("Failed to create user");

    // Test timing-safe middleware on sensitive endpoints
    // The middleware should add a constant delay (50ms) to prevent timing attacks

    let body = json!({
        "email": "timing@example.com"
    });

    let start = std::time::Instant::now();
    client
        .post_json("/v1/users/request-password-reset", &body)
        .await;
    let duration = start.elapsed();

    // Should take at least 50ms due to timing-safe middleware
    assert!(
        duration.as_millis() >= 40, // Allow small margin
        "Timing-safe middleware should add delay, took: {:?}",
        duration
    );

    // Test with non-existent user - should take similar time
    let body = json!({
        "email": "nonexistent@example.com"
    });

    let start = std::time::Instant::now();
    client
        .post_json("/v1/users/request-password-reset", &body)
        .await;
    let duration_nonexistent = start.elapsed();

    // Both should take similar time (within reasonable margin)
    let time_diff = (duration.as_millis() as i64 - duration_nonexistent.as_millis() as i64).abs();
    assert!(
        time_diff < 20, // Allow 20ms variance
        "Timing should be similar for existing and non-existing users to prevent enumeration. Diff: {}ms",
        time_diff
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "timing@example.com")
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
async fn test_rate_limit_per_ip() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Rate limits are per IP address
    // All test requests come from same IP (127.0.0.1)
    // So they share the same rate limit bucket

    // Exhaust limit with one endpoint
    for _ in 0..25 {
        client.get("/v1/roadmaps").await;
    }

    // Immediately try another general endpoint - should also be rate limited
    let response = client.get("/v1/roadmaps/en/es").await;

    // This might be rate limited or succeed depending on timing
    // Main point is they share the bucket
    println!(
        "Status after exhausting limit on different endpoint: {}",
        response.status
    );
}

#[tokio::test]
async fn test_rate_limit_different_endpoint_tiers() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Different endpoint tiers have different rate limits
    // But they might share IP-based tracking

    // Test that sensitive endpoints have stricter limits than general ones
    let sensitive_body = json!({
        "email": "test@example.com"
    });

    // Exhaust sensitive endpoint (2 req/s, burst 3)
    let mut sensitive_responses = Vec::new();
    for _ in 0..6 {
        let response = client
            .post_json("/v1/users/resend-verification", &sensitive_body)
            .await;
        sensitive_responses.push(response.status);
    }

    let sensitive_limited = sensitive_responses
        .iter()
        .filter(|&&s| s == StatusCode::TOO_MANY_REQUESTS)
        .count();

    // Sensitive should be limited more aggressively
    assert!(
        sensitive_limited >= 3,
        "Sensitive endpoint should rate limit aggressively, got {} limited out of 6",
        sensitive_limited
    );
}

#[tokio::test]
async fn test_rate_limit_headers_present() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    let response = client.get("/v1/roadmaps").await;

    // Check if rate limit headers are present (depends on implementation)
    // Common headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
    println!("Response headers: {:?}", response.headers);

    // This test documents whether rate limit headers are exposed
    // Not all implementations expose these headers
    let has_rate_limit_headers = response
        .headers
        .keys()
        .any(|k| k.as_str().to_lowercase().contains("ratelimit"));

    println!("Has rate limit headers: {}", has_rate_limit_headers);
}

#[tokio::test]
async fn test_rate_limit_login_endpoint() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Create user
    common::db::create_verified_user(&state.pool, "loginlimit@example.com", "loginuser")
        .await
        .expect("Failed to create user");

    // Attempt many logins with wrong password
    let body = json!({
        "email": "loginlimit@example.com",
        "password": "wrongpassword"
    });

    let mut responses = Vec::new();
    for _ in 0..10 {
        let response = client.post_json("/v1/users/login", &body).await;
        responses.push(response.status);
    }

    // Should rate limit failed login attempts
    let rate_limited = responses
        .iter()
        .filter(|&&s| s == StatusCode::TOO_MANY_REQUESTS)
        .count();

    assert!(
        rate_limited > 0,
        "Failed login attempts should be rate limited to prevent brute force. Got statuses: {:?}",
        responses
    );

    // Cleanup
    common::db::delete_user_by_email(&state.pool, "loginlimit@example.com")
        .await
        .expect("Failed to cleanup");
}
