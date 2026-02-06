use serde::Deserialize;

/// Environment mode for the application
#[derive(Default, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    /// Default to production for safety
    #[default]
    Production,
}

impl Environment {
    /// Returns true if in development mode
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }

    /// Returns true if in production mode
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

/// Main application configuration
///
/// All environment variables are parsed and validated at application startup.
/// This ensures fail-fast behavior if configuration is invalid.
///
/// Environment variables are automatically deserialized using `envy`.
#[derive(Clone, Debug, Deserialize)]
pub struct ApiConfig {
    // OAuth & Authentication
    pub google_client_id: String,
    pub google_client_secret: String,
    pub redirect_url: String,

    // JWT & Security
    pub jwt_secret: String,
    pub cookie_secret: String,

    /// Bcrypt cost factor for password hashing (default: 10)
    /// Higher values are more secure but slower (each increment doubles the time)
    /// Recommended: 10 (fast, ~100ms), 11 (medium, ~200ms), 12 (secure, ~400ms)
    #[serde(default = "default_bcrypt_cost")]
    pub bcrypt_cost: u32,

    /// JWT token expiry in hours (default: 24)
    #[serde(default = "default_jwt_expiry_hours")]
    pub jwt_expiry_hours: i64,

    /// Refresh token expiry in days (default: 30)
    #[serde(default = "default_refresh_token_expiry_days")]
    pub refresh_token_expiry_days: i64,

    /// OIDC flow cookie expiry in minutes (default: 10)
    #[serde(default = "default_oidc_flow_expiry_minutes")]
    pub oidc_flow_expiry_minutes: i64,

    // Email / SMTP (optional)
    pub smtp_host: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from_email: Option<String>,
    pub smtp_from_name: Option<String>,

    // Database
    pub database_url: String,

    /// Maximum number of database connections in the pool (default: 10)
    #[serde(default = "default_database_max_connections")]
    pub database_max_connections: u32,

    // Server Configuration
    /// Port to run the server on (default: 3000)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Maximum request body size in bytes (default: 2MB)
    #[serde(default = "default_max_request_body_size")]
    pub max_request_body_size: usize,

    /// Request timeout in seconds (default: 30)
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u64,

    // Frontend & CORS
    pub frontend_url: String,

    /// Cookie domain for cross-subdomain cookie sharing
    /// - Development: "localhost"
    /// - Production: ".matcha-time.dev" (with leading dot for subdomains)
    pub cookie_domain: String,

    /// Comma-separated list of allowed origins for CORS
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: String,

    // Rate Limiting
    /// Number of requests allowed per second (default: 2)
    #[serde(default = "default_rate_limit_per_second")]
    pub rate_limit_per_second: u64,

    /// Maximum burst size for rate limiting (default: 100)
    #[serde(default = "default_rate_limit_burst_size")]
    pub rate_limit_burst_size: u32,

    /// Environment mode (development/production)
    #[serde(default)]
    pub env: Environment,
}

/// Default value for bcrypt cost (10 = ~100ms, good balance of security and speed)
fn default_bcrypt_cost() -> u32 {
    10
}

/// Default value for allowed_origins
fn default_allowed_origins() -> String {
    "http://localhost:8080".to_string()
}

/// Default value for rate_limit_per_second
fn default_rate_limit_per_second() -> u64 {
    2
}

/// Default value for rate_limit_burst_size
fn default_rate_limit_burst_size() -> u32 {
    100
}

/// Default value for database_max_connections
fn default_database_max_connections() -> u32 {
    10
}

/// Default value for port
fn default_port() -> u16 {
    3000
}

/// Default value for max_request_body_size (2MB)
fn default_max_request_body_size() -> usize {
    2 * 1024 * 1024 // 2MB in bytes
}

/// Default value for request_timeout_seconds
fn default_request_timeout_seconds() -> u64 {
    30
}

/// Default value for JWT expiry (24 hours)
fn default_jwt_expiry_hours() -> i64 {
    24
}

/// Default value for refresh token expiry (30 days)
fn default_refresh_token_expiry_days() -> i64 {
    30
}

/// Default value for OIDC flow cookie expiry (10 minutes)
fn default_oidc_flow_expiry_minutes() -> i64 {
    10
}

/// Custom error type for configuration
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration parse error: {0}")]
    ParseError(#[from] envy::Error),
    #[error("Configuration validation error: {0}")]
    ValidationError(String),
}

impl ApiConfig {
    /// Load and validate configuration from environment variables
    ///
    /// This method should be called once at application startup.
    /// It will fail fast if any required variables are missing or invalid.
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        let config: Self = envy::from_env()?;

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from a shuttle SecretStore (for Shuttle.rs deployment)
    #[cfg(feature = "shuttle")]
    pub fn from_shuttle_secrets(
        secrets: &shuttle_runtime::SecretStore,
    ) -> Result<Self, ConfigError> {
        let config = Self {
            google_client_id: secrets
                .get("GOOGLE_CLIENT_ID")
                .ok_or_else(|| {
                    ConfigError::ValidationError("GOOGLE_CLIENT_ID not found".to_string())
                })?
                .to_string(),
            google_client_secret: secrets
                .get("GOOGLE_CLIENT_SECRET")
                .ok_or_else(|| {
                    ConfigError::ValidationError("GOOGLE_CLIENT_SECRET not found".to_string())
                })?
                .to_string(),
            redirect_url: secrets
                .get("REDIRECT_URL")
                .ok_or_else(|| ConfigError::ValidationError("REDIRECT_URL not found".to_string()))?
                .to_string(),
            jwt_secret: secrets
                .get("JWT_SECRET")
                .ok_or_else(|| ConfigError::ValidationError("JWT_SECRET not found".to_string()))?
                .to_string(),
            cookie_secret: secrets
                .get("COOKIE_SECRET")
                .ok_or_else(|| ConfigError::ValidationError("COOKIE_SECRET not found".to_string()))?
                .to_string(),
            bcrypt_cost: secrets
                .get("BCRYPT_COST")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_bcrypt_cost),
            jwt_expiry_hours: secrets
                .get("JWT_EXPIRY_HOURS")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_jwt_expiry_hours),
            refresh_token_expiry_days: secrets
                .get("REFRESH_TOKEN_EXPIRY_DAYS")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_refresh_token_expiry_days),
            oidc_flow_expiry_minutes: secrets
                .get("OIDC_FLOW_EXPIRY_MINUTES")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_oidc_flow_expiry_minutes),
            smtp_host: secrets.get("SMTP_HOST").map(|s| s.to_string()),
            smtp_username: secrets.get("SMTP_USERNAME").map(|s| s.to_string()),
            smtp_password: secrets.get("SMTP_PASSWORD").map(|s| s.to_string()),
            smtp_from_email: secrets.get("SMTP_FROM_EMAIL").map(|s| s.to_string()),
            smtp_from_name: secrets.get("SMTP_FROM_NAME").map(|s| s.to_string()),
            // DATABASE_URL is not needed when using Shuttle's provided pool
            // Use a dummy value since the pool is passed directly
            database_url: secrets
                .get("DATABASE_URL")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "postgres://shuttle-provided".to_string()),
            database_max_connections: secrets
                .get("DATABASE_MAX_CONNECTIONS")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_database_max_connections),
            port: secrets
                .get("PORT")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_port),
            max_request_body_size: secrets
                .get("MAX_REQUEST_BODY_SIZE")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_request_body_size),
            request_timeout_seconds: secrets
                .get("REQUEST_TIMEOUT_SECONDS")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_request_timeout_seconds),
            frontend_url: secrets
                .get("FRONTEND_URL")
                .ok_or_else(|| ConfigError::ValidationError("FRONTEND_URL not found".to_string()))?
                .to_string(),
            cookie_domain: secrets
                .get("COOKIE_DOMAIN")
                .ok_or_else(|| ConfigError::ValidationError("COOKIE_DOMAIN not found".to_string()))?
                .to_string(),
            allowed_origins: secrets
                .get("ALLOWED_ORIGINS")
                .map(|s| s.to_string())
                .unwrap_or_else(default_allowed_origins),
            rate_limit_per_second: secrets
                .get("RATE_LIMIT_PER_SECOND")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_rate_limit_per_second),
            rate_limit_burst_size: secrets
                .get("RATE_LIMIT_BURST_SIZE")
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_rate_limit_burst_size),
            env: secrets
                .get("ENV")
                .map(|s| match s.to_lowercase().as_str() {
                    "development" => Environment::Development,
                    _ => Environment::Production,
                })
                .unwrap_or_default(),
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate JWT secret length and entropy
        if self.jwt_secret.len() < 32 {
            return Err(ConfigError::ValidationError(
                "JWT_SECRET must be at least 32 characters long for security".to_string(),
            ));
        }

        // Check for weak secrets (common patterns)
        if self
            .jwt_secret
            .chars()
            .all(|c| c == self.jwt_secret.chars().next().unwrap())
        {
            return Err(ConfigError::ValidationError(
                "JWT_SECRET appears to be a repeated character pattern. Use a cryptographically random secret.".to_string(),
            ));
        }

        // Check for basic entropy - ensure some variety in characters
        let unique_chars: std::collections::HashSet<char> = self.jwt_secret.chars().collect();
        if unique_chars.len() < 16 {
            return Err(ConfigError::ValidationError(
                "JWT_SECRET has insufficient entropy (too few unique characters). Use a cryptographically random secret with at least 16 unique characters.".to_string(),
            ));
        }

        // Validate cookie secret length
        if self.cookie_secret.len() < 64 {
            return Err(ConfigError::ValidationError(
                "COOKIE_SECRET must be at least 64 characters long for secure encryption"
                    .to_string(),
            ));
        }

        // Validate that allowed_origins is not empty
        if self.allowed_origins.trim().is_empty() {
            return Err(ConfigError::ValidationError(
                "ALLOWED_ORIGINS cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Parse allowed origins into a vector
    pub fn parsed_allowed_origins(&self) -> Vec<String> {
        self.allowed_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
