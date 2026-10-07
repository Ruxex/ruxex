use std::path::PathBuf;
use std::sync::Arc;

use genex_api::ApiError;
use genex_api::AuthProvider;
use genex_api::ChatCompletionApi;
use genex_api::ChatRequest;
use genex_api::ChatRequestInput;
use genex_api::ResponseStream;
use genex_api::SharedAuthProvider;
use genex_api::SseTelemetry;
use genex_api::StandardChatCompletionApi;
use genex_api::StreamResponse;
use genex_login::AuthManager;
use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_models_manager::manager::RemoteModelsManager;
use genex_models_manager::manager::SharedModelsManager;
use genex_models_manager::manager::StaticModelsManager;
use genex_protocol::account::ProviderAccount;
use genex_protocol::model_types::ModelsResponse;
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

pub(crate) fn my_provider_auth_provider(auth: Option<&GenexAuth>) -> SharedAuthProvider {
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

    fn auth(&self) -> ModelProviderFuture<'_, Option<GenexAuth>> {
        Box::pin(async move {
            match self.auth_manager.as_ref() {
                Some(auth_manager) => auth_manager.auth().await,
                None => None,
            }
        })
    }

    fn account_state(&self) -> ProviderAccountResult {
        let account = if self.info.requires_auth {
            self.auth_manager
                .as_ref()
                .and_then(|auth_manager| {
                    let auth = auth_manager.auth_cached()?;
                    if auth_manager.refresh_failure_for_auth(&auth).is_some() {
                        return None;
                    }
                    if matches!(auth, GenexAuth::Headers(_)) {
                        return None;
                    }
                    Some(auth)
                })
                .map(|auth| match &auth {
                    GenexAuth::ApiKey(_) => Ok(ProviderAccount::ApiKey),
                    GenexAuth::Headers(_) => Ok(ProviderAccount::ApiKey),
                })
                .transpose()?
        } else {
            None
        };

        Ok(ProviderAccountState {
            account,
            requires_auth: self.info.requires_auth,
        })
    }

    fn api_auth(
        &self,
    ) -> ModelProviderFuture<'_, genex_protocol::error::Result<SharedAuthProvider>> {
        Box::pin(async move { Ok(my_provider_auth_provider(self.auth().await.as_ref())) })
    }

    fn models_manager(
        &self,
        genex_home: PathBuf,
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
                    genex_home,
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
    use genex_http_client::HttpClientFactory;
    use genex_http_client::OutboundProxyPolicy;
    use genex_login::GenexAuth;
    use genex_models_manager::manager::RefreshStrategy;
    use genex_protocol::model_types::ModelsResponse;
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
        let model: genex_protocol::model_types::ModelInfo = serde_json::from_value(json!({
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
            AuthManager::from_auth_for_testing(GenexAuth::from_api_key("test-api-key"));
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

    #[test]
    fn my_provider_without_auth_requires_login() {
        let provider_info = ModelProviderInfo {
            requires_auth: true,
            ..ModelProviderInfo::create_my_provider()
        };
        let provider = MyModelProvider::new(provider_info, None);

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                account: None,
                requires_auth: true,
            })
        );
    }

    #[test]
    fn my_provider_with_api_key_reports_api_key_account() {
        let provider_info = ModelProviderInfo {
            requires_auth: true,
            ..ModelProviderInfo::create_my_provider()
        };
        let auth_manager = AuthManager::from_auth_for_testing(GenexAuth::from_api_key("test-key"));
        let provider = MyModelProvider::new(provider_info, Some(auth_manager));

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                account: Some(ProviderAccount::ApiKey),
                requires_auth: true,
            })
        );
    }

    #[test]
    fn my_provider_without_auth_can_disable_login() {
        let provider_info = ModelProviderInfo {
            requires_auth: false,
            ..ModelProviderInfo::create_my_provider()
        };
        let provider = MyModelProvider::new(provider_info, None);

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                account: None,
                requires_auth: false,
            })
        );
    }
}
