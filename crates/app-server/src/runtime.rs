//! Shared provider/runtime construction for all AppServer transports.

use std::sync::Arc;

use genex_login::CredentialManager;
use genex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use genex_model_provider::ProviderDescriptor;
use genex_model_provider::SharedModelProvider;
use genex_model_provider::SharedProviderRuntime;
use genex_model_provider::create_model_provider;
use genex_model_provider::provider_contract;
use genex_model_provider_info::ModelProviderInfo;

/// Provider state shared by an AppServer transport and its request processor.
///
/// The legacy execution core still consumes [`SharedProviderRuntime`] while
/// provider-neutral app-server APIs consume [`SharedModelProvider`]. The latter
/// is always derived from the former here, so a runtime cannot accidentally
/// expose one provider while executing turns with another.
#[derive(Clone)]
pub struct AppServerRuntime {
    provider_runtime: SharedProviderRuntime,
    provider_contract: SharedModelProvider,
}

impl AppServerRuntime {
    pub(crate) fn provider_runtime(&self) -> SharedProviderRuntime {
        Arc::clone(&self.provider_runtime)
    }

    pub(crate) fn provider_contract(&self) -> SharedModelProvider {
        Arc::clone(&self.provider_contract)
    }

    pub(crate) fn credential_manager(&self) -> Option<Arc<CredentialManager>> {
        self.provider_runtime.credential_manager()
    }

    /// Returns the descriptor used for provider-home mount validation.
    pub fn provider_descriptor(&self) -> ProviderDescriptor {
        self.provider_contract.descriptor()
    }
}

/// Constructs the shared provider/runtime state used by an AppServer.
pub struct AppServerRuntimeBuilder {
    provider_runtime: SharedProviderRuntime,
}

impl AppServerRuntimeBuilder {
    /// Starts building an AppServer runtime from its single provider
    /// implementation.
    pub fn new(provider_runtime: SharedProviderRuntime) -> Self {
        Self { provider_runtime }
    }

    /// Builds the runtime and derives its provider-neutral contract adapter.
    pub fn build(self) -> AppServerRuntime {
        let provider_contract = provider_contract(Arc::clone(&self.provider_runtime));
        AppServerRuntime {
            provider_runtime: self.provider_runtime,
            provider_contract,
        }
    }
}

/// Creates the configured AppServer runtime for startup paths that do not
/// receive an already-built runtime from their host.
pub(crate) fn configured_app_server_runtime(
    provider_info: ModelProviderInfo,
    credential_manager: Arc<CredentialManager>,
) -> AppServerRuntime {
    AppServerRuntimeBuilder::new(create_model_provider(
        provider_info,
        Some(credential_manager),
    ))
    .build()
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
            AppServerRuntimeBuilder::new(create_model_provider(ModelProviderInfo::default(), None))
                .build();

        assert_eq!(
            runtime.provider_descriptor(),
            runtime.provider_runtime().descriptor()
        );
    }
}
