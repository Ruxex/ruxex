use std::sync::Arc;

use genex_http_client::HttpClientFactory;
use genex_login::CredentialManager;
use genex_model_provider_info::ModelProviderInfo;

use crate::ModelProvider;
use crate::ProviderDescriptor;

/// Process-local inputs supplied when a host constructs its concrete provider.
///
/// A daemon host reconstructs this value after loading its startup
/// configuration. It is not a serialized provider instance and must not be
/// treated as one.
pub struct ProviderInit {
    pub provider_info: ModelProviderInfo,
    pub credential_manager: Arc<CredentialManager>,
    pub http_client_factory: HttpClientFactory,
}

/// Compile-time provider construction contract for host binaries.
pub trait ModelProviderFactory: ModelProvider + Sized {
    /// Returns the stable identity used before constructing the provider.
    fn provider_descriptor() -> ProviderDescriptor;

    fn new(init: ProviderInit) -> Self;
}
