# Integration and Load Tests for Matcha Time API

This directory contains comprehensive integration, security, and load tests for the Matcha Time API.

## Test Files

1. **`auth_tests.rs`** - Authentication flows (OAuth, login, logout, JWT)
2. **`email_verification_tests.rs`** - Email verification flow (9 tests)
3. **`password_reset_tests.rs`** - Password reset flow (8 tests)
4. **`refresh_token_tests.rs`** - Refresh token rotation (9 tests)
5. **`roadmap_deck_practice_tests.rs`** - Core features: roadmaps, decks, practice with answer validation (11 tests)
6. **`user_tests.rs`** - User registration, dashboard, profile management
7. **`rate_limit_tests.rs`** - Rate limiting (8 tests)
8. **`security_tests.rs`** - Security vulnerabilities (15 tests)
9. **`load_tests.rs`** - Performance tests (6 tests, ignored by default)

Total: 88 integration tests + 6 load tests

## Running Tests

```bash
# Run all integration tests
cargo test --package mms-api --test integration -- --test-threads=1

# Run specific test file
cargo test --package mms-api --test integration email_verification_tests -- --test-threads=1

# Run load tests (ignored by default)
cargo test --package mms-api --test integration --ignored -- --test-threads=1

# Run with output
cargo test --package mms-api --test integration -- --test-threads=1 --nocapture
```

## Test Coverage Summary

### ✅ Email Verification (9 tests)

- Full verification flow
- Expired/used/invalid tokens
- Resend functionality
- Email enumeration prevention

### ✅ Password Reset (8 tests)

- Full reset flow
- Token security
- Session revocation
- Enumeration prevention

### ✅ Refresh Tokens (9 tests)

- Token rotation
- Reuse detection
- Multi-device support
- Breach detection

### ✅ Core Features (11 tests)

- Roadmap endpoints (list, filter by language, nodes, progress)
- Deck practice sessions
- Review submissions with answer validation (correct/wrong answers)
- Flashcard-to-deck authorization check
- Progress and stats tracking after reviews

### ✅ Rate Limiting (8 tests)

- Different endpoint tiers
- Timing-safe middleware
- Per-IP tracking
- Brute-force protection

### ✅ Security (15 tests)

- SQL injection protection
- XSS prevention
- Auth bypass attempts
- IDOR protection
- Path traversal
- Input validation

### ⏱️ Load Tests (6 tests)

- Registration load (10x10 requests)
- Login load (20x50 requests)
- Roadmap reads (50x100 requests)
- Practice submissions
- Database stress
- Bcrypt performance

## Test Helpers Available

The tests include helpful utility functions in `common/mod.rs`:

```rust
// Create test verification tokens
common::verification::create_test_verification_token(pool, user_id)

// Create test password reset tokens
common::verification::create_test_password_reset_token(pool, user_id)

// Create test JWT tokens
common::jwt::create_test_token(user_id, email, jwt_secret)

// Database helpers
common::db::create_test_user(pool, email, username, password_hash)
common::db::create_verified_user(pool, email, username)
common::db::delete_user_by_email(pool, email)
```

## Test Configuration

- Test DB: `postgres://test_user:test_password@localhost:5433/matcha_time_test`
- Sequential: Use `--test-threads=1` for guaranteed pass
- Each test uses unique data (UUIDs) to minimize conflicts
- All 88 tests pass (6 load tests ignored by default)
