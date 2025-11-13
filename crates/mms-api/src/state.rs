use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use oauth2::{EndpointMaybeSet, EndpointNotSet, EndpointSet};
use openidconnect::{
    ClientId, ClientSecret, IssuerUrl, RedirectUrl,
    core::{CoreClient, CoreProviderMetadata},
};

use crate::ApiConfig;
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
    pub frontend_url: String,
    pub cookie_key: Key,
    pub pool: PgPool,
}

impl ApiState {
    pub async fn new(config: ApiConfig) -> anyhow::Result<Self> {
        // Create cookie key
        let cookie_key = Key::from(config.cookie_secret.as_bytes());

        // Initialize database pool and run migrations
        let pool = mms_db::create_pool(&config.database_url).await?;
        mms_db::ensure_db_and_migrate(&config.database_url, &pool).await?;

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

        Ok(Self {
            oidc_client,
            jwt_secret: config.jwt_secret,
            frontend_url: config.frontend_url,
            cookie_key,
            pool,
        })
    }
}

impl FromRef<ApiState> for Key {
    fn from_ref(state: &ApiState) -> Self {
        state.cookie_key.clone()
    }
}
