use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use oauth2::{EndpointMaybeSet, EndpointNotSet, EndpointSet};
use openidconnect::{
    ClientId, ClientSecret, IssuerUrl, RedirectUrl,
    core::{CoreClient, CoreProviderMetadata},
};

use crate::{ApiConfig, config::Environment, user::email::EmailService};
use sqlx::PgPool;

pub type OpenIdClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Clone)]
pub struct ApiState {
    pub oidc_client: OpenIdClient,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
    pub oidc_flow_expiry_minutes: i64,
    pub frontend_url: String,
    pub cookie_key: Key,
    pub pool: PgPool,
    pub environment: Environment,
    pub email_service: Option<EmailService>,
}

impl ApiState {
    pub async fn new(config: ApiConfig, pool: PgPool) -> anyhow::Result<Self> {
        // Create cookie key
        let cookie_key = Key::from(config.cookie_secret.as_bytes());

        // Discover Google's OIDC configuration
        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new("https://accounts.google.com".to_string())?,
            &reqwest::Client::new(),
        )
        .await?;

        // Create OIDC client
        let oidc_client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.google_client_id),
            Some(ClientSecret::new(config.google_client_secret)),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_url)?);

        // Initialize email service if SMTP is configured
        let email_service = if let (
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
            match EmailService::new(
                host,
                username,
                password,
                from_email,
                from_name,
                &config.frontend_url,
            ) {
                Ok(service) => Some(service),
                Err(e) => {
                    tracing::error!("Failed to initialize email service: {e}");
                    None
                }
            }
        } else {
            tracing::warn!("Email service not configured (missing SMTP environment variables)");
            None
        };

        Ok(Self {
            oidc_client,
            jwt_secret: config.jwt_secret,
            jwt_expiry_hours: config.jwt_expiry_hours,
            refresh_token_expiry_days: config.refresh_token_expiry_days,
            oidc_flow_expiry_minutes: config.oidc_flow_expiry_minutes,
            frontend_url: config.frontend_url.clone(),
            cookie_key,
            pool,
            environment: config.env,
            email_service,
        })
    }
}

impl FromRef<ApiState> for Key {
    fn from_ref(state: &ApiState) -> Self {
        tracing::debug!("FromRef<ApiState> for Key called");
        state.cookie_key.clone()
    }
}
