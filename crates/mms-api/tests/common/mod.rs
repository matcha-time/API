use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use axum_extra::extract::cookie::Key;
use http_body_util::BodyExt;
use mms_api::{config::Environment, state::ApiState};
use oauth2::{ClientId, ClientSecret, RedirectUrl};
use openidconnect::{IssuerUrl, core::{CoreClient, CoreProviderMetadata}};
use serde::Deserialize;
use sqlx::PgPool;
use tower::ServiceExt;

/// Test configuration
pub struct TestConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub cookie_secret: String,
    pub frontend_url: String,
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

    pub fn with_database_url(mut self, url: String) -> Self {
        self.config.database_url = url;
        self
    }

    pub fn with_jwt_secret(mut self, secret: String) -> Self {
        self.config.jwt_secret = secret;
        self
    }

    /// Build a test ApiState with a real database connection
    pub async fn build(self) -> anyhow::Result<ApiState> {
        // Create database pool
        let pool = mms_db::create_pool(&self.config.database_url).await?;

        // Run migrations
        mms_db::ensure_db_and_migrate(&self.config.database_url, &pool).await?;

        // Create a mock OIDC client (won't be used in most tests)
        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new("https://accounts.google.com".to_string())?,
            &reqwest::Client::new(),
        )
        .await?;

        let oidc_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new("test_client_id".to_string()),
            Some(ClientSecret::new("test_client_secret".to_string())),
        )
        .set_redirect_uri(RedirectUrl::new(
            "http://localhost:3000/auth/callback".to_string(),
        )?);

        // Create cookie key
        let cookie_key = Key::from(self.config.cookie_secret.as_bytes());

        Ok(ApiState {
            oidc_client,
            jwt_secret: self.config.jwt_secret,
            frontend_url: self.config.frontend_url,
            cookie_key,
            pool,
            environment: Environment::Development,
            email_service: None, // No email service in tests
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
    pub async fn request(&self, request: Request<Body>) -> TestResponse {
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
            .body(Body::from(json_body))
            .expect("Failed to build request");

        self.request(request).await
    }

    /// Send a PATCH request with JSON body
    pub async fn patch_json<T: serde::Serialize>(&self, uri: &str, body: &T) -> TestResponse {
        let json_body = serde_json::to_string(body).expect("Failed to serialize body");

        let request = Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(json_body))
            .expect("Failed to build request");

        self.request(request).await
    }

    /// Send a PATCH request with JSON body and authentication cookie
    pub async fn patch_json_with_auth<T: serde::Serialize>(&self, uri: &str, body: &T, token: &str, cookie_key: &Key) -> TestResponse {
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
            .header("cookie", format!("{}={}", encrypted.name(), encrypted.value()))
            .body(Body::from(json_body))
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a DELETE request
    pub async fn delete(&self, uri: &str) -> TestResponse {
        let request = Request::builder()
            .method("DELETE")
            .uri(uri)
            .body(Body::empty())
            .expect("Failed to build request");

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
            .header("cookie", format!("{}={}", encrypted.name(), encrypted.value()))
            .body(Body::empty())
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a GET request with authentication cookie
    /// Note: This uses encrypted cookies via PrivateCookieJar
    pub async fn get_with_auth(&self, uri: &str, token: &str, cookie_key: &Key) -> TestResponse {
        use axum_extra::extract::cookie::Cookie;
        use axum_extra::extract::PrivateCookieJar;

        // Create a cookie with owned string using Cookie::build
        let cookie = Cookie::build(("auth_token", token.to_string())).build();

        // Create an empty jar and add the cookie (this encrypts it)
        let empty_jar = PrivateCookieJar::<Key>::new(cookie_key.clone());
        let jar_with_cookie = empty_jar.add(cookie);

        // We need to extract the Set-Cookie header value from the jar
        // The way to do this in axum is to use the IntoResponseParts trait
        // But for testing, we can manually encrypt using the cookie crate's private jar

        // Actually, let's use a different approach: create a CookieJar and manually encrypt
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();
        let raw_cookie = cookie::Cookie::new("auth_token", token.to_string());
        raw_jar.private_mut(&raw_key).add(raw_cookie);

        // Get the encrypted cookie
        let encrypted = raw_jar.get("auth_token").expect("Cookie should exist");

        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .header("cookie", format!("{}={}", encrypted.name(), encrypted.value()))
            .body(Body::empty())
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a GET request with both auth and refresh token cookies
    pub async fn get_with_auth_and_refresh(&self, uri: &str, auth_token: &str, refresh_token: &str, cookie_key: &Key) -> TestResponse {
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
        let encrypted_refresh = raw_jar.get("refresh_token").expect("Refresh cookie should exist");

        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .header("cookie", format!("{}={}; {}={}",
                encrypted_auth.name(), encrypted_auth.value(),
                encrypted_refresh.name(), encrypted_refresh.value()))
            .body(Body::empty())
            .expect("Failed to build authenticated request");

        self.request(request).await
    }

    /// Send a request with authentication cookie
    pub async fn with_auth_cookie(&self, mut request: Request<Body>, token: &str, cookie_key: &Key) -> TestResponse {
        use cookie::{CookieJar as RawCookieJar, Key as RawKey};

        let raw_key = RawKey::try_from(cookie_key.master()).expect("Invalid key");
        let mut raw_jar = RawCookieJar::new();
        let raw_cookie = cookie::Cookie::new("auth_token", token.to_string());
        raw_jar.private_mut(&raw_key).add(raw_cookie);

        // Get the encrypted cookie
        let encrypted = raw_jar.get("auth_token").expect("Cookie should exist");

        request
            .headers_mut()
            .insert("cookie", format!("{}={}", encrypted.name(), encrypted.value()).parse().unwrap());

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

    /// Setup test database - cleanup before running tests
    /// Call this at the start of each test to ensure a clean state
    pub async fn setup(pool: &PgPool) -> anyhow::Result<()> {
        cleanup(pool).await
    }

    /// Clean up test database - delete all data from tables
    pub async fn cleanup(pool: &PgPool) -> anyhow::Result<()> {
        sqlx::query!("DELETE FROM user_card_progress")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM user_deck_progress")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM user_activity")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM deck_flashcards")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM flashcards")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM decks").execute(pool).await?;
        sqlx::query!("DELETE FROM roadmap_nodes")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM roadmaps")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM refresh_tokens")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM email_verification_tokens")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM password_reset_tokens")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM user_stats")
            .execute(pool)
            .await?;
        sqlx::query!("DELETE FROM users").execute(pool).await?;

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

        sqlx::query!(
            r#"
            INSERT INTO users (id, email, username, password_hash, auth_provider, email_verified, created_at)
            VALUES ($1, $2, $3, $4, 'email', true, NOW())
            "#,
            user_id,
            email,
            username,
            password_hash
        )
        .execute(pool)
        .await?;

        // Create user_stats entry
        sqlx::query!(
            r#"
            INSERT INTO user_stats (user_id)
            VALUES ($1)
            "#,
            user_id
        )
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
        let result = sqlx::query!(
            r#"
            SELECT id FROM users WHERE email = $1
            "#,
            email
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| r.id))
    }

    /// Delete a specific user by email (for test cleanup)
    /// This will cascade delete related records due to foreign key constraints
    pub async fn delete_user_by_email(pool: &PgPool, email: &str) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM users WHERE email = $1
            "#,
            email
        )
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
        generate_jwt_token(user_id, email.to_string(), jwt_secret)
            .expect("Failed to generate test JWT token")
    }
}
