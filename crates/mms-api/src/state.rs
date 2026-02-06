use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use tokio::sync::mpsc;

use crate::auth::google::{self, OpenIdClient};
use crate::{
    ApiConfig,
    config::Environment,
    user::email::{EmailJob, EmailService},
};
use sqlx::PgPool;

/// JWT and password-hashing configuration.
#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub bcrypt_cost: u32,
    pub jwt_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
}

/// Cookie-related configuration.
#[derive(Clone)]
pub struct CookieConfig {
    pub cookie_domain: String,
    pub cookie_key: Key,
    pub environment: Environment,
}

/// Google OIDC configuration.
#[derive(Clone)]
pub struct OidcConfig {
    pub oidc_client: OpenIdClient,
    pub oidc_flow_expiry_minutes: i64,
    pub frontend_url: String,
}

#[derive(Clone)]
pub struct ApiState {
    pub auth: AuthConfig,
    pub cookie: CookieConfig,
    pub oidc: OidcConfig,
    pub pool: PgPool,
    pub email_tx: Option<mpsc::UnboundedSender<EmailJob>>,
}

impl ApiState {
    pub async fn new(config: ApiConfig, pool: PgPool) -> anyhow::Result<Self> {
        // Create cookie key
        let cookie_key = Key::from(config.cookie_secret.as_bytes());

        // Create Google OIDC client
        let oidc_client = google::create_oidc_client(
            config.google_client_id,
            config.google_client_secret,
            config.redirect_url,
        )
        .await?;

        // Initialize email worker if SMTP is configured
        let email_tx = if let (
            Some(host),
            Some(username),
            Some(password),
            Some(from_email),
            Some(from_name),
        ) = (
            config.smtp_host.as_ref(),
            config.smtp_username.as_ref(),
            config.smtp_password.as_ref(),
            config.smtp_from_email.as_ref(),
            config.smtp_from_name.as_ref(),
        ) {
            tracing::info!("Initializing email service with host: {}", host);
            match EmailService::new(
                host,
                username,
                password,
                from_email,
                from_name,
                &config.frontend_url,
            ) {
                Ok(service) => {
                    tracing::info!("Email service initialized successfully");
                    let tx = crate::user::email::start_email_worker(service);
                    tracing::info!("Email background worker started");
                    Some(tx)
                }
                Err(e) => {
                    tracing::error!("Failed to initialize email service: {e}");
                    None
                }
            }
        } else {
            tracing::warn!(
                "Email service not configured. SMTP config: host={:?}, username={:?}, password=***, from_email={:?}, from_name={:?}",
                config.smtp_host,
                config.smtp_username,
                config.smtp_from_email,
                config.smtp_from_name
            );
            None
        };

        tracing::info!(
            "Initializing ApiState with bcrypt_cost: {} (estimated login time: ~{}ms)",
            config.bcrypt_cost,
            2_u32.pow(config.bcrypt_cost) / 10
        );

        Ok(Self {
            auth: AuthConfig {
                jwt_secret: config.jwt_secret,
                bcrypt_cost: config.bcrypt_cost,
                jwt_expiry_hours: config.jwt_expiry_hours,
                refresh_token_expiry_days: config.refresh_token_expiry_days,
            },
            cookie: CookieConfig {
                cookie_domain: config.cookie_domain,
                cookie_key,
                environment: config.env,
            },
            oidc: OidcConfig {
                oidc_client,
                oidc_flow_expiry_minutes: config.oidc_flow_expiry_minutes,
                frontend_url: config.frontend_url.clone(),
            },
            pool,
            email_tx,
        })
    }
}

impl FromRef<ApiState> for Key {
    fn from_ref(state: &ApiState) -> Self {
        state.cookie.cookie_key.clone()
    }
}

impl FromRef<ApiState> for AuthConfig {
    fn from_ref(state: &ApiState) -> Self {
        state.auth.clone()
    }
}

impl FromRef<ApiState> for CookieConfig {
    fn from_ref(state: &ApiState) -> Self {
        state.cookie.clone()
    }
}

impl FromRef<ApiState> for OidcConfig {
    fn from_ref(state: &ApiState) -> Self {
        state.oidc.clone()
    }
}

impl FromRef<ApiState> for PgPool {
    fn from_ref(state: &ApiState) -> Self {
        state.pool.clone()
    }
}
