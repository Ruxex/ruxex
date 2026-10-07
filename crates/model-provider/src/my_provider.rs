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
use genex_http_client::HttpClientFactory;
use genex_login::AuthManager;
use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_protocol::account::ProviderAccount;
use genex_protocol::account::ProviderAuthStatus;
use genex_protocol::error::Result as CoreResult;
use http::HeaderMap;
use http::HeaderValue;

use super::auth::cached_provider_auth;
use super::auth::provider_auth;
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
    provider_id: Option<String>,
    info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
}

impl MyModelProvider {
    pub(crate) fn new(
        provider_id: Option<String>,
        info: ModelProviderInfo,
        auth_manager: Option<Arc<AuthManager>>,
    ) -> Self {
        Self {
            provider_id,
            info,
            auth_manager,
        }
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

    fn api_key_model_discovery_enabled_by_default(&self) -> bool {
        true
    }

    fn list_models(
        &self,
        client_version: &str,
        http_client_factory: HttpClientFactory,
    ) -> ModelProviderFuture<'_, CoreResult<ModelCatalogSnapshot>> {
        let client_version = client_version.to_string();
        let endpoint = ModelsEndpoint::new_with_provider_auth_mode(
            self.provider_id.clone(),
            self.info.clone(),
            self.auth_manager.clone(),
            ModelsEndpointAuthMode::MyProvider,
        );
        Box::pin(async move {
            endpoint
                .list_models(&client_version, http_client_factory)
                .await
        })
    }

    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        self.auth_manager.clone()
    }

    fn provider_id(&self) -> Option<&str> {
        self.provider_id.as_deref()
    }

    fn auth(&self) -> ModelProviderFuture<'_, Option<GenexAuth>> {
        Box::pin(async move {
            provider_auth(
                self.auth_manager.as_ref(),
                None,
                &self.info,
                self.provider_id(),
            )
            .await
        })
    }

    fn account_state(&self) -> ProviderAccountResult {
        let auth = self
            .auth_manager
            .as_ref()
            .and_then(|auth_manager| cached_provider_auth(auth_manager, self.provider_id()));
        let auth_invalid = auth.as_ref().is_some_and(|auth| {
            self.auth_manager
                .as_ref()
                .is_some_and(|auth_manager| auth_manager.refresh_failure_for_auth(auth).is_some())
        });
        let account = if self.info.requires_auth {
            auth.filter(|_| !auth_invalid)
                .filter(|auth| !matches!(auth, GenexAuth::Headers(_)))
                .map(|auth| match &auth {
                    GenexAuth::ApiKey(_) => Ok(ProviderAccount::ApiKey),
                    GenexAuth::Headers(_) => Ok(ProviderAccount::ApiKey),
                })
                .transpose()?
        } else {
            None
        };

        let auth_status = if self.info.requires_auth {
            if auth_invalid {
                ProviderAuthStatus::Invalid
            } else if account.is_some() {
                ProviderAuthStatus::Authenticated
            } else {
                ProviderAuthStatus::Unauthenticated
            }
        } else {
            ProviderAuthStatus::NotRequired
        };

        Ok(ProviderAccountState {
            provider_id: self
                .provider_id()
                .unwrap_or(self.info.name.as_str())
                .to_string(),
            account,
            account_id: self.auth_manager.as_ref().and_then(|auth_manager| {
                cached_provider_auth(auth_manager, self.provider_id())
                    .and_then(|auth| auth.get_account_id())
            }),
            display_name: None,
            auth_status,
            usage: None,
            requires_auth: self.info.requires_auth,
        })
    }

    fn api_auth(
        &self,
    ) -> ModelProviderFuture<'_, genex_protocol::error::Result<SharedAuthProvider>> {
        Box::pin(async move { Ok(my_provider_auth_provider(self.auth().await.as_ref())) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_catalog_source::ProviderModelCatalogSource;
    use genex_http_client::HttpClientFactory;
    use genex_http_client::OutboundProxyPolicy;
    use genex_login::GenexAuth;
    use genex_models_manager::manager::RefreshStrategy;
    use genex_protocol::model_types::ModelCatalog;
    use serde_json::json;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::header;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    #[tokio::test]
    async fn my_provider_enables_api_key_model_discovery_by_default() {
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
            .respond_with(ResponseTemplate::new(200).set_body_json(ModelCatalog {
                models: vec![model.clone()],
            }))
            .expect(1)
            .mount(&server)
            .await;

        let mut provider_info = ModelProviderInfo::create_my_provider();
        provider_info.base_url = Some(server.uri());
        let auth_manager =
            AuthManager::from_auth_for_testing(GenexAuth::from_api_key("test-api-key"));
        let provider = Arc::new(MyModelProvider::new(
            None,
            provider_info,
            Some(auth_manager),
        ));
        let manager = genex_models_manager::manager::build_models_manager_without_cache(
            Arc::new(ProviderModelCatalogSource::new(provider.clone())),
            provider.auth_manager(),
            /*config_model_catalog*/ None,
        );
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
        let provider = MyModelProvider::new(None, provider_info, None);

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                provider_id: "My Provider".to_string(),
                account: None,
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Unauthenticated,
                usage: None,
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
        let provider = MyModelProvider::new(None, provider_info, Some(auth_manager));

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                provider_id: "My Provider".to_string(),
                account: Some(ProviderAccount::ApiKey),
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Authenticated,
                usage: None,
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
        let provider = MyModelProvider::new(None, provider_info, None);

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                provider_id: "My Provider".to_string(),
                account: None,
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::NotRequired,
                usage: None,
                requires_auth: false,
            })
        );
    }
}
