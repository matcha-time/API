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

    // Database
    pub database_url: String,

    // Frontend & CORS
    pub frontend_url: String,

    /// Comma-separated list of allowed origins for CORS
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: String,

    /// Environment mode (development/production)
    #[serde(default)]
    pub env: Environment,
}

/// Default value for allowed_origins
fn default_allowed_origins() -> String {
    "http://localhost:8080".to_string()
}

/// Custom error type for configuration
#[derive(Debug)]
pub enum ConfigError {
    ParseError(envy::Error),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ParseError(err) => write!(f, "Configuration parse error: {}", err),
            ConfigError::ValidationError(msg) => {
                write!(f, "Configuration validation error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<envy::Error> for ConfigError {
    fn from(err: envy::Error) -> Self {
        ConfigError::ParseError(err)
    }
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

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate JWT secret length
        if self.jwt_secret.len() < 32 {
            return Err(ConfigError::ValidationError(
                "JWT_SECRET must be at least 32 characters long for security".to_string(),
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
