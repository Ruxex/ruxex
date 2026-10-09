use genex_http_client::HttpClientFactory;
use genex_models_manager::manager::ModelCatalogFuture;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_models_manager::manager::ModelCatalogSource;
use genex_protocol::error::Result as CoreResult;

use crate::contract::ListModelsRequest;
use crate::contract::SharedModelProvider;

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
        self.provider.model_catalog_identity().ok()
    }

    fn has_command_auth(&self) -> bool {
        self.provider.capabilities().command_auth
    }

    fn uses_managed_auth(&self) -> ModelCatalogFuture<'_, bool> {
        Box::pin(async { false })
    }

    fn supports_api_key_models(&self) -> bool {
        self.provider.capabilities().api_key_model_discovery
    }

    fn supports_remote_models(&self) -> bool {
        self.provider.capabilities().model_listing
    }

    fn list_models<'a>(
        &'a self,
        client_version: &'a str,
        http_client_factory: HttpClientFactory,
    ) -> ModelCatalogFuture<'a, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(async move {
            let catalog = self
                .provider
                .list_models(ListModelsRequest::new(client_version, http_client_factory))
                .await
                .map_err(|error| genex_protocol::error::GenexErr::Fatal(error.to_string()))?;

            Ok(ModelCatalogSnapshot {
                catalog: catalog.catalog,
                identity: catalog.identity,
                etag: catalog.etag,
            })
        })
    }
}
