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
use genex_login::CredentialManager;
use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_model_provider_info::ProviderAuthField;
use genex_model_provider_info::WireApi;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_protocol::account::ProviderAccount;
use genex_protocol::account::ProviderAuthStatus;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result as CoreResult;
use genex_protocol::protocol::RateLimitSnapshot;
use serde::Deserialize;
use serde::Serialize;

use crate::auth::ProviderAuthScope;
use crate::auth::ResolvedProviderAuth;
use crate::auth::cached_provider_auth;
use crate::auth::credential_manager_for_provider;
use crate::auth::provider_auth;
use crate::auth::resolve_provider_auth;
use crate::auth::resolve_provider_auth_for_scope;
use crate::cloudflare_provider::CloudflareModelProvider;
use crate::models_endpoint::ModelsEndpoint;

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
    /// Whether the provider can enumerate or otherwise expose its model catalog.
    pub model_listing: bool,
    /// Whether the provider can return a streamed chat/response result.
    pub streaming: bool,
    /// Whether the provider exposes a usage or quota operation.
    pub usage: bool,
    /// Whether the provider exposes an interactive login flow.
    pub interactive_login: bool,
    pub namespace_tools: bool,
    pub web_search: bool,
    /// Whether the provider exposes a standalone web-search backend.
    pub standalone_web_search: bool,
    pub external_web_access: bool,
    /// Whether the provider exposes the managed History/Notes backend.
    pub history_notes: bool,
    pub remote_compaction: RemoteCompactionSupport,
}

/// Stable, non-secret identity and compatibility information for a provider runtime.
///
/// This is the contract that will be persisted in the data-root mount marker. It deliberately
/// excludes endpoints, model settings, and credentials so changing an account or API key within
/// the same provider does not invalidate the data root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub provider_kind: String,
    pub provider_protocol_version: String,
}

impl ProviderDescriptor {
    pub fn new(
        provider_kind: impl Into<String>,
        provider_protocol_version: impl Into<String>,
    ) -> Self {
        Self {
            provider_kind: provider_kind.into(),
            provider_protocol_version: provider_protocol_version.into(),
        }
    }
}

/// Version of the provider contract represented by the current data-root marker.
pub const CURRENT_PROVIDER_PROTOCOL_VERSION: &str = "1";

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            model_listing: true,
            streaming: true,
            usage: false,
            interactive_login: false,
            namespace_tools: true,
            web_search: true,
            standalone_web_search: false,
            external_web_access: true,
            history_notes: false,
            remote_compaction: RemoteCompactionSupport::Unsupported,
        }
    }
}

/// Current app-visible account state for a model provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderAccountState {
    pub provider_id: String,
    pub account: Option<ProviderAccount>,
    pub account_id: Option<String>,
    pub display_name: Option<String>,
    pub auth_status: ProviderAuthStatus,
    pub usage: Option<RateLimitSnapshot>,
}

/// Result of a provider-owned usage capability.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderUsage {
    Unsupported,
    Available(RateLimitSnapshot),
}

pub type ProviderUsageResult = std::result::Result<ProviderUsage, ProviderAccountError>;

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

    /// Returns the stable provider identity used to validate a data-root mount.
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor::new(
            self.provider_id().unwrap_or(self.info().name.as_str()),
            CURRENT_PROVIDER_PROTOCOL_VERSION,
        )
    }

    /// Returns the credential values requested by the interactive setup flow.
    fn auth_fields(&self) -> Vec<ProviderAuthField> {
        if self.info().auth.is_some() || self.info().experimental_bearer_token.is_some() {
            Vec::new()
        } else {
            vec![ProviderAuthField::new("api_key", "API key", true)]
        }
    }

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
        let endpoint = ModelsEndpoint::new_for_provider(
            self.provider_id().map(str::to_owned),
            self.info().clone(),
            self.credential_manager(),
        );
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
    fn credential_manager(&self) -> Option<Arc<CredentialManager>>;

    /// Returns the configured provider ID used for provider-scoped credentials.
    fn provider_id(&self) -> Option<&str> {
        None
    }

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

    /// Returns provider-owned usage data when the provider exposes such a capability.
    fn usage(&self) -> ModelProviderFuture<'_, ProviderUsageResult> {
        Box::pin(async { Ok(ProviderUsage::Unsupported) })
    }

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
            resolve_provider_auth_for_scope(
                self.credential_manager(),
                auth.as_ref(),
                self.info(),
                self.provider_id(),
                scope,
            )
            .await
        })
    }
}

pub type ModelProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Shared runtime model provider handle.
pub type SharedModelProvider = Arc<dyn ModelProvider>;

fn provider_uses_stored_auth_path(provider: &ModelProviderInfo) -> bool {
    provider.env_key.is_none()
        && provider.experimental_bearer_token.is_none()
        && provider.auth.is_none()
}

/// Creates the default runtime model provider for configured provider metadata.
pub fn create_model_provider(
    provider_info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
) -> SharedModelProvider {
    let provider_id = provider_info.id.clone();
    create_model_provider_internal(provider_id, provider_info, credential_manager)
}

fn create_model_provider_internal(
    provider_id: Option<String>,
    provider_info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
) -> SharedModelProvider {
    match provider_info.name.as_str() {
        genex_model_provider_info::CLOUDFLARE_PROVIDER_NAME => Arc::new(
            CloudflareModelProvider::new(provider_id, provider_info, credential_manager),
        ),
        _ => Arc::new(ConfiguredModelProvider::new(
            provider_id,
            provider_info,
            credential_manager,
        )),
    }
}

/// Runtime model provider backed by configured `ModelProviderInfo`.
#[derive(Clone, Debug)]
struct ConfiguredModelProvider {
    provider_id: Option<String>,
    info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
}

impl ConfiguredModelProvider {
    fn new(
        provider_id: Option<String>,
        provider_info: ModelProviderInfo,
        credential_manager: Option<Arc<CredentialManager>>,
    ) -> Self {
        let credential_manager =
            credential_manager_for_provider(credential_manager, &provider_info);
        Self {
            provider_id,
            info: provider_info,
            credential_manager,
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
            history_notes: self.info.is_responses_provider(),
            standalone_web_search: self.info.supports_standalone_web_search,
            remote_compaction,
            ..ProviderCapabilities::default()
        }
    }

    fn credential_manager(&self) -> Option<Arc<CredentialManager>> {
        self.credential_manager.clone()
    }

    fn provider_id(&self) -> Option<&str> {
        self.provider_id.as_deref()
    }

    fn supports_attestation(&self) -> bool {
        false
    }

    fn auth(&self) -> ModelProviderFuture<'_, Option<GenexAuth>> {
        Box::pin(async move {
            provider_auth(
                self.credential_manager.as_ref(),
                None,
                &self.info,
                self.provider_id(),
            )
            .await
        })
    }

    fn account_state(&self) -> ProviderAccountResult {
        let auth = self
            .credential_manager
            .as_ref()
            .and_then(|credential_manager| {
                cached_provider_auth(credential_manager, self.provider_id())
            });
        let auth_invalid = auth.as_ref().is_some_and(|auth| {
            self.credential_manager
                .as_ref()
                .is_some_and(|credential_manager| {
                    credential_manager.refresh_failure_for_auth(auth).is_some()
                })
        });
        let configured_auth =
            self.info.experimental_bearer_token.is_some() || self.info.api_key().is_ok();
        let account = if auth_invalid {
            None
        } else if auth.is_some() || configured_auth {
            Some(ProviderAccount::ApiKey)
        } else {
            None
        };
        let auth_status = if auth_invalid {
            ProviderAuthStatus::Invalid
        } else if account.is_some() {
            ProviderAuthStatus::Authenticated
        } else {
            ProviderAuthStatus::Unauthenticated
        };

        Ok(ProviderAccountState {
            provider_id: self
                .provider_id()
                .unwrap_or(self.info.name.as_str())
                .to_string(),
            account,
            account_id: self
                .credential_manager
                .as_ref()
                .and_then(|credential_manager| {
                    cached_provider_auth(credential_manager, self.provider_id())
                        .and_then(|auth| auth.get_account_id())
                }),
            display_name: None,
            auth_status,
            usage: None,
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
    fn cloudflare_provider_selects_chat_runtime_api() {
        let provider = create_model_provider(
            ModelProviderInfo::create_cloudflare_provider(),
            /*credential_manager*/ None,
        );

        assert!(matches!(provider.api(), ProviderApi::Chat(_)));
    }

    #[test]
    fn cloudflare_provider_declares_api_key_and_account_id_auth_fields() {
        let provider = create_model_provider(
            ModelProviderInfo::create_cloudflare_provider(),
            /*credential_manager*/ None,
        );

        assert_eq!(
            provider
                .auth_fields()
                .into_iter()
                .map(|field| field.id)
                .collect::<Vec<_>>(),
            vec!["api_key", "account_id"]
        );
    }

    #[test]
    fn provider_descriptor_uses_explicit_provider_id_and_round_trips() {
        let provider = create_model_provider(
            ModelProviderInfo {
                id: Some("account-a".to_string()),
                ..ModelProviderInfo::create_responses_provider(None)
            },
            /*credential_manager*/ None,
        );

        let descriptor = provider.descriptor();
        assert_eq!(descriptor.provider_kind, "account-a");
        assert_eq!(
            descriptor.provider_protocol_version,
            CURRENT_PROVIDER_PROTOCOL_VERSION
        );

        let serialized = serde_json::to_string(&descriptor).expect("descriptor should serialize");
        let decoded: ProviderDescriptor =
            serde_json::from_str(&serialized).expect("descriptor should deserialize");
        assert_eq!(decoded, descriptor);
    }

    #[test]
    fn provider_descriptor_falls_back_to_provider_name_without_id() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        assert_eq!(
            provider.descriptor(),
            ProviderDescriptor {
                provider_kind: "Responses provider".to_string(),
                provider_protocol_version: CURRENT_PROVIDER_PROTOCOL_VERSION.to_string(),
            }
        );
    }

    #[test]
    fn provider_capabilities_report_common_operation_support() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        let capabilities = provider.capabilities();
        assert!(capabilities.model_listing);
        assert!(capabilities.streaming);
        assert!(!capabilities.usage);
        assert!(!capabilities.interactive_login);
    }

    #[test]
    fn responses_provider_declares_history_notes_capability() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        assert!(provider.capabilities().history_notes);
    }

    #[test]
    fn configured_provider_reports_standalone_web_search_capability() {
        let provider = create_model_provider(
            ModelProviderInfo {
                supports_standalone_web_search: true,
                ..ModelProviderInfo::default()
            },
            /*credential_manager*/ None,
        );

        assert!(provider.capabilities().standalone_web_search);
    }

    #[test]
    fn provider_model_catalog_source_preserves_provider_capabilities() {
        let mut info = ModelProviderInfo::create_cloudflare_provider();
        info.base_url = Some("https://example.test".to_string());
        info.env_key = None;
        let provider = create_model_provider(info, /*credential_manager*/ None);
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
            .and(wiremock::matchers::query_param("task", "Text Generation"))
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
        let provider = create_model_provider(info, /*credential_manager*/ None);
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
            /*credential_manager*/ None,
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
            ..ModelProviderInfo::create_responses_provider(/*base_url*/ None)
        }
    }

    fn test_genex_home() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("genex-model-provider-test-{}", std::process::id()))
    }

    fn provider_for(base_url: String) -> ModelProviderInfo {
        ModelProviderInfo {
            id: None,
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
            /*credential_manager*/ None,
        );

        let auth = provider
            .api_auth_for_scope(ProviderAuthScope)
            .await
            .expect("auth should resolve");

        assert!(auth.auth.to_auth_headers().is_empty());
    }

    #[tokio::test]
    async fn provider_auth_does_not_fall_back_to_another_provider() {
        let home = tempfile::tempdir().expect("temporary auth home");
        let credential_manager = Arc::new(
            CredentialManager::new(
                home.path().to_path_buf(),
                false,
                genex_login::AuthCredentialsStoreMode::File,
                None,
                None,
                genex_login::AuthKeyringBackendKind::default(),
                genex_login::AuthRouteConfig::from_http_client_factory(HttpClientFactory::new(
                    OutboundProxyPolicy::ReqwestDefault,
                )),
            )
            .await,
        );
        credential_manager
            .login("provider-a", genex_login::ProviderAuth::api_key("a-secret"))
            .await
            .expect("provider A login");

        let provider_a = create_model_provider(
            ModelProviderInfo {
                id: Some("provider-a".to_string()),
                ..ModelProviderInfo::create_responses_provider(None)
            },
            Some(Arc::clone(&credential_manager)),
        );
        let provider_b = create_model_provider(
            ModelProviderInfo {
                id: Some("provider-b".to_string()),
                ..ModelProviderInfo::create_responses_provider(None)
            },
            Some(credential_manager),
        );

        assert_eq!(
            provider_a
                .api_auth()
                .await
                .expect("provider A auth")
                .to_auth_headers()
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer a-secret")
        );
        assert!(
            provider_b
                .api_auth()
                .await
                .expect("provider B auth")
                .to_auth_headers()
                .is_empty()
        );
    }

    #[test]
    fn responses_provider_enables_remote_compaction() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*credential_manager*/ None,
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
            let provider = create_model_provider(provider_info, /*credential_manager*/ None);
            assert_eq!(provider.capabilities().remote_compaction, expected);
        }
    }

    #[tokio::test]
    async fn configured_provider_runtime_base_url_uses_configured_base_url() {
        let provider = create_model_provider(
            provider_for("https://example.test/v1".to_string()),
            /*credential_manager*/ None,
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
    fn create_model_provider_builds_command_credential_manager_without_base_manager() {
        let provider = create_model_provider(
            provider_info_with_command_auth(),
            /*credential_manager*/ None,
        );

        let credential_manager = provider
            .credential_manager()
            .expect("command auth provider should have an auth manager");

        assert!(credential_manager.has_external_auth());
    }

    #[test]
    fn responses_provider_returns_unauthenticated_provider_account_state() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*credential_manager*/ None,
        );

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                provider_id: "Responses provider".to_string(),
                account: None,
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Unauthenticated,
                usage: None,
            })
        );
    }

    #[test]
    fn responses_provider_returns_api_key_account_state() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            Some(CredentialManager::from_auth_for_testing(
                GenexAuth::from_api_key("provider-api-key"),
            )),
        );

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                provider_id: "Responses provider".to_string(),
                account: Some(ProviderAccount::ApiKey),
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Authenticated,
                usage: None,
            })
        );
    }

    #[tokio::test]
    async fn configured_provider_usage_is_unsupported_by_default() {
        let provider = create_model_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*credential_manager*/ None,
        );

        assert_eq!(provider.usage().await, Ok(ProviderUsage::Unsupported));
    }

    #[test]
    fn custom_non_responses_provider_returns_no_account_state() {
        let provider = create_model_provider(
            ModelProviderInfo {
                name: "Custom".to_string(),
                base_url: Some("http://localhost:1234/v1".to_string()),
                wire_api: WireApi::Responses,
                ..Default::default()
            },
            /*credential_manager*/ None,
        );

        assert_eq!(
            provider.account_state(),
            Ok(ProviderAccountState {
                provider_id: "Custom".to_string(),
                account: None,
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Unauthenticated,
                usage: None,
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
            Some(CredentialManager::from_auth_for_testing(
                GenexAuth::from_api_key("ambient-api-key"),
            )),
        );

        let manager = build_models_manager(
            test_genex_home(),
            Arc::new(ProviderModelCatalogSource::new(provider.clone())),
            provider.credential_manager(),
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
