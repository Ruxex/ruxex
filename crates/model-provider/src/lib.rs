mod auth;
mod bearer_auth_provider;
mod cloudflare_provider;
mod model_catalog_source;
mod models_endpoint;
mod models_identity;
mod my_provider;
mod provider;
pub mod test_support;

/// Effective destination and credentials that permit reusing a Responses socket.
#[derive(Debug, PartialEq, Eq)]
pub struct ResponsesConnectionKey {
    base_url: String,
    auth_revision: Option<u64>,
}

impl ResponsesConnectionKey {
    pub fn new(provider: &genex_api::Provider, auth_revision: Option<u64>) -> Self {
        Self {
            base_url: provider.base_url.clone(),
            auth_revision,
        }
    }
}

pub use auth::ProviderAuthScope;
pub use auth::ResolvedProviderAuth;
pub use auth::auth_provider_from_auth;
pub use auth::auth_provider_from_auth_manager;
pub use auth::unauthenticated_auth_provider;
pub use bearer_auth_provider::BearerAuthProvider;
pub use bearer_auth_provider::BearerAuthProvider as CoreAuthProvider;
pub use genex_protocol::account::ProviderAccount;
pub use model_catalog_source::ProviderModelCatalogSource;
pub use provider::ModelProvider;
pub use provider::ModelProviderFuture;
pub use provider::ProviderAccountError;
pub use provider::ProviderAccountResult;
pub use provider::ProviderAccountState;
pub use provider::ProviderApi;
pub use provider::ProviderAuthRecoveryMessages;
pub use provider::ProviderCapabilities;
pub use provider::ProviderUnauthorizedRecovery;
pub use provider::RemoteCompactionSupport;
pub use provider::SharedModelProvider;
pub use provider::create_model_provider;
