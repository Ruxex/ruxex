use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use genex_api::ApiError;
use genex_api::ChatCompletionApi;
use genex_api::Provider;
use genex_api::SharedAuthProvider;
use genex_api::StandardChatCompletionApi;
use genex_api::TransportError;
use genex_api::is_azure_responses_provider;
use genex_http_client::HttpClientFactory;
use genex_login::AuthManager;
use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_model_provider_info::WireApi;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_protocol::account::ProviderAccount;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result as CoreResult;

use crate::auth::ProviderAuthScope;
use crate::auth::ResolvedProviderAuth;
use crate::auth::auth_manager_for_provider;
use crate::auth::resolve_provider_auth;
use crate::auth::resolve_provider_auth_for_scope;
use crate::cloudflare_provider::CloudflareModelProvider;
use crate::models_endpoint::ModelsEndpoint;
use crate::my_provider::MyModelProvider;

/// Remote context-compaction protocols supported by a model provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCompactionSupport {
    /// The provider does not support remote compaction.
    Unsupported,
    /// The provider supports `compaction_trigger` items over the Responses endpoint.
    V2,
}

/// Optional provider-backed features that Genex may expose at runtime.
///
/// These capabilities are a provider-owned upper bound. Callers can disable
/// more functionality through normal config, but should not expose a feature
/// that the active provider marks unsupported here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub namespace_tools: bool,
    pub web_search: bool,
    pub external_web_access: bool,
    pub remote_compaction: RemoteCompactionSupport,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            namespace_tools: true,
            web_search: true,
            external_web_access: true,
            remote_compaction: RemoteCompactionSupport::Unsupported,
        }
    }
}

/// Current app-visible account state for a model provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAccountState {
    pub account: Option<ProviderAccount>,
    pub requires_auth: bool,
}

/// Outcome of a provider-owned attempt to recover from an authentication failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderUnauthorizedRecovery {
    /// The provider has no provider-specific authentication recovery configured.
    NotConfigured,
    /// The provider recovered its authentication state and the request can be retried.
    Recovered,
}

/// User-facing lifecycle messages for provider-owned authentication recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderAuthRecoveryMessages {
    pub started: &'static str,
    pub succeeded: &'static str,
}

/// Error returned when a provider cannot construct its app-visible account state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderAccountError {}

impl fmt::Display for ProviderAccountError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for ProviderAccountError {}

pub type ProviderAccountResult = std::result::Result<ProviderAccountState, ProviderAccountError>;

/// Runtime API selected for model execution.
#[derive(Clone)]
pub enum ProviderApi {
    /// The existing concrete Responses API implementation.
    Responses,
    /// A provider-specific Chat Completions wire adapter.
    Chat(Arc<dyn ChatCompletionApi>),
}

/// Runtime provider abstraction used by model execution.
///
/// Implementations own provider-specific behavior for a model backend. The
/// `ModelProviderInfo` returned by `info` is the serialized/configured provider
/// metadata used by the default provider-compatible implementation.
pub trait ModelProvider: fmt::Debug + Send + Sync {
    /// Returns the configured provider metadata.
    fn info(&self) -> &ModelProviderInfo;

    /// Resolves the runtime API implementation for this provider.
    fn api(&self) -> ProviderApi {
        match self.info().wire_api {
            WireApi::Responses => ProviderApi::Responses,
            WireApi::Chat => ProviderApi::Chat(Arc::new(StandardChatCompletionApi)),
        }
    }

    /// Fetches the provider's current model catalog.
    ///
    /// The default delegates to the provider-compatible `/models` endpoint.
    /// Providers with a different catalog API should override this method and
    /// return their normalized model metadata here.
    fn list_models(
        &self,
        client_version: &str,
        http_client_factory: HttpClientFactory,
    ) -> ModelProviderFuture<'_, CoreResult<ModelCatalogSnapshot>> {
        let client_version = client_version.to_string();
        let endpoint = ModelsEndpoint::new(self.info().clone(), self.auth_manager());
        Box::pin(async move {
            endpoint
                .list_models(&client_version, http_client_factory)
                .await
        })
    }

    /// Returns whether API-key model discovery should be enabled by default.
    fn api_key_model_discovery_enabled_by_default(&self) -> bool {
        false
    }

    /// Returns the provider-owned capability upper bounds.
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    /// Returns whether requests made through this provider should include attestation.
    fn supports_attestation(&self) -> bool {
        false
    }

    /// Returns the provider-scoped auth manager, when this provider uses one.
    ///
    /// TODO(celia-oai): Make auth manager access internal to this crate so callers
    /// resolve provider-specific auth only through `ModelProvider`. We first need
    /// to think through whether Genex should have a unified provider-specific auth
    /// manager throughout the codebase; that is a larger refactor than this change.
    fn auth_manager(&self) -> Option<Arc<AuthManager>>;

    /// Returns whether this transport failure can be recovered by provider-scoped authentication.
    ///
    /// The default preserves existing unauthorized-response handling. Providers with other
    /// authentication failure shapes may recognize additional response or request-signing errors.
    fn is_recoverable_auth_error(&self, error: &TransportError) -> bool {
        matches!(
            error,
            TransportError::Http { status, .. } if *status == http::StatusCode::UNAUTHORIZED
        )
    }

    /// Returns lifecycle messages when provider-owned authentication recovery is active.
    fn auth_recovery_messages(&self) -> Option<ProviderAuthRecoveryMessages> {
        None
    }

    /// Attempts provider-owned authentication recovery before using the auth manager.
    fn recover_from_unauthorized(
        &self,
    ) -> ModelProviderFuture<'_, genex_protocol::error::Result<ProviderUnauthorizedRecovery>> {
        Box::pin(async { Ok(ProviderUnauthorizedRecovery::NotConfigured) })
    }

    /// Returns the current provider-scoped auth value, if one is configured.
    fn auth(&self) -> ModelProviderFuture<'_, Option<GenexAuth>>;

    /// Returns the current app-visible account state for this provider.
    fn account_state(&self) -> ProviderAccountResult;

    /// Maps an API client error into the provider's user-facing error representation.
    fn map_api_error(&self, error: ApiError) -> GenexErr {
        genex_api::map_api_error(error)
    }

    /// Returns provider configuration adapted for the API client.
    fn api_provider(&self) -> ModelProviderFuture<'_, genex_protocol::error::Result<Provider>> {
        Box::pin(async move {
            let auth = self.auth().await;
            self.info()
                .to_api_provider(auth.as_ref().map(GenexAuth::auth_mode))
        })
    }

    /// Returns the provider base URL that will be used at request time.
    fn runtime_base_url(
        &self,
    ) -> ModelProviderFuture<'_, genex_protocol::error::Result<Option<String>>> {
        Box::pin(async { Ok(self.info().base_url.clone()) })
    }

    /// Returns the auth provider used to attach request credentials.
    fn api_auth(
        &self,
    ) -> ModelProviderFuture<'_, genex_protocol::error::Result<SharedAuthProvider>> {
        Box::pin(async move {
            let auth = self.auth().await;
            resolve_provider_auth(auth.as_ref(), self.info())
        })
    }

    /// Returns request credentials, optionally scoped to a Genex session task.
    fn api_auth_for_scope(
        &self,
        scope: ProviderAuthScope,
    ) -> ModelProviderFuture<'_, genex_protocol::error::Result<ResolvedProviderAuth>> {
        Box::pin(async move {
            if !provider_uses_stored_auth_path(self.info()) {
                return self.api_auth().await.map(ResolvedProviderAuth::new);
            }
            let auth = self.auth().await;
            resolve_provider_auth_for_scope(self.auth_manager(), auth.as_ref(), self.info(), scope)
                .await
        })
    }
}

pub type ModelProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Shared runtime model provider handle.
pub type SharedModelProvider = Arc<dyn ModelProvider>;

fn provider_uses_stored_auth_path(provider: &ModelProviderInfo) -> bool {
    provider.requires_auth
        && provider.env_key.is_none()
        && provider.experimental_bearer_token.is_none()
        && provider.auth.is_none()
}

/// Creates the default runtime model provider for configured provider metadata.
pub fn create_model_provider(
    provider_info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
) -> SharedModelProvider {
    match provider_info.name.as_str() {
        genex_model_provider_info::MY_PROVIDER_NAME => {
            Arc::new(MyModelProvider::new(provider_info, auth_manager))
        }
        genex_model_provider_info::CLOUDFLARE_PROVIDER_NAME => {
            Arc::new(CloudflareModelProvider::new(provider_info, auth_manager))
        }
        _ => Arc::new(ConfiguredModelProvider::new(provider_info, auth_manager)),
    }
}

/// Runtime model provider backed by configured `ModelProviderInfo`.
#[derive(Clone, Debug)]
struct ConfiguredModelProvider {
    info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
}

impl ConfiguredModelProvider {
    fn new(provider_info: ModelProviderInfo, auth_manager: Option<Arc<AuthManager>>) -> Self {
        let auth_manager = auth_manager_for_provider(auth_manager, &provider_info);
        Self {
            info: provider_info,
            auth_manager,
        }
    }
}

impl ModelProvider for ConfiguredModelProvider {
    fn info(&self) -> &ModelProviderInfo {
        &self.info
    }

    fn capabilities(&self) -> ProviderCapabilities {
        let remote_compaction = if self.info.is_responses_provider()
            || is_azure_responses_provider(&self.info.name, self.info.base_url.as_deref())
        {
            RemoteCompactionSupport::V2
        } else {
            RemoteCompactionSupport::Unsupported
        };

        ProviderCapabilities {
            remote_compaction,
            ..ProviderCapabilities::default()
        }
    }

    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        self.auth_manager.clone()
    }

    fn supports_attestation(&self) -> bool {
        false
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
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use genex_http_client::HttpClientFactory;
    use genex_http_client::OutboundProxyPolicy;
    use genex_model_provider_info::WireApi;
    use genex_model_provider_info::create_responses_provider_with_base_url;
    use genex_models_manager::manager::ModelCatalogSource;
    use genex_models_manager::manager::RefreshStrategy;
    use genex_models_manager::manager::build_models_manager;
    use genex_protocol::config_types::ModelProviderAuthInfo;
    use genex_protocol::model_types::ModelCatalog;
    use genex_protocol::model_types::ModelInfo;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::header_regex;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    use super::*;
    use crate::model_catalog_source::ProviderModelCatalogSource;

    #[test]
    fn my_provider_selects_chat_runtime_api() {
        let provider = create_model_provider(
            ModelProviderInfo::create_my_provider(),
            /*auth_manager*/ None,
        );

        assert!(matches!(provider.api(), ProviderApi::Chat(_)));
    }

    #[test]
    fn my_provider_factory_reports_unauthenticated_requires_auth() {
        let provider = create_model_provider(
            ModelProviderInfo::create_my_provider(),
            /*auth_manager*/ None,
        );

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                account: None,
                requires_auth: true,
            })
        );
    }

    #[test]
    fn cloudflare_provider_selects_chat_runtime_api() {
        let provider = create_model_provider(
            ModelProviderInfo::create_cloudflare_provider(),
            /*auth_manager*/ None,
        );

        assert!(matches!(provider.api(), ProviderApi::Chat(_)));
    }

    #[test]
    fn provider_model_catalog_source_preserves_provider_capabilities() {
        let mut info = ModelProviderInfo::create_cloudflare_provider();
        info.base_url = Some("https://example.test".to_string());
        info.env_key = None;
        let provider = create_model_provider(info, /*auth_manager*/ None);
        let source = ProviderModelCatalogSource::new(provider);

        assert!(!source.supports_api_key_models());
        assert!(source.supports_remote_models());
        assert!(!source.has_command_auth());
        assert!(source.identity().is_some());
    }

    #[tokio::test]
    async fn cloudflare_provider_lists_models_from_cloudflare_search_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models/search"))
            .and(wiremock::matchers::query_param("task", "text-generation"))
            .and(wiremock::matchers::query_param("hide_experimental", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "errors": [],
                "messages": [],
                "result": [{
                    "name": "@cf/meta/llama-3.1-8b-instruct",
                    "description": "Llama 3.1 8B instruct"
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let mut info = ModelProviderInfo::create_cloudflare_provider();
        info.base_url = Some(server.uri());
        info.env_key = None;
        let provider = create_model_provider(info, /*auth_manager*/ None);
        let catalog = provider
            .list_models(
                "test-client",
                HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            )
            .await
            .expect("Cloudflare provider should list models");

        assert_eq!(catalog.catalog.models.len(), 1);
        assert_eq!(
            catalog.catalog.models[0].slug,
            "@cf/meta/llama-3.1-8b-instruct"
        );
        assert_eq!(
            catalog.catalog.models[0].description.as_deref(),
            Some("Llama 3.1 8B instruct")
        );
    }

    #[test]
    fn responses_provider_keeps_concrete_runtime_api() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*auth_manager*/ None,
        );

        assert!(matches!(provider.api(), ProviderApi::Responses));
    }

    fn provider_info_with_command_auth() -> ModelProviderInfo {
        ModelProviderInfo {
            auth: Some(ModelProviderAuthInfo {
                command: "print-token".to_string(),
                args: Vec::new(),
                timeout_ms: NonZeroU64::new(5_000).expect("timeout should be non-zero"),
                refresh_interval_ms: 300_000,
                cwd: std::env::current_dir()
                    .expect("current dir should be available")
                    .try_into()
                    .expect("current dir should be absolute"),
            }),
            requires_auth: false,
            ..ModelProviderInfo::create_responses_provider(/*base_url*/ None)
        }
    }

    fn test_genex_home() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("genex-model-provider-test-{}", std::process::id()))
    }

    fn provider_for(base_url: String) -> ModelProviderInfo {
        ModelProviderInfo {
            name: "mock".into(),
            base_url: Some(base_url),
            env_key: None,
            env_key_instructions: None,
            experimental_bearer_token: None,
            auth: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: Some(0),
            stream_max_retries: Some(0),
            stream_idle_timeout_ms: Some(5_000),
            websocket_connect_timeout_ms: None,
            requires_auth: false,
            supports_websockets: false,
            supports_standalone_web_search: false,
        }
    }

    fn remote_model(slug: &str) -> ModelInfo {
        serde_json::from_value(json!({
            "slug": slug,
            "display_name": slug,
            "description": null,
            "default_reasoning_level": "medium",
            "supported_reasoning_levels": [],
            "shell_type": "shell_command",
            "visibility": "list",
            "supported_in_api": true,
            "priority": 0,
            "upgrade": null,
            "support_verbosity": false,
            "default_verbosity": null,
            "apply_patch_tool_type": null,
            "truncation_policy": {"mode": "bytes", "limit": 10_000},
            "supports_image_detail_original": false,
            "context_window": 272_000,
            "max_context_window": 272_000,
            "experimental_supported_tools": [],
        }))
        .expect("valid model")
    }

    #[tokio::test]
    async fn scoped_auth_ignores_scope_for_non_responses_provider() {
        let provider = create_model_provider(
            create_responses_provider_with_base_url(
                "http://localhost:11434/v1",
                WireApi::Responses,
            ),
            /*auth_manager*/ None,
        );

        let auth = provider
            .api_auth_for_scope(ProviderAuthScope)
            .await
            .expect("auth should resolve");

        assert!(auth.auth.to_auth_headers().is_empty());
    }

    #[tokio::test]
    async fn my_provider_uses_api_key_header_auth() {
        let provider = create_model_provider(
            ModelProviderInfo {
                name: "My Provider".to_string(),
                base_url: Some("https://example.com".to_string()),
                ..ModelProviderInfo::default()
            },
            Some(AuthManager::from_auth_for_testing(GenexAuth::from_api_key(
                "test-key",
            ))),
        );

        let auth = provider.api_auth().await.expect("auth should resolve");
        assert_eq!(
            auth.to_auth_headers()
                .get("APIKEY")
                .and_then(|value| value.to_str().ok()),
            Some("apikey test-key")
        );
        assert!(!auth.to_auth_headers().contains_key("authorization"));
    }

    #[test]
    fn responses_provider_enables_remote_compaction() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*auth_manager*/ None,
        );

        assert_eq!(
            provider.capabilities(),
            ProviderCapabilities {
                remote_compaction: RemoteCompactionSupport::V2,
                ..ProviderCapabilities::default()
            }
        );
    }

    #[test]
    fn configured_provider_remote_compaction_matches_provider_support() {
        let cases = [
            (
                ModelProviderInfo::create_responses_provider(/*base_url*/ None),
                RemoteCompactionSupport::V2,
            ),
            (
                ModelProviderInfo {
                    name: "Azure".to_string(),
                    base_url: Some("https://example.com/provider".to_string()),
                    ..ModelProviderInfo::default()
                },
                RemoteCompactionSupport::V2,
            ),
            (
                ModelProviderInfo {
                    name: "Custom".to_string(),
                    base_url: Some("https://example.provider.azure.com/provider/v1".to_string()),
                    ..ModelProviderInfo::default()
                },
                RemoteCompactionSupport::V2,
            ),
            (
                provider_for("https://example.test/v1".to_string()),
                RemoteCompactionSupport::V2,
            ),
        ];

        for (provider_info, expected) in cases {
            let provider = create_model_provider(provider_info, /*auth_manager*/ None);
            assert_eq!(provider.capabilities().remote_compaction, expected);
        }
    }

    #[tokio::test]
    async fn configured_provider_runtime_base_url_uses_configured_base_url() {
        let provider = create_model_provider(
            provider_for("https://example.test/v1".to_string()),
            /*auth_manager*/ None,
        );

        assert_eq!(
            provider
                .runtime_base_url()
                .await
                .expect("runtime base URL should resolve"),
            Some("https://example.test/v1".to_string())
        );
    }

    #[test]
    fn create_model_provider_builds_command_auth_manager_without_base_manager() {
        let provider = create_model_provider(
            provider_info_with_command_auth(),
            /*auth_manager*/ None,
        );

        let auth_manager = provider
            .auth_manager()
            .expect("command auth provider should have an auth manager");

        assert!(auth_manager.has_external_auth());
    }

    #[test]
    fn responses_provider_returns_unauthenticated_provider_account_state() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*auth_manager*/ None,
        );

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                account: None,
                requires_auth: true,
            })
        );
    }

    #[test]
    fn responses_provider_returns_api_key_account_state() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            Some(AuthManager::from_auth_for_testing(GenexAuth::from_api_key(
                "provider-api-key",
            ))),
        );

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                account: Some(ProviderAccount::ApiKey),
                requires_auth: true,
            })
        );
    }

    #[test]
    fn custom_non_responses_provider_returns_no_account_state() {
        let provider = create_model_provider(
            ModelProviderInfo {
                name: "Custom".to_string(),
                base_url: Some("http://localhost:1234/v1".to_string()),
                wire_api: WireApi::Responses,
                requires_auth: false,
                ..Default::default()
            },
            /*auth_manager*/ None,
        );

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                account: None,
                requires_auth: false,
            })
        );
    }

    #[tokio::test]
    async fn configured_provider_models_manager_uses_provider_bearer_token() {
        let server = MockServer::start().await;
        let remote_models = vec![remote_model("provider-model")];

        Mock::given(method("GET"))
            .and(path("/models"))
            .and(header_regex("Authorization", "Bearer provider-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_json(ModelCatalog {
                        models: remote_models.clone(),
                    }),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut provider_info = provider_for(server.uri());
        provider_info.experimental_bearer_token = Some("provider-token".into());
        let provider = create_model_provider(
            provider_info,
            Some(AuthManager::from_auth_for_testing(GenexAuth::from_api_key(
                "ambient-api-key",
            ))),
        );

        let manager = build_models_manager(
            test_genex_home(),
            Arc::new(ProviderModelCatalogSource::new(provider.clone())),
            provider.auth_manager(),
            /*config_model_catalog*/ None,
        );
        let catalog = manager
            .raw_model_catalog(
                RefreshStrategy::Online,
                HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            )
            .await;

        assert!(
            catalog
                .models
                .iter()
                .any(|model| model.slug == "provider-model")
        );
    }
}
