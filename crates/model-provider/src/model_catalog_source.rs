use genex_http_client::HttpClientFactory;
use genex_models_manager::manager::ModelCatalogFuture;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_models_manager::manager::ModelCatalogSource;
use genex_protocol::error::Result as CoreResult;

use crate::models_identity;
use crate::provider::SharedModelProvider;

/// Adapts a runtime provider to the manager-facing catalog source abstraction.
#[derive(Clone, Debug)]
pub struct ProviderModelCatalogSource {
    provider: SharedModelProvider,
}

impl ProviderModelCatalogSource {
    pub fn new(provider: SharedModelProvider) -> Self {
        Self { provider }
    }
}

impl ModelCatalogSource for ProviderModelCatalogSource {
    fn identity(&self) -> Option<String> {
        let auth = self
            .provider
            .credential_manager()
            .and_then(|manager| manager.auth_cached());
        models_identity::identity(
            self.provider.provider_id(),
            self.provider.info(),
            auth.as_ref(),
        )
        .ok()
    }

    fn has_command_auth(&self) -> bool {
        self.provider.info().has_command_auth()
    }

    fn uses_managed_auth(&self) -> ModelCatalogFuture<'_, bool> {
        Box::pin(async { false })
    }

    fn supports_api_key_models(&self) -> bool {
        self.provider.info().env_key.is_some()
    }

    fn supports_remote_models(&self) -> bool {
        true
    }

    fn list_models<'a>(
        &'a self,
        client_version: &'a str,
        http_client_factory: HttpClientFactory,
    ) -> ModelCatalogFuture<'a, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(async move {
            self.provider
                .list_models(client_version, http_client_factory)
                .await
        })
    }
}
