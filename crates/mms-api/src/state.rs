use crate::ApiConfig;
use axum_extra::extract::cookie::Key;
use oauth2::{EndpointMaybeSet, EndpointNotSet, EndpointSet};
use openidconnect::{
    ClientId, ClientSecret, IssuerUrl, RedirectUrl,
    core::{CoreClient, CoreProviderMetadata},
};

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
    pub cookie_key: Key,
}

impl ApiState {
    pub async fn new(config: ApiConfig) -> Result<Self, Box<dyn std::error::Error>> {
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

        Ok(Self {
            oidc_client,
            jwt_secret: config.jwt_secret,
            cookie_key,
        })
    }
}
