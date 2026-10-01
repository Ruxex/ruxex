use std::path::PathBuf;
use std::sync::Arc;

use codex_api::ApiError;
use codex_api::AuthProvider;
use codex_api::ChatCompletionApi;
use codex_api::ChatRequest;
use codex_api::ChatRequestInput;
use codex_api::ResponseStream;
use codex_api::SharedAuthProvider;
use codex_api::SseTelemetry;
use codex_api::StandardChatCompletionApi;
use codex_api::StreamResponse;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::manager::RemoteModelsManager;
use codex_models_manager::manager::SharedModelsManager;
use codex_models_manager::manager::StaticModelsManager;
use codex_protocol::account::ProviderAccount;
use codex_protocol::model_types::ModelsResponse;
use http::HeaderMap;
use http::HeaderValue;

use super::auth::unauthenticated_auth_provider;
use super::models_endpoint::ModelsEndpoint;
use super::models_endpoint::ModelsEndpointAuthMode;
use super::provider::ModelProvider;
use super::provider::ModelProviderFuture;
use super::provider::ProviderAccountResult;
use super::provider::ProviderAccountState;
use super::provider::ProviderApi;

#[derive(Clone, Debug)]
pub(crate) struct MyModelProvider {
    info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
}

impl MyModelProvider {
    pub(crate) fn new(info: ModelProviderInfo, auth_manager: Option<Arc<AuthManager>>) -> Self {
        Self { info, auth_manager }
    }
}

#[derive(Clone, Debug)]
struct MyApiKeyAuthProvider {
    api_key: String,
}

pub(crate) fn my_provider_auth_provider(auth: Option<&CodexAuth>) -> SharedAuthProvider {
    let Some(auth) = auth else {
        return unauthenticated_auth_provider();
    };
    let Ok(api_key) = auth.get_token() else {
        return unauthenticated_auth_provider();
    };
    Arc::new(MyApiKeyAuthProvider { api_key }) as SharedAuthProvider
}

/// Runtime Chat adapter reserved for the built-in My Provider.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MyProviderChatCompletionApi;

impl ChatCompletionApi for MyProviderChatCompletionApi {
    fn build_request(&self, input: ChatRequestInput<'_>) -> Result<ChatRequest, ApiError> {
        StandardChatCompletionApi.build_request(input)
    }

    fn spawn_stream(
        &self,
        response: StreamResponse,
        idle_timeout: std::time::Duration,
        telemetry: Option<Arc<dyn SseTelemetry>>,
    ) -> ResponseStream {
        StandardChatCompletionApi.spawn_stream(response, idle_timeout, telemetry)
    }
}

impl AuthProvider for MyApiKeyAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        if let Ok(value) = HeaderValue::from_str(&format!("apikey {}", self.api_key)) {
            headers.insert("APIKEY", value);
        }
    }
}

impl ModelProvider for MyModelProvider {
    fn info(&self) -> &ModelProviderInfo {
        &self.info
    }

    fn api(&self) -> ProviderApi {
        ProviderApi::Chat(Arc::new(MyProviderChatCompletionApi))
    }

    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        self.auth_manager.clone()
    }

    fn auth(&self) -> ModelProviderFuture<'_, Option<CodexAuth>> {
        Box::pin(async move {
            match self.auth_manager.as_ref() {
                Some(auth_manager) => auth_manager.auth().await,
                None => None,
            }
        })
    }

    fn account_state(&self) -> ProviderAccountResult {
        Ok(ProviderAccountState {
            account: Some(ProviderAccount::ApiKey),
            requires_auth: false,
        })
    }

    fn api_auth(
        &self,
    ) -> ModelProviderFuture<'_, codex_protocol::error::Result<SharedAuthProvider>> {
        Box::pin(async move { Ok(my_provider_auth_provider(self.auth().await.as_ref())) })
    }

    fn models_manager(
        &self,
        codex_home: PathBuf,
        config_model_catalog: Option<ModelsResponse>,
    ) -> SharedModelsManager {
        match config_model_catalog {
            Some(model_catalog) => Arc::new(StaticModelsManager::new(
                self.auth_manager.clone(),
                model_catalog,
            )),
            None => {
                let endpoint = Arc::new(ModelsEndpoint::new_with_auth_mode(
                    self.info.clone(),
                    self.auth_manager.clone(),
                    ModelsEndpointAuthMode::MyProvider,
                ));
                Arc::new(RemoteModelsManager::new(
                    codex_home,
                    endpoint,
                    self.auth_manager.clone(),
                ))
            }
        }
    }

    fn models_manager_without_cache(
        &self,
        config_model_catalog: Option<ModelsResponse>,
    ) -> SharedModelsManager {
        match config_model_catalog {
            Some(model_catalog) => Arc::new(StaticModelsManager::new(
                self.auth_manager.clone(),
                model_catalog,
            )),
            None => {
                let endpoint = Arc::new(ModelsEndpoint::new_with_auth_mode(
                    self.info.clone(),
                    self.auth_manager.clone(),
                    ModelsEndpointAuthMode::MyProvider,
                ));
                Arc::new(RemoteModelsManager::new_without_cache(
                    endpoint,
                    self.auth_manager.clone(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_http_client::HttpClientFactory;
    use codex_http_client::OutboundProxyPolicy;
    use codex_login::CodexAuth;
    use codex_models_manager::manager::RefreshStrategy;
    use codex_protocol::model_types::ModelsResponse;
    use serde_json::json;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::header;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    #[tokio::test]
    async fn model_list_is_fetched_dynamically_with_my_provider_auth() {
        let server = MockServer::start().await;
        let model: codex_protocol::model_types::ModelInfo = serde_json::from_value(json!({
            "slug": "my-dynamic-model",
            "display_name": "My Dynamic Model",
            "description": "model returned by My Provider",
            "default_reasoning_level": "medium",
            "supported_reasoning_levels": [{"effort": "medium", "description": "medium"}],
            "shell_type": "shell_command",
            "visibility": "list",
            "supported_in_api": true,
            "priority": 0,
            "model_messages": null,
            "support_verbosity": false,
            "default_verbosity": null,
            "apply_patch_tool_type": null,
            "truncation_policy": {"mode": "bytes", "limit": 10000},
            "supports_image_detail_original": false,
            "context_window": 272000,
            "max_context_window": 272000,
            "experimental_supported_tools": []
        }))
        .expect("test model should deserialize");

        Mock::given(method("GET"))
            .and(path("/models"))
            .and(header("APIKEY", "apikey test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ModelsResponse {
                models: vec![model.clone()],
            }))
            .expect(1)
            .mount(&server)
            .await;

        let mut provider_info = ModelProviderInfo::create_my_provider();
        provider_info.base_url = Some(server.uri());
        let auth_manager =
            AuthManager::from_auth_for_testing(CodexAuth::from_api_key("test-api-key"));
        let provider = MyModelProvider::new(provider_info, Some(auth_manager));
        let manager = provider.models_manager_without_cache(None);
        manager.set_api_key_model_discovery_enabled(true);

        let catalog = manager
            .raw_model_catalog(
                RefreshStrategy::Online,
                HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            )
            .await;

        assert_eq!(catalog.models.len(), 1);
        assert_eq!(catalog.models[0].slug, model.slug);
    }
}
