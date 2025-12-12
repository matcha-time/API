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

/// Create an OIDC client for Google OAuth
pub async fn create_oidc_client(
    client_id: String,
    client_secret: String,
    redirect_url: String,
) -> anyhow::Result<OpenIdClient> {
    // Discover Google's OIDC configuration
    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new("https://accounts.google.com".to_string())?,
        &reqwest::Client::new(),
    )
    .await?;

    // Create OIDC client
    let oidc_client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url)?);

    Ok(oidc_client)
}
