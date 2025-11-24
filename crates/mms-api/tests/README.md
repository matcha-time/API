# Testing Guide for Matcha Time API

This guide covers how to run and write tests for the Matcha Time API.

## Table of Contents

- [Test Overview](#test-overview)
- [Setup](#setup)
- [Running Tests](#running-tests)
- [Test Structure](#test-structure)
- [Writing Tests](#writing-tests)
- [Continuous Integration](#continuous-integration)

## Test Overview

The API has comprehensive test coverage including:

- **Integration Tests**: Test complete HTTP request/response cycles with real database
- **Unit Tests**: Test individual functions and modules in isolation

### Test Coverage

#### Integration Tests

- **Auth Tests** (`auth_tests.rs`)
  - Health check endpoint
  - `/auth/me` with valid/invalid/expired tokens
  - `/auth/logout` with refresh token cleanup
  - `/auth/google` OAuth redirect

- **User Tests** (`user_tests.rs`)
  - User registration (success, duplicate email, invalid input)
  - User login (success, invalid credentials, unverified email)
  - User dashboard retrieval
  - User profile updates
  - User deletion
  - User stats creation

#### Unit Tests

- **JWT Tests** ([../src/auth/jwt.rs](../src/auth/jwt.rs))
  - Token generation and verification
  - Invalid token handling
  - Token expiration
  - Auth cookie creation (development/production)
  - OIDC flow cookie creation
  - Claims serialization

- **Validation Tests** ([../src/auth/validation.rs](../src/auth/validation.rs))
  - Email validation
  - Password strength validation
  - Username validation

## Setup

### Prerequisites

1. **Docker & Docker Compose** - For running the test database
2. **Rust toolchain** - `rustc 1.90.0` or later
3. **PostgreSQL client** (optional) - For manual database inspection

### 1. Start the Test Database

The test database runs in a separate Docker container on port 5433 (to avoid conflicts with development database on 5432).

```bash
# From the project root
cd ../../../

# Start the test database
docker compose -f compose.test.yaml up -d

# Verify the database is running
docker compose -f compose.test.yaml ps

# View logs if needed
docker compose -f compose.test.yaml logs -f postgres-test
```

The test database configuration:

- **Host**: localhost
- **Port**: 5433
- **Database**: matcha_time_test
- **User**: test_user
- **Password**: test_password
- **Connection String**: `postgres://test_user:test_password@localhost:5433/matcha_time_test`

### 2. Set Environment Variables

The tests use a default test database URL, but you can override it:

```bash
export TEST_DATABASE_URL="postgres://test_user:test_password@localhost:5433/matcha_time_test"
```

### 3. Run Database Migrations

Migrations are automatically run when tests start, but you can run them manually if needed:

```bash
# From the project root
cd crates/mms-db
cargo run --bin migrate
```

## Running Tests

### Run All Tests

```bash
# From the project root
cargo test

# Run with output visible
cargo test -- --nocapture

# Run tests in parallel (default)
cargo test -- --test-threads=4
```

### Run Specific Test Suites

```bash
# Run only integration tests
cargo test --test '*'

# Run only auth integration tests
cargo test --test auth_tests

# Run only user integration tests
cargo test --test user_tests

# Run only unit tests (embedded in source files)
cargo test --lib
```

### Run Specific Tests

```bash
# Run a specific test by name
cargo test test_user_registration_success

# Run tests matching a pattern
cargo test user_login

# Run tests in a specific module
cargo test auth::jwt::tests
```

### Run Tests with Logging

```bash
# Show println! output and logs
cargo test -- --nocapture

# Show only failing test output
cargo test -- --show-output
```

### Run Tests Sequentially

Integration tests share a test database and may have race conditions when run in parallel. For deterministic results, run integration tests sequentially:

```bash
# From the crates/mms-api directory:
cd crates/mms-api
cargo test auth_tests -- --test-threads=1
cargo test user_tests -- --test-threads=1

# Or from the workspace root:
cargo test --package mms-api --test auth_tests -- --test-threads=1
cargo test --package mms-api --test user_tests -- --test-threads=1

# Run all tests sequentially
cargo test -- --test-threads=1
```

**Note**: Sequential execution takes longer but ensures reliable test results.

## Test Structure

```bash
crates/mms-api/
├── src/
│   ├── auth/
│   │   ├── jwt.rs              # JWT unit tests (embedded)
│   │   └── validation.rs       # Validation unit tests (embedded)
│   └── ...
└── tests/
    ├── common/
    │   └── mod.rs              # Shared test utilities
    ├── auth_tests.rs           # Auth integration tests
    ├── user_tests.rs           # User integration tests
    └── README.md               # This file
```

### Test Utilities (`common/mod.rs`)

The common module provides helpful utilities:

#### `TestStateBuilder`

Builds test ApiState with mock configuration:

```rust
let state = TestStateBuilder::new()
    .with_database_url("custom_url".to_string())
    .with_jwt_secret("custom_secret".to_string())
    .build()
    .await?;
```

#### `TestClient`

Makes HTTP requests to test routes:

```rust
let client = TestClient::new(router);

// GET request
let response = client.get("/health").await;

// POST request with JSON
let body = json!({"email": "test@example.com"});
let response = client.post_json("/users/register", &body).await;

// Authenticated request
let response = client.with_auth_cookie(request, &token).await;
```

#### `TestResponse`

Provides response assertions and parsing:

```rust
response.assert_status(StatusCode::OK);
let body: serde_json::Value = response.json();
let text = response.text();
let cookie = response.get_cookie("auth_token");
```

#### Database Helpers (`common::db`)

```rust
// Clean up all test data
common::db::cleanup(&pool).await?;

// Create test users
let user_id = common::db::create_verified_user(&pool, "test@example.com", "testuser").await?;
let user_id = common::db::create_test_user(&pool, "test@example.com", "testuser", &password_hash).await?;

// Query users
let user_id = common::db::get_user_by_email(&pool, "test@example.com").await?;
```

#### JWT Helpers (`common::jwt`)

```rust
let token = common::jwt::create_test_token(user_id, "test@example.com", &jwt_secret);
```

## Writing Tests

### Integration Test Template

Create a new file in `tests/`:

```rust
mod common;

use axum::http::StatusCode;
use common::{TestClient, TestStateBuilder};
use mms_api::router;
use serde_json::json;

#[tokio::test]
async fn test_my_endpoint() {
    // Setup
    let state = TestStateBuilder::new()
        .build()
        .await
        .expect("Failed to create test state");

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Execute
    let body = json!({"key": "value"});
    let response = client.post_json("/my/endpoint", &body).await;

    // Assert
    response.assert_status(StatusCode::OK);
    let json: serde_json::Value = response.json();
    assert_eq!(json["key"], "expected_value");

    // Cleanup
    common::db::cleanup(&state.pool)
        .await
        .expect("Failed to cleanup database");
}
```

### Unit Test Template

Add to existing source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        let result = my_function(input);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

### Best Practices

1. **Database Cleanup**: Always cleanup test data after each test
2. **Isolation**: Each test should be independent and not rely on others
3. **Descriptive Names**: Use clear test names like `test_user_registration_with_duplicate_email`
4. **Arrange-Act-Assert**: Structure tests with clear setup, execution, and assertion phases
5. **Error Messages**: Use descriptive assertion messages for debugging
6. **Test Both Success and Failure**: Cover happy paths and error cases

### Testing Authenticated Endpoints

```rust
#[tokio::test]
async fn test_protected_endpoint() {
    let state = TestStateBuilder::new().build().await?;

    // Create a test user
    let user_id = common::db::create_verified_user(
        &state.pool,
        "test@example.com",
        "testuser"
    ).await?;

    // Generate auth token
    let token = common::jwt::create_test_token(
        user_id,
        "test@example.com",
        &state.jwt_secret
    );

    let app = router::router().with_state(state.clone());
    let client = TestClient::new(app);

    // Make authenticated request
    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/protected/endpoint")
        .header("cookie", format!("auth_token={}", token))
        .body(axum::body::Body::empty())?;

    let response = client.with_auth_cookie(request, &token).await;
    response.assert_status(StatusCode::OK);

    // Cleanup
    common::db::cleanup(&state.pool).await?;
}
```

## Continuous Integration

### Running Tests in CI

Example GitHub Actions workflow:

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:17-alpine
        env:
          POSTGRES_USER: test_user
          POSTGRES_PASSWORD: test_password
          POSTGRES_DB: matcha_time_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5433:5432

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Run tests
        env:
          TEST_DATABASE_URL: postgres://test_user:test_password@localhost:5433/matcha_time_test
        run: cargo test --verbose
```

### Test Coverage %

To generate test coverage reports:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# Open coverage report
open coverage/index.html
```

## Troubleshooting

### Test Database Connection Issues

```bash
# Check if database is running
docker compose -f compose.test.yaml ps

# Check database logs
docker compose -f compose.test.yaml logs postgres-test

# Restart database
docker compose -f compose.test.yaml restart

# Stop and remove database (fresh start)
docker compose -f compose.test.yaml down -v
docker compose -f compose.test.yaml up -d
```

### Migration Issues

```bash
# Check migration status
cd crates/mms-db
sqlx migrate info --database-url "postgres://test_user:test_password@localhost:5433/matcha_time_test"

# Revert all migrations
sqlx migrate revert --database-url "postgres://test_user:test_password@localhost:5433/matcha_time_test"

# Run migrations manually
sqlx migrate run --database-url "postgres://test_user:test_password@localhost:5433/matcha_time_test"
```

### Tests Hanging

If tests hang, it's usually due to database connections not being closed:

```bash
# Kill all tests
pkill -f cargo

# Stop test database
docker compose -f compose.test.yaml down

# Start fresh
docker compose -f compose.test.yaml up -d
cargo test
```

### Port Conflicts

If port 5433 is already in use:

```bash
# Find process using port 5433
lsof -i :5433

# Kill the process
kill -9 <PID>

# Or change the port in compose.test.yaml
```

## Next Steps

- Add more integration tests for remaining endpoints:
  - Email verification flow
  - Password reset flow
  - Roadmap endpoints
  - Deck endpoints
  - Practice endpoints
- Add performance tests for critical paths
- Add stress tests for rate limiting
- Set up continuous integration with GitHub Actions
- Add test coverage tracking

## Resources

- [Axum Testing Guide](https://docs.rs/axum/latest/axum/index.html#testing)
- [Tower Service Testing](https://docs.rs/tower/latest/tower/trait.Service.html)
- [SQLx Testing](https://github.com/launchbadge/sqlx#testing)
- [Rust Testing Book](https://doc.rust-lang.org/book/ch11-00-testing.html)
