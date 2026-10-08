mod auth;
mod bearer_auth_provider;
mod cloudflare_provider;
mod contract;
mod home_marker;
mod model_catalog_source;
mod models_endpoint;
mod models_identity;
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
pub use auth::auth_provider_from_credential_manager;
pub use auth::unauthenticated_auth_provider;
pub use bearer_auth_provider::BearerAuthProvider;
pub use bearer_auth_provider::BearerAuthProvider as CoreAuthProvider;
pub use contract::ChatRequest;
pub use contract::ListModelsRequest;
pub use contract::ModelProvider;
pub use contract::ProviderAuthState;
pub use contract::ProviderChatEvent;
pub use contract::ProviderChatStream;
pub use contract::ProviderError;
pub use contract::ProviderFuture;
pub use contract::ProviderResult;
pub use contract::SharedModelProvider;
pub use contract::UsageRequest;
pub use genex_protocol::account::ProviderAccount;
pub use home_marker::PROVIDER_HOME_FORMAT_VERSION;
pub use home_marker::PROVIDER_HOME_MARKER_FILE;
pub use home_marker::ProviderHomeError;
pub use home_marker::ProviderHomeMarker;
pub use home_marker::mount_provider_home;
pub use model_catalog_source::ProviderModelCatalogSource;
pub use provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
pub use provider::ModelProviderFuture;
pub use provider::ProviderAccountError;
pub use provider::ProviderAccountResult;
pub use provider::ProviderAccountState;
pub use provider::ProviderApi;
pub use provider::ProviderAuthError;
pub use provider::ProviderAuthRecoveryMessages;
pub use provider::ProviderAuthResult;
pub use provider::ProviderCapabilities;
pub use provider::ProviderDeleteResult;
pub use provider::ProviderDescriptor;
pub use provider::ProviderLoginRequest;
pub use provider::ProviderLogoutResult;
pub use provider::ProviderRuntime;
pub use provider::ProviderUnauthorizedRecovery;
pub use provider::ProviderUsage;
pub use provider::ProviderUsageResult;
pub use provider::RemoteCompactionSupport;
pub use provider::SharedProviderRuntime;
pub use provider::create_model_provider;
