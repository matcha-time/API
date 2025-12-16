use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use axum_extra::extract::cookie::Key;
use http_body_util::BodyExt;
use mms_api::{config::Environment, state::ApiState};
use serde::Deserialize;
use tower::ServiceExt;

/// Test configuration
pub struct TestConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub cookie_secret: String,
    pub frontend_url: String,
    pub jwt_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
    pub oidc_flow_expiry_minutes: i64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            database_url: std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
                "postgres://test_user:test_password@localhost:5433/matcha_time_test".to_string()
            }),
            jwt_secret: "test_jwt_secret_minimum_32_characters_long".to_string(),
            cookie_secret: "test_cookie_secret_minimum_64_characters_long_for_secure_encryption"
                .to_string(),
            frontend_url: "http://localhost:8080".to_string(),
            jwt_expiry_hours: 24,
            refresh_token_expiry_days: 30,
            oidc_flow_expiry_minutes: 10,
        }
    }
}

/// Test state builder for creating mock ApiState
pub struct TestStateBuilder {
    config: TestConfig,
}

impl TestStateBuilder {
    pub fn new() -> Self {
        Self {
            config: TestConfig::default(),
        }
    }

    /// Build a test ApiState with a real database connection
    pub async fn build(self) -> anyhow::Result<ApiState> {
        // Create database pool with default max_connections for tests
        let pool = mms_db::create_pool(&self.config.database_url, 10).await?;

        // Run migrations
        mms_db::ensure_db_and_migrate(&self.config.database_url, &pool).await?;

        // Create a mock OIDC client using the google module
        let oidc_client = mms_api::auth::google::create_oidc_client(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "http://localhost:3000/auth/callback".to_string(),
        )
        .await?;

        // Create cookie key
        let cookie_key = Key::from(self.config.cookie_secret.as_bytes());

        Ok(ApiState {
            oidc_client,
            jwt_secret: self.config.jwt_secret,
            jwt_expiry_hours: self.config.jwt_expiry_hours,
            refresh_token_expiry_days: self.config.refresh_token_expiry_days,
            oidc_flow_expiry_minutes: self.config.oidc_flow_expiry_minutes,
            frontend_url: self.config.frontend_url,
            cookie_key,
            pool,
            environment: Environment::Development,
            email_service: None, // No email service in tests
            cookie_domain: "localhost".to_string(),
            bcrypt_cost: 8,
        })
    }
}

impl Default for TestStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to make requests to the test app
pub struct TestClient {
    router: Router,
}

impl TestClient {
    pub fn new(router: Router) -> Self {
        Self { router }
    }

    /// Send a request and get the response
    pub async fn request(&self, mut request: Request<Body>) -> TestResponse {
        // Add ConnectInfo extension for rate limiting to work in tests
        use axum::extract::ConnectInfo;
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        let test_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        request.extensions_mut().insert(ConnectInfo(test_addr));

        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request");

        let status = response.status();
        let headers = response.headers().clone();
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("Failed to read response body")
            .to_bytes();

        TestResponse {
            status,
            body: body_bytes.to_vec(),
            headers,
        }
    }

    /// Send a GET request
    pub async fn get(&self, uri: &str) -> TestResponse {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .body(Body::empty())
            .expect("Failed to build request");

        self.request(request).await
    }

    /// Send a POST request with no body
    pub async fn post(&self, uri: &str) -> TestResponse {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .body(Body::empty())
            .expect("Failed to build request");

        self.request(request).await
    }

    /// Send a POST request with JSON body
    pub async fn post_json<T: serde::Serialize>(&self, uri: &str, body: &T) -> TestResponse {
        let json_body = serde_json::to_string(body).expect("Failed to serialize body");

        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .body(Body::from(json_body))
            .expect("Failed to build request");

        self.request(request).await
    }

    /// Send a PATCH request with JSON body and authentication cookie
    pub async fn patch_json_with_auth<T: serde::Serialize>(
        &self,
        uri: &str,
        body: &T,
        token: &str,
        cookie_key: &Key,
    ) -> TestResponse {
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();
        let raw_cookie = cookie::Cookie::new("auth_token", token.to_string());
        raw_jar.private_mut(&raw_key).add(raw_cookie);

        let encrypted = raw_jar.get("auth_token").expect("Cookie should exist");
        let json_body = serde_json::to_string(body).expect("Failed to serialize body");

        let request = Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("content-type", "application/json")
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .header(
                "cookie",
                format!("{}={}", encrypted.name(), encrypted.value()),
            )
            .body(Body::from(json_body))
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a DELETE request with authentication cookie
    pub async fn delete_with_auth(&self, uri: &str, token: &str, cookie_key: &Key) -> TestResponse {
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();
        let raw_cookie = cookie::Cookie::new("auth_token", token.to_string());
        raw_jar.private_mut(&raw_key).add(raw_cookie);

        let encrypted = raw_jar.get("auth_token").expect("Cookie should exist");

        let request = Request::builder()
            .method("DELETE")
            .uri(uri)
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .header(
                "cookie",
                format!("{}={}", encrypted.name(), encrypted.value()),
            )
            .body(Body::empty())
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a GET request with authentication cookie
    pub async fn get_with_auth(&self, uri: &str, token: &str, cookie_key: &Key) -> TestResponse {
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();
        let raw_cookie = cookie::Cookie::new("auth_token", token.to_string());
        raw_jar.private_mut(&raw_key).add(raw_cookie);

        let encrypted = raw_jar.get("auth_token").expect("Cookie should exist");

        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .header(
                "cookie",
                format!("{}={}", encrypted.name(), encrypted.value()),
            )
            .body(Body::empty())
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a POST request with authentication cookie (no body)
    pub async fn post_with_auth(&self, uri: &str, token: &str, cookie_key: &Key) -> TestResponse {
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();
        let raw_cookie = cookie::Cookie::new("auth_token", token.to_string());
        raw_jar.private_mut(&raw_key).add(raw_cookie);

        let encrypted = raw_jar.get("auth_token").expect("Cookie should exist");

        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .header(
                "cookie",
                format!("{}={}", encrypted.name(), encrypted.value()),
            )
            .body(Body::empty())
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a POST request with both auth and refresh token cookies (no body)
    pub async fn post_with_auth_and_refresh(
        &self,
        uri: &str,
        auth_token: &str,
        refresh_token: &str,
        cookie_key: &Key,
    ) -> TestResponse {
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();

        // Add auth token
        let auth_cookie = cookie::Cookie::new("auth_token", auth_token.to_string());
        raw_jar.private_mut(&raw_key).add(auth_cookie);

        // Add refresh token
        let refresh_cookie = cookie::Cookie::new("refresh_token", refresh_token.to_string());
        raw_jar.private_mut(&raw_key).add(refresh_cookie);

        // Get both encrypted cookies
        let encrypted_auth = raw_jar.get("auth_token").expect("Auth cookie should exist");
        let encrypted_refresh = raw_jar
            .get("refresh_token")
            .expect("Refresh cookie should exist");

        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .header(
                "cookie",
                format!(
                    "{}={}; {}={}",
                    encrypted_auth.name(),
                    encrypted_auth.value(),
                    encrypted_refresh.name(),
                    encrypted_refresh.value()
                ),
            )
            .body(Body::empty())
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a POST request with JSON body and authentication cookie
    pub async fn post_json_with_auth<T: serde::Serialize>(
        &self,
        uri: &str,
        body: &T,
        token: &str,
        cookie_key: &Key,
    ) -> TestResponse {
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();
        let raw_cookie = cookie::Cookie::new("auth_token", token.to_string());
        raw_jar.private_mut(&raw_key).add(raw_cookie);

        let encrypted = raw_jar.get("auth_token").expect("Cookie should exist");
        let json_body = serde_json::to_string(body).expect("Failed to serialize body");

        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("x-forwarded-for", "127.0.0.1") // Required for rate limiting in tests
            .header(
                "cookie",
                format!("{}={}", encrypted.name(), encrypted.value()),
            )
            .body(Body::from(json_body))
            .expect("Failed to build authenticated request");

        self.request(request).await
    }
}

/// Test response wrapper
pub struct TestResponse {
    pub status: StatusCode,
    pub body: Vec<u8>,
    pub headers: axum::http::HeaderMap,
}

impl TestResponse {
    /// Get response body as string
    pub fn text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("Response body is not valid UTF-8")
    }

    /// Parse response body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> T {
        serde_json::from_slice(&self.body).expect("Failed to parse JSON response")
    }

    /// Assert status code
    pub fn assert_status(&self, expected: StatusCode) {
        assert_eq!(
            self.status,
            expected,
            "Expected status {}, got {}. Body: {}",
            expected,
            self.status,
            self.text()
        );
    }

    /// Extract cookie value by name
    pub fn get_cookie(&self, name: &str) -> Option<String> {
        // Use get_all to handle multiple Set-Cookie headers
        for value in self.headers.get_all("set-cookie").iter() {
            if let Ok(cookie_str) = value.to_str() {
                if cookie_str.starts_with(&format!("{}=", name)) {
                    let value = cookie_str.split(';').next()?.split('=').nth(1)?.to_string();
                    return Some(value);
                }
            }
        }
        None
    }
}

/// Database test helper functions
pub mod db {
    use sqlx::PgPool;
    use uuid::Uuid;

    /// Clean up test database - delete all data from tables
    /// Used for initial database setup before running tests
    pub async fn cleanup(pool: &PgPool) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM user_card_progress")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM user_deck_progress")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM user_activity")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM deck_flashcards")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM flashcards").execute(pool).await?;
        sqlx::query("DELETE FROM roadmap_nodes")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM decks").execute(pool).await?;
        sqlx::query("DELETE FROM roadmaps").execute(pool).await?;
        sqlx::query("DELETE FROM refresh_tokens")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM email_verification_tokens")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM password_reset_tokens")
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM user_stats").execute(pool).await?;
        sqlx::query("DELETE FROM users").execute(pool).await?;

        Ok(())
    }

    /// Create a test user and return the user_id
    pub async fn create_test_user(
        pool: &PgPool,
        email: &str,
        username: &str,
        password_hash: &str,
    ) -> anyhow::Result<Uuid> {
        let user_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO users (id, email, username, password_hash, auth_provider, email_verified, created_at)
            VALUES ($1, $2, $3, $4, 'email', true, NOW())
            "#,
        )
        .bind(user_id)
        .bind(email)
        .bind(username)
        .bind(password_hash)
        .execute(pool)
        .await?;

        // Create user_stats entry
        sqlx::query(
            r#"
            INSERT INTO user_stats (user_id)
            VALUES ($1)
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(user_id)
    }

    /// Create a test user with verified email
    pub async fn create_verified_user(
        pool: &PgPool,
        email: &str,
        username: &str,
    ) -> anyhow::Result<Uuid> {
        let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST)?;
        create_test_user(pool, email, username, &password_hash).await
    }

    /// Get user by email
    pub async fn get_user_by_email(pool: &PgPool, email: &str) -> anyhow::Result<Option<Uuid>> {
        let result: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM users WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| r.0))
    }

    /// Delete a specific user by email (for test cleanup)
    /// This will cascade delete related records due to foreign key constraints
    pub async fn delete_user_by_email(pool: &PgPool, email: &str) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM users WHERE email = $1
            "#,
        )
        .bind(email)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete a specific roadmap by ID (for test cleanup)
    /// This will cascade delete related records due to foreign key constraints
    pub async fn delete_roadmap_by_id(pool: &PgPool, roadmap_id: uuid::Uuid) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM roadmaps WHERE id = $1
            "#,
        )
        .bind(roadmap_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// JWT test helpers
pub mod jwt {
    use mms_api::auth::jwt::generate_jwt_token;
    use uuid::Uuid;

    /// Generate a test JWT token
    pub fn create_test_token(user_id: Uuid, email: &str, jwt_secret: &str) -> String {
        generate_jwt_token(user_id, email.to_string(), jwt_secret, 24)
            .expect("Failed to generate test JWT token")
    }
}

/// Test data helpers
pub mod test_data {
    /// Generate a unique email for test isolation
    /// Each test should use this to ensure no conflicts in concurrent execution
    pub fn unique_email(base: &str) -> String {
        let uuid = uuid::Uuid::new_v4();
        format!("{}+{}@example.com", base, &uuid.to_string()[..8])
    }

    /// Generate a unique username for test isolation
    pub fn unique_username(base: &str) -> String {
        let uuid = uuid::Uuid::new_v4();
        format!("{}_{}", base, &uuid.to_string()[..8])
    }
}

/// Email verification test helpers
pub mod verification {
    use sqlx::PgPool;
    use uuid::Uuid;

    /// Create an email verification token for testing
    /// Returns the plain token that can be used in verification URLs
    pub async fn create_test_verification_token(
        pool: &PgPool,
        user_id: Uuid,
    ) -> anyhow::Result<String> {
        // Use the actual implementation from the API
        mms_api::user::email_verification::create_verification_token(pool, user_id, 24)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create verification token: {}", e))
    }

    /// Create a password reset token for testing
    /// Returns the plain token that can be used in reset URLs
    pub async fn create_test_password_reset_token(
        pool: &PgPool,
        user_id: Uuid,
    ) -> anyhow::Result<String> {
        // Use the actual implementation from the API
        mms_api::user::password_reset::create_reset_token(pool, user_id, 1)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create password reset token: {}", e))
    }
}
