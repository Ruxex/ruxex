use std::time::Duration;

use genex_api::ModelsClient;
use genex_api::ReqwestTransport;
use genex_api::map_api_error;
use genex_http_client::ClientRouteClass;
use genex_http_client::HttpClientFactory;
use genex_login::GenexAuth;
use genex_login::default_client::create_client_for_route_async;
use genex_model_provider_info::ModelProviderInfo;
use genex_models_manager::manager::ModelsEndpointClient;
use genex_models_manager::manager::ModelsEndpointFuture;
use genex_models_manager::manager::ModelsEndpointResponse;
use genex_protocol::error::Result;
use genex_protocol::model_types::ModelsResponse;
use http::HeaderMap;
use tokio::time::timeout;

use super::AmazonBedrockModelProvider;
use super::catalog::normalize_bedrock_catalog;

const MODELS_REFRESH_TIMEOUT: Duration = Duration::from_secs(5);

/// Provider-compatible model discovery for Bedrock's Responses endpoints.
///
/// Bedrock authentication is not expressible through the generic configured-provider
/// endpoint, because it may require SigV4 signing. This adapter keeps that provider-owned
/// authentication while sharing the remote catalog/cache lifecycle with other providers.
#[derive(Clone, Debug)]
pub(super) struct BedrockModelsEndpoint {
    provider: AmazonBedrockModelProvider,
}

impl BedrockModelsEndpoint {
    pub(super) fn new(provider: AmazonBedrockModelProvider) -> Self {
        Self { provider }
    }

    fn identity_value(&self) -> String {
        let endpoint = match self.provider.endpoint {
            super::BedrockEndpoint::Mantle => "mantle",
            super::BedrockEndpoint::Runtime => "runtime",
        };
        let base_url = self.provider.info.base_url.as_deref().unwrap_or("regional");
        let region = self.provider.aws.region.as_deref().unwrap_or("default");
        format!("amazon-bedrock:{endpoint}:{base_url}:{region}")
    }

    async fn list_models(
        &self,
        client_version: &str,
        http_client_factory: HttpClientFactory,
    ) -> Result<ModelsEndpointResponse> {
        let auth = self.provider.auth().await;
        let mut provider_info: ModelProviderInfo = self.provider.info.clone();
        provider_info.base_url = self.provider.runtime_base_url().await?;
        let api_provider = provider_info.to_api_provider(auth.as_ref().map(GenexAuth::auth_mode))?;
        let api_auth = self.provider.api_auth().await?;
        let request_url = ModelsClient::<ReqwestTransport>::request_url(&api_provider, client_version);

        let models_result = timeout(MODELS_REFRESH_TIMEOUT, async {
            let transport = create_client_for_route_async(
                http_client_factory,
                request_url.clone(),
                ClientRouteClass::Api,
            )
            .await
            .map(ReqwestTransport::from_http_client)
            .map_err(genex_protocol::error::GenexErr::Io)?;
            ModelsClient::new(transport, api_provider, api_auth)
                .list_models(request_url, HeaderMap::new())
                .await
                .map_err(map_api_error)
        })
        .await
        .map_err(|_| genex_protocol::error::GenexErr::RequestTimeout)??;

        let (models, etag) = models_result;
        let normalized = normalize_bedrock_catalog(ModelsResponse { models });
        Ok(ModelsEndpointResponse {
            models: normalized.models,
            etag,
            identity: self.identity_value(),
        })
    }
}

impl ModelsEndpointClient for BedrockModelsEndpoint {
    fn identity(&self) -> Option<String> {
        Some(self.identity_value())
    }

    fn has_command_auth(&self) -> bool {
        self.provider.info.has_command_auth()
    }

    fn uses_managed_auth(&self) -> ModelsEndpointFuture<'_, bool> {
        // Bedrock credentials may come from AWS SDK state rather than AuthManager. Returning
        // true here tells the remote manager that this provider owns a live model catalog.
        Box::pin(async { true })
    }

    fn list_models<'a>(
        &'a self,
        client_version: &'a str,
        http_client_factory: HttpClientFactory,
    ) -> ModelsEndpointFuture<'a, Result<ModelsEndpointResponse>> {
        Box::pin(BedrockModelsEndpoint::list_models(
            self,
            client_version,
            http_client_factory,
        ))
    }
}
