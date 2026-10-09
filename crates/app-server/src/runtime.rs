//! Shared provider/runtime construction for all AppServer transports.

use std::sync::Arc;

use genex_login::CredentialManager;
use genex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use genex_model_provider::ProviderDescriptor;
use genex_model_provider::SharedModelProvider;
use genex_model_provider::create_model_provider;
use genex_model_provider::provider_contract;
use genex_model_provider_info::ModelProviderInfo;

/// The single provider contract shared by an AppServer transport and its clients.
#[derive(Clone)]
pub struct AppServerRuntime {
    provider: SharedModelProvider,
}

impl AppServerRuntime {
    /// Returns the single provider owned by this runtime.
    pub fn provider(&self) -> SharedModelProvider {
        Arc::clone(&self.provider)
    }

    pub(crate) fn credential_manager(&self) -> Option<Arc<CredentialManager>> {
        self.provider.credential_manager()
    }

    /// Returns the descriptor used for provider-home mount validation.
    pub fn provider_descriptor(&self) -> ProviderDescriptor {
        self.provider.descriptor()
    }
}

/// Constructs the shared provider/runtime state used by an AppServer.
pub struct AppServerRuntimeBuilder {
    provider: SharedModelProvider,
}

impl AppServerRuntimeBuilder {
    /// Starts building an AppServer runtime from its single provider
    /// implementation.
    pub fn new(provider: SharedModelProvider) -> Self {
        Self { provider }
    }

    /// Starts building a runtime for the provider selected by host configuration.
    ///
    /// Provider selection belongs at this host boundary. Callers that already own a concrete
    /// provider can use [`Self::new`].
    pub fn from_provider_info(
        provider_info: ModelProviderInfo,
        credential_manager: Option<Arc<CredentialManager>>,
    ) -> Self {
        Self::new(provider_contract(create_model_provider(
            provider_info,
            credential_manager,
        )))
    }

    /// Returns the descriptor that must be used when mounting this configured provider's data
    /// root, without constructing credentials or opening provider state.
    pub fn descriptor_for(provider_info: &ModelProviderInfo) -> ProviderDescriptor {
        configured_provider_descriptor(provider_info)
    }

    /// Builds the runtime around the injected provider.
    pub fn build(self) -> AppServerRuntime {
        AppServerRuntime {
            provider: self.provider,
        }
    }
}

/// Creates the configured AppServer runtime for startup paths that do not
/// receive an already-built runtime from their host.
pub(crate) fn configured_app_server_runtime(
    provider_info: ModelProviderInfo,
    credential_manager: Arc<CredentialManager>,
) -> AppServerRuntime {
    AppServerRuntimeBuilder::from_provider_info(provider_info, Some(credential_manager)).build()
}

pub(crate) fn configured_provider_descriptor(
    provider_info: &ModelProviderInfo,
) -> ProviderDescriptor {
    ProviderDescriptor::new(
        provider_info
            .id
            .as_deref()
            .unwrap_or(provider_info.name.as_str()),
        CURRENT_PROVIDER_PROTOCOL_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_is_derived_from_the_runtime_provider() {
        let runtime =
            AppServerRuntimeBuilder::from_provider_info(ModelProviderInfo::default(), None).build();

        assert_eq!(
            runtime.provider_descriptor(),
            runtime.provider().descriptor()
        );
    }
}
