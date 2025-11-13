use std::env;

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub google_client_id: String,
    pub google_client_secret: String,
    pub redirect_url: String,
    pub frontend_url: String,
    pub database_url: String,
    pub jwt_secret: String,
    pub cookie_secret: String,
}

// TODO: Move this in a CliConfig
impl ApiConfig {
    pub fn from_env() -> Result<Self, env::VarError> {
        // TODO: proper message error for cookie_secret too short
        Ok(Self {
            google_client_id: env::var("GOOGLE_CLIENT_ID")?,
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET")?,
            redirect_url: env::var("REDIRECT_URL")?,
            frontend_url: env::var("FRONTEND_URL")?,
            database_url: env::var("DATABASE_URL")?,
            jwt_secret: env::var("JWT_SECRET")?,
            cookie_secret: env::var("COOKIE_SECRET")?,
        })
    }
}
