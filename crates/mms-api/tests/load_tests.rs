use crate::common::{self, TestClient, TestStateBuilder};
use axum::http::StatusCode;
use mms_api::router;
use serde_json::json;
use std::time::{Duration, Instant};

/// Load test configuration
struct LoadTestConfig {
    concurrent_requests: usize,
    requests_per_client: usize,
    acceptable_avg_latency_ms: u128,
    acceptable_p95_latency_ms: u128,
}

/// Load test results
struct LoadTestResults {
    total_requests: usize,
    successful_requests: usize,
    failed_requests: usize,
    avg_latency_ms: u128,
    p95_latency_ms: u128,
    p99_latency_ms: u128,
    min_latency_ms: u128,
    max_latency_ms: u128,
    total_duration_ms: u128,
    throughput_rps: f64,
}

impl LoadTestResults {
    fn new(latencies: Vec<Duration>, total_duration: Duration, failed: usize) -> Self {
        let mut sorted_latencies: Vec<u128> = latencies.iter().map(|d| d.as_millis()).collect();
        sorted_latencies.sort();

        let total_requests = sorted_latencies.len() + failed;
        let successful_requests = sorted_latencies.len();

        let avg_latency_ms = if !sorted_latencies.is_empty() {
            sorted_latencies.iter().sum::<u128>() / sorted_latencies.len() as u128
        } else {
            0
        };

        let p95_index = (sorted_latencies.len() as f64 * 0.95) as usize;
        let p99_index = (sorted_latencies.len() as f64 * 0.99) as usize;

        let p95_latency_ms = sorted_latencies.get(p95_index).copied().unwrap_or(0);
        let p99_latency_ms = sorted_latencies.get(p99_index).copied().unwrap_or(0);

        let min_latency_ms = sorted_latencies.first().copied().unwrap_or(0);
        let max_latency_ms = sorted_latencies.last().copied().unwrap_or(0);

        let total_duration_ms = total_duration.as_millis();
        let throughput_rps = if total_duration_ms > 0 {
            successful_requests as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            total_requests,
            successful_requests,
            failed_requests: failed,
            avg_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            min_latency_ms,
            max_latency_ms,
            total_duration_ms,
            throughput_rps,
        }
    }

    fn print(&self, test_name: &str) {
        println!("\n========== Load Test Results: {} ==========", test_name);
        println!("Total requests:      {}", self.total_requests);
        println!("Successful:          {}", self.successful_requests);
        println!("Failed:              {}", self.failed_requests);
        println!(
            "Success rate:        {:.2}%",
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        );
        println!("\nLatency:");
        println!("  Min:               {} ms", self.min_latency_ms);
        println!("  Average:           {} ms", self.avg_latency_ms);
        println!("  P95:               {} ms", self.p95_latency_ms);
        println!("  P99:               {} ms", self.p99_latency_ms);
        println!("  Max:               {} ms", self.max_latency_ms);
        println!(
            "\nThroughput:          {:.2} requests/sec",
            self.throughput_rps
        );
        println!("Total duration:      {} ms", self.total_duration_ms);
        println!("================================================\n");
    }
}

#[tokio::test]
#[ignore] // Ignore by default, run with `cargo test --ignored`
async fn load_test_user_registration() {
    let config = LoadTestConfig {
        concurrent_requests: 10,
        requests_per_client: 10,
        acceptable_avg_latency_ms: 100,
        acceptable_p95_latency_ms: 200,
    };

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());

    let start = Instant::now();
    let mut handles = vec![];
    let mut failed = 0;

    for i in 0..config.concurrent_requests {
        let client = TestClient::new(app.clone());

        let handle = tokio::spawn(async move {
            let mut latencies = vec![];

            for j in 0..config.requests_per_client {
                let body = json!({
                    "username": format!("loaduser_{}_{}", i, j),
                    "email": format!("load_{}_{} @example.com", i, j),
                    "password": "SecureP@ssw0rd123"
                });

                let req_start = Instant::now();
                let response = client.post_json("/v1/users/register", &body).await;
                let latency = req_start.elapsed();

                if response.status == StatusCode::OK {
                    latencies.push(latency);
                }
            }

            latencies
        });

        handles.push(handle);
    }

    // Collect all latencies
    let mut all_latencies = vec![];
    for handle in handles {
        match handle.await {
            Ok(latencies) => all_latencies.extend(latencies),
            Err(_) => failed += config.requests_per_client,
        }
    }

    let total_duration = start.elapsed();
    let results = LoadTestResults::new(all_latencies, total_duration, failed);
    results.print("User Registration");

    // Assert performance criteria
    assert!(
        results.avg_latency_ms <= config.acceptable_avg_latency_ms,
        "Average latency {} ms exceeds acceptable {} ms",
        results.avg_latency_ms,
        config.acceptable_avg_latency_ms
    );

    assert!(
        results.p95_latency_ms <= config.acceptable_p95_latency_ms,
        "P95 latency {} ms exceeds acceptable {} ms",
        results.p95_latency_ms,
        config.acceptable_p95_latency_ms
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn load_test_user_login() {
    let config = LoadTestConfig {
        concurrent_requests: 20,
        requests_per_client: 50,
        acceptable_avg_latency_ms: 50,
        acceptable_p95_latency_ms: 150,
    };

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create test users first
    for i in 0..config.concurrent_requests {
        let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
        common::db::create_test_user(
            &state.pool,
            &format!("loginload{}@example.com", i),
            &format!("loginuser{}", i),
            &password_hash,
        )
        .await
        .expect("Failed to create user");
    }

    let app = router::router().with_state(state.clone());

    let start = Instant::now();
    let mut handles = vec![];
    let mut failed = 0;

    for i in 0..config.concurrent_requests {
        let client = TestClient::new(app.clone());

        let handle = tokio::spawn(async move {
            let mut latencies = vec![];

            let body = json!({
                "email": format!("loginload{}@example.com", i),
                "password": "password123"
            });

            for _ in 0..config.requests_per_client {
                let req_start = Instant::now();
                let response = client.post_json("/v1/users/login", &body).await;
                let latency = req_start.elapsed();

                if response.status == StatusCode::OK {
                    latencies.push(latency);
                }
            }

            latencies
        });

        handles.push(handle);
    }

    let mut all_latencies = vec![];
    for handle in handles {
        match handle.await {
            Ok(latencies) => all_latencies.extend(latencies),
            Err(_) => failed += config.requests_per_client,
        }
    }

    let total_duration = start.elapsed();
    let results = LoadTestResults::new(all_latencies, total_duration, failed);
    results.print("User Login");

    assert!(
        results.avg_latency_ms <= config.acceptable_avg_latency_ms,
        "Average latency {} ms exceeds acceptable {} ms",
        results.avg_latency_ms,
        config.acceptable_avg_latency_ms
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn load_test_get_roadmaps() {
    let config = LoadTestConfig {
        concurrent_requests: 50,
        requests_per_client: 100,
        acceptable_avg_latency_ms: 20,
        acceptable_p95_latency_ms: 50,
    };

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create some test roadmaps
    for i in 0..5 {
        let roadmap_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO roadmaps (id, title, description, language_from, language_to, created_at)
            VALUES ($1, $2, 'Test roadmap', 'en', 'es', NOW())
            "#,
        )
        .bind(roadmap_id)
        .bind(format!("Test Roadmap {}", i))
        .execute(&state.pool)
        .await
        .expect("Failed to create roadmap");
    }

    let app = router::router().with_state(state.clone());

    let start = Instant::now();
    let mut handles = vec![];
    let mut failed = 0;

    for _ in 0..config.concurrent_requests {
        let client = TestClient::new(app.clone());

        let handle = tokio::spawn(async move {
            let mut latencies = vec![];

            for _ in 0..config.requests_per_client {
                let req_start = Instant::now();
                let response = client.get("/v1/roadmaps").await;
                let latency = req_start.elapsed();

                if response.status == StatusCode::OK {
                    latencies.push(latency);
                }
            }

            latencies
        });

        handles.push(handle);
    }

    let mut all_latencies = vec![];
    for handle in handles {
        match handle.await {
            Ok(latencies) => all_latencies.extend(latencies),
            Err(_) => failed += config.requests_per_client,
        }
    }

    let total_duration = start.elapsed();
    let results = LoadTestResults::new(all_latencies, total_duration, failed);
    results.print("Get Roadmaps");

    assert!(
        results.avg_latency_ms <= config.acceptable_avg_latency_ms,
        "Average latency {} ms exceeds acceptable {} ms",
        results.avg_latency_ms,
        config.acceptable_avg_latency_ms
    );

    assert!(
        results.throughput_rps >= 100.0,
        "Throughput {:.2} rps is below acceptable 100 rps",
        results.throughput_rps
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn load_test_practice_review_submission() {
    let config = LoadTestConfig {
        concurrent_requests: 10,
        requests_per_client: 20,
        acceptable_avg_latency_ms: 100,
        acceptable_p95_latency_ms: 250,
    };

    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Create test data
    let deck_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO decks (id, title, description, language_from, language_to, created_at)
        VALUES ($1, 'Load Test Deck', 'Deck for load testing', 'en', 'es', NOW())
        "#,
    )
    .bind(deck_id)
    .execute(&state.pool)
    .await
    .expect("Failed to create deck");

    let flashcard_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO flashcards (id, term, translation, language_from, language_to, created_at)
        VALUES ($1, 'hello', 'hola', 'en', 'es', NOW())
        "#,
    )
    .bind(flashcard_id)
    .execute(&state.pool)
    .await
    .expect("Failed to create flashcard");

    sqlx::query(
        r#"
        INSERT INTO deck_flashcards (deck_id, flashcard_id)
        VALUES ($1, $2)
        "#,
    )
    .bind(deck_id)
    .bind(flashcard_id)
    .execute(&state.pool)
    .await
    .expect("Failed to link flashcard");

    // Create users
    let mut user_ids = vec![];
    for i in 0..config.concurrent_requests {
        let user_id = common::db::create_verified_user(
            &state.pool,
            &format!("practice{}@example.com", i),
            &format!("practiceuser{}", i),
        )
        .await
        .expect("Failed to create user");
        user_ids.push(user_id);
    }

    let app = router::router().with_state(state.clone());

    let start = Instant::now();
    let mut handles = vec![];
    let mut failed = 0;

    for (i, user_id) in user_ids.iter().enumerate() {
        let client = TestClient::new(app.clone());
        let user_id = *user_id;
        let deck_id = deck_id;
        let flashcard_id = flashcard_id;
        let jwt_secret = state.jwt_secret.clone();
        let cookie_key = state.cookie_key.clone();

        let handle = tokio::spawn(async move {
            let mut latencies = vec![];

            let token = common::jwt::create_test_token(
                user_id,
                &format!("practice{}@example.com", i),
                &jwt_secret,
            );

            for _ in 0..config.requests_per_client {
                let body = json!({
                    "correct": true,
                    "next_review_at": "2025-12-01T10:00:00Z",
                    "deck_id": deck_id.to_string()
                });

                let req_start = Instant::now();
                let response = client
                    .post_json_with_auth(
                        &format!("/v1/practice/{}/{}/review", user_id, flashcard_id),
                        &body,
                        &token,
                        &cookie_key,
                    )
                    .await;
                let latency = req_start.elapsed();

                if response.status == StatusCode::OK {
                    latencies.push(latency);
                }
            }

            latencies
        });

        handles.push(handle);
    }

    let mut all_latencies = vec![];
    for handle in handles {
        match handle.await {
            Ok(latencies) => all_latencies.extend(latencies),
            Err(_) => failed += config.requests_per_client,
        }
    }

    let total_duration = start.elapsed();
    let results = LoadTestResults::new(all_latencies, total_duration, failed);
    results.print("Practice Review Submission");

    assert!(
        results.avg_latency_ms <= config.acceptable_avg_latency_ms,
        "Average latency {} ms exceeds acceptable {} ms",
        results.avg_latency_ms,
        config.acceptable_avg_latency_ms
    );

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup");
}

#[tokio::test]
#[ignore]
async fn stress_test_database_connections() {
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    // Test with many concurrent database operations
    let concurrent_tasks = 100;
    let mut handles = vec![];

    let start = Instant::now();

    for _i in 0..concurrent_tasks {
        let pool = state.pool.clone();

        let handle = tokio::spawn(async move {
            // Simulate concurrent database operations
            let result: Result<i64, sqlx::Error> = sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(&pool)
                .await;

            result.is_ok()
        });

        handles.push(handle);
    }

    let mut successful = 0;
    for handle in handles {
        if let Ok(true) = handle.await {
            successful += 1;
        }
    }

    let duration = start.elapsed();

    println!("\n========== Database Stress Test ==========");
    println!("Concurrent queries:  {}", concurrent_tasks);
    println!("Successful:          {}", successful);
    println!("Failed:              {}", concurrent_tasks - successful);
    println!("Duration:            {:?}", duration);
    println!("==========================================\n");

    // All queries should succeed (pool should handle concurrent connections)
    assert_eq!(
        successful, concurrent_tasks,
        "All database queries should succeed"
    );
}

#[tokio::test]
#[ignore]
async fn performance_test_bcrypt_hashing() {
    // Test bcrypt performance (important for login/registration)
    let iterations = 10;
    let password = "TestPassword123!";

    let start = Instant::now();
    for _ in 0..iterations {
        bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash");
    }
    let duration = start.elapsed();

    let avg_ms = duration.as_millis() / iterations;

    println!("\n========== Bcrypt Performance ==========");
    println!("Iterations:          {}", iterations);
    println!("Average time:        {} ms", avg_ms);
    println!("Total time:          {:?}", duration);
    println!("=========================================\n");

    // Bcrypt should not be too slow (>500ms would impact UX)
    assert!(avg_ms < 500, "Bcrypt hashing is too slow: {} ms", avg_ms);
}
