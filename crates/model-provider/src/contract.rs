//! Provider-neutral boundary for Genex hosts and provider implementations.
//!
//! The runtime adapter still lives in [`crate::provider`].  This module is intentionally free of
//! credential-store and wire-client types so a provider implementation can be supplied without
//! making those implementation details part of the Genex contract.

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use futures::Stream;
use genex_http_client::HttpClientFactory;
use genex_protocol::account::{ProviderAccount, ProviderAuthStatus};
use genex_protocol::model_types::ModelCatalog;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::RateLimitSnapshot;
use genex_protocol::protocol::SessionSource;
use http::HeaderMap;
use serde_json::Value;

/// Remote context-compaction protocols supported by a model provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCompactionSupport {
    Unsupported,
    V2,
}

/// Optional provider-backed features that Genex may expose at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub model_listing: bool,
    pub api_key_model_discovery: bool,
    pub command_auth: bool,
    pub streaming: bool,
    pub websocket: bool,
    pub usage: bool,
    pub interactive_login: bool,
    pub namespace_tools: bool,
    pub web_search: bool,
    pub standalone_web_search: bool,
    pub external_web_access: bool,
    pub history_notes: bool,
    pub remote_compaction: RemoteCompactionSupport,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            model_listing: true,
            api_key_model_discovery: false,
            command_auth: false,
            streaming: true,
            websocket: false,
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

/// Provider wire protocol selected by the mounted provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderWireApi {
    Responses,
    Chat,
}

/// Stable, non-secret provider identity used by the mounted data-root marker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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

pub const CURRENT_PROVIDER_PROTOCOL_VERSION: &str = "1";

/// Result of a provider-owned usage capability.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderUsage {
    Unsupported,
    Available(RateLimitSnapshot),
}

pub type ProviderUsageResult = Result<ProviderUsage, ProviderError>;

/// Provider-neutral model catalog together with cache validation metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderModelCatalog {
    pub catalog: ModelCatalog,
    pub identity: String,
    pub etag: Option<String>,
}

/// Provider-neutral error classes exposed to a Genex host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// The provider does not implement the requested operation.
    Unsupported,
    /// The provider has no usable credentials.
    Unauthenticated,
    /// The provider rejected the supplied credentials or request authorization.
    Unauthorized,
    /// The request does not satisfy the provider's input constraints.
    InvalidRequest(String),
    /// The provider quota or rate limit was reached.
    RateLimited { retry_after_seconds: Option<u64> },
    /// A retryable provider-side failure.
    Transient(String),
    /// A connection or streaming failure.
    Transport(String),
    /// The provider requires the host to fall back from WebSocket transport.
    UpgradeRequired,
    /// A provider-specific failure without a more precise common classification.
    Provider(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("provider operation is unsupported"),
            Self::Unauthenticated => f.write_str("provider is unauthenticated"),
            Self::Unauthorized => f.write_str("provider rejected the request"),
            Self::InvalidRequest(message)
            | Self::Transient(message)
            | Self::Transport(message)
            | Self::Provider(message) => f.write_str(message),
            Self::UpgradeRequired => f.write_str("provider requires a different transport"),
            Self::RateLimited { .. } => f.write_str("provider rate limit reached"),
        }
    }
}

impl std::error::Error for ProviderError {}

pub type ProviderResult<T> = Result<T, ProviderError>;
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type ProviderChatStream = Pin<Box<dyn Stream<Item = ProviderResult<ProviderChatEvent>> + Send>>;
pub type SharedModelProvider = Arc<dyn ModelProvider>;

/// Provider-neutral telemetry event emitted by a WebSocket transport.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderWebsocketEvent {
    Message(Value),
    Ignored,
    Closed,
    Error(ProviderError),
}

/// Host telemetry sink for provider-owned WebSocket transport.
pub trait ProviderWebsocketTelemetry: Send + Sync {
    fn on_request(
        &self,
        duration: Duration,
        error: Option<&ProviderError>,
        connection_reused: bool,
    );

    fn on_event(&self, event: ProviderWebsocketEvent, duration: Duration);
}

/// Input to a provider-owned WebSocket connection.
pub struct ProviderWebsocketConnectRequest {
    pub http_client_factory: HttpClientFactory,
    pub extra_headers: HeaderMap,
    pub default_headers: HeaderMap,
    pub telemetry: Option<Arc<dyn ProviderWebsocketTelemetry>>,
}

/// A serialized provider wire request sent over a WebSocket connection.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderWebsocketRequest {
    pub payload: Value,
    pub connection_reused: bool,
    pub turn_state: Option<Arc<OnceLock<String>>>,
}

/// Provider-neutral handle for a connected WebSocket.
pub trait ProviderWebsocketConnection: fmt::Debug + Send + Sync {
    fn is_closed(&self) -> ProviderFuture<'_, ProviderResult<bool>>;

    fn stream_request(
        &self,
        request: ProviderWebsocketRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>>;
}

pub type SharedProviderWebsocketConnection = Arc<dyn ProviderWebsocketConnection>;

/// Current provider authentication state. Credential values are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAuthState {
    pub status: ProviderAuthStatus,
    pub account_id: Option<String>,
    pub display_name: Option<String>,
}

/// A provider-owned credential field exposed to a host login form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAuthField {
    pub id: String,
    pub label: String,
    pub secret: bool,
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

impl ProviderAuthField {
    pub fn new(id: impl Into<String>, label: impl Into<String>, secret: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            secret,
        }
    }
}

/// Provider-owned account and usage state suitable for host/UI projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderAccountState {
    pub provider_id: String,
    pub account: Option<ProviderAccount>,
    pub account_id: Option<String>,
    pub display_name: Option<String>,
    pub auth_status: ProviderAuthStatus,
    pub usage: Option<RateLimitSnapshot>,
}

/// Input to a provider model-catalog operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListModelsRequest {
    pub client_version: String,
    pub http_client_factory: HttpClientFactory,
}

impl ListModelsRequest {
    pub fn new(client_version: impl Into<String>, http_client_factory: HttpClientFactory) -> Self {
        Self {
            client_version: client_version.into(),
            http_client_factory,
        }
    }
}

/// Input to a provider usage operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageRequest {
    pub model: Option<String>,
}

/// Provider-neutral chat input. Wire request construction remains inside the provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub instructions: Option<String>,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<Value>,
    pub metadata: BTreeMap<String, Value>,
    pub conversation_id: Option<String>,
    pub session_source: Option<SessionSource>,
    pub payload: Option<Value>,
    pub transport: ChatTransportOptions,
}

/// Host-owned transport context passed to a provider operation.
///
/// The provider chooses the endpoint and wire protocol; the host supplies outbound networking
/// policy and request-scoped routing state without exposing its client implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTransportOptions {
    pub http_client_factory: HttpClientFactory,
    pub session_id: Option<String>,
    pub thread_id: Option<String>,
    pub extra_headers: HeaderMap,
    pub compression: ProviderRequestCompression,
    pub turn_state: Option<Arc<OnceLock<String>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRequestCompression {
    None,
    Zstd,
}

/// Provider-neutral request for the standalone web-search operation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderSearchRequest {
    pub id: String,
    pub model: String,
    pub input: Vec<ResponseItem>,
    pub commands: Value,
    pub settings: Value,
    pub max_output_tokens: Option<u64>,
    pub http_client_factory: HttpClientFactory,
    pub extra_headers: HeaderMap,
}

/// Provider-neutral standalone web-search result.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderSearchResponse {
    pub output: String,
    pub results: Option<Vec<Value>>,
}

/// Provider-neutral request for a provider-owned JSON backend operation.
///
/// The host owns feature-specific context and headers. The provider owns endpoint resolution,
/// authentication, outbound routing, and the concrete HTTP transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderBackendRequest {
    pub path: String,
    pub body: Value,
    pub headers: HeaderMap,
    pub timeout: Duration,
    pub http_client_factory: HttpClientFactory,
}

/// JSON response returned by a provider-owned backend operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderBackendResponse {
    pub body: Value,
}

/// Provider-neutral streamed chat events.
///
/// The current migration reuses the repository's normalized response event vocabulary. Wire
/// transport errors are still converted to [`ProviderError`] by the provider adapter.
pub type ProviderChatEvent = genex_api::ResponseEvent;

/// Provider-owned model execution, catalog, authentication, and usage contract.
pub trait ModelProvider: fmt::Debug + Send + Sync + 'static {
    fn descriptor(&self) -> ProviderDescriptor;

    fn wire_api(&self) -> ProviderWireApi {
        ProviderWireApi::Responses
    }

    fn websocket_connect_timeout(&self) -> Duration {
        Duration::from_secs(10)
    }

    fn connect_websocket(
        &self,
        request: ProviderWebsocketConnectRequest,
    ) -> ProviderFuture<'_, ProviderResult<SharedProviderWebsocketConnection>> {
        let _ = request;
        Box::pin(async { Err(ProviderError::Unsupported) })
    }

    /// Stable cache identity for provider-owned model catalog data.
    fn model_catalog_identity(&self) -> ProviderResult<String> {
        Ok(self.descriptor().provider_kind)
    }

    fn capabilities(&self) -> ProviderCapabilities;

    fn chat(&self, request: ChatRequest) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>>;

    fn search(
        &self,
        request: ProviderSearchRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderSearchResponse>> {
        let _ = request;
        Box::pin(async { Err(ProviderError::Unsupported) })
    }

    fn backend(
        &self,
        request: ProviderBackendRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderBackendResponse>> {
        let _ = request;
        Box::pin(async { Err(ProviderError::Unsupported) })
    }

    fn auth_recovery_messages(&self) -> Option<ProviderAuthRecoveryMessages> {
        None
    }

    fn recover_from_unauthorized(
        &self,
    ) -> ProviderFuture<'_, ProviderResult<ProviderUnauthorizedRecovery>> {
        Box::pin(async { Ok(ProviderUnauthorizedRecovery::NotConfigured) })
    }

    fn list_models(
        &self,
        request: ListModelsRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>>;

    fn auth_state(&self) -> ProviderAuthState;

    fn auth_fields(&self) -> Vec<ProviderAuthField>;

    fn account_state(&self) -> ProviderResult<ProviderAccountState>;

    fn login(
        &self,
        credentials: BTreeMap<String, String>,
    ) -> ProviderFuture<'_, ProviderResult<()>>;

    fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>>;

    fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>>;

    fn usage(&self, request: UsageRequest) -> ProviderFuture<'_, ProviderResult<ProviderUsage>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct FakeProvider;

    impl ModelProvider for FakeProvider {
        fn descriptor(&self) -> ProviderDescriptor {
            ProviderDescriptor::new("fake", "1")
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn chat(
            &self,
            _request: ChatRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn list_models(
            &self,
            _request: ListModelsRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn auth_state(&self) -> ProviderAuthState {
            ProviderAuthState {
                status: ProviderAuthStatus::Unauthenticated,
                account_id: None,
                display_name: None,
            }
        }

        fn auth_fields(&self) -> Vec<ProviderAuthField> {
            vec![ProviderAuthField::new("api_key", "API key", true)]
        }

        fn account_state(&self) -> ProviderResult<ProviderAccountState> {
            Ok(ProviderAccountState {
                provider_id: "fake".to_string(),
                account: None,
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Unauthenticated,
                usage: None,
            })
        }

        fn login(
            &self,
            _credentials: BTreeMap<String, String>,
        ) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Ok(()) })
        }

        fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Ok(()) })
        }

        fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>> {
            Box::pin(async { Ok(false) })
        }

        fn usage(
            &self,
            _request: UsageRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderUsage>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }
    }

    #[test]
    fn error_classification_is_explicit() {
        assert_eq!(
            ProviderError::Unsupported.to_string(),
            "provider operation is unsupported"
        );
        assert!(matches!(
            ProviderError::RateLimited {
                retry_after_seconds: Some(5)
            },
            ProviderError::RateLimited { .. }
        ));
    }

    #[test]
    fn contract_is_object_safe_for_host_injection() {
        let provider: SharedModelProvider = Arc::new(FakeProvider);
        assert_eq!(provider.descriptor().provider_kind, "fake");
        assert_eq!(
            provider.auth_state().status,
            ProviderAuthStatus::Unauthenticated
        );
    }
}
