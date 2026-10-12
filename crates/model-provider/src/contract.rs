//! Provider-neutral boundary for Genex hosts and provider implementations.
//!
//! The request, response, stream, capability, descriptor, authentication, and usage surface is
//! provider-neutral. Provider-specific HTTP, credentials, wire payloads, and error details stay
//! inside the adapter implementation.

use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use futures::Stream;
use genex_protocol::ResponseUsageMetadata;
use genex_protocol::account::{ProviderAccount, ProviderAuthStatus};
use genex_protocol::config_types::ReasoningSummary;
use genex_protocol::config_types::Verbosity;
use genex_protocol::model_types::ModelCatalog;
use genex_protocol::model_types::ReasoningEffort;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::ModelVerification;
use genex_protocol::protocol::RateLimitSnapshot;
use genex_protocol::protocol::SessionSource;
use genex_protocol::protocol::TokenUsage;
use http::HeaderMap;
use http::HeaderName;
use http::HeaderValue;
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

/// Provider request format selected by the mounted provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRequestFormat {
    Responses,
    Chat,
}

/// Internal provider-implementation representation of the runtime wire implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderWireApi {
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

pub(crate) fn headers_to_http(headers: BTreeMap<String, String>) -> ProviderResult<HeaderMap> {
    let mut result = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
            ProviderError::InvalidRequest(format!("invalid provider header name: {error}"))
        })?;
        let value = HeaderValue::from_str(&value).map_err(|error| {
            ProviderError::InvalidRequest(format!("invalid provider header value: {error}"))
        })?;
        result.insert(name, value);
    }
    Ok(result)
}

/// Provider-neutral telemetry event emitted by a provider-owned streaming transport.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderStreamEvent {
    Message(Value),
    Ignored,
    Closed,
    Error(ProviderError),
}

/// Host telemetry sink for provider-owned streaming transport.
pub trait ProviderStreamTelemetry: Send + Sync {
    fn on_request(
        &self,
        duration: Duration,
        error: Option<&ProviderError>,
        connection_reused: bool,
    );

    fn on_event(&self, event: ProviderStreamEvent, duration: Duration);
}

/// Provider-neutral telemetry event emitted by a WebSocket transport.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProviderWebsocketEvent {
    Message(Value),
    Ignored,
    Closed,
    Error(ProviderError),
}

/// Host telemetry sink for provider-owned WebSocket transport.
pub(crate) trait ProviderWebsocketTelemetry: Send + Sync {
    fn on_request(
        &self,
        duration: Duration,
        error: Option<&ProviderError>,
        connection_reused: bool,
    );

    fn on_event(&self, event: ProviderWebsocketEvent, duration: Duration);
}

/// Input to a provider-owned WebSocket connection.
pub(crate) struct ProviderWebsocketConnectRequest {
    pub session_id: Option<String>,
    pub thread_id: Option<String>,
    pub extra_headers: BTreeMap<String, String>,
    pub default_headers: BTreeMap<String, String>,
    /// Opaque credentials captured when the owning turn was admitted.
    pub credential_snapshot: Option<SharedProviderCredentialSnapshot>,
    pub telemetry: Option<Arc<dyn ProviderWebsocketTelemetry>>,
}

/// A serialized provider wire request sent over a WebSocket connection.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProviderWebsocketRequest {
    pub payload: Value,
    pub connection_reused: bool,
    pub turn_state: Option<Arc<OnceLock<String>>>,
}

/// Provider-neutral handle for a connected WebSocket.
pub(crate) trait ProviderWebsocketConnection: fmt::Debug + Send + Sync {
    fn is_closed(&self) -> ProviderFuture<'_, ProviderResult<bool>>;

    fn stream_request(
        &self,
        request: ProviderWebsocketRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>>;
}

pub(crate) type SharedProviderWebsocketConnection = Arc<dyn ProviderWebsocketConnection>;

/// Current provider authentication state. Credential values are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAuthState {
    pub status: ProviderAuthStatus,
    pub account_id: Option<String>,
    pub display_name: Option<String>,
}

/// Auth metadata captured at the start of a provider operation.
///
/// This deliberately contains no credential value or provider-specific auth object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRequestAuth {
    pub authenticated: bool,
    pub managed_auth: bool,
    pub auth_header_attached: bool,
    pub auth_header_name: Option<&'static str>,
    pub revision: Option<u64>,
    pub owner_generation: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConnectionKey {
    pub provider_kind: String,
    pub auth_revision: Option<u64>,
}

/// Opaque provider-owned credentials captured for one request or turn.
///
/// Hosts can carry this handle across the provider-neutral boundary but cannot inspect or
/// persist its contents. Provider adapters downcast it only inside their own crate.
pub trait ProviderCredentialSnapshot: fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

pub type SharedProviderCredentialSnapshot = Arc<dyn ProviderCredentialSnapshot>;

/// Provider-owned request admission and connection-reuse snapshot.
#[derive(Debug, Clone)]
pub struct ProviderRequestSnapshot {
    pub auth: ProviderRequestAuth,
    pub connection_key: ProviderConnectionKey,
    pub credential_snapshot: SharedProviderCredentialSnapshot,
}

impl PartialEq for ProviderRequestSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.auth == other.auth && self.connection_key == other.connection_key
    }
}

impl Eq for ProviderRequestSnapshot {}

/// Provider-neutral transport options for one chat request.
#[derive(Debug, Clone)]
pub struct ChatTransportOptions {
    pub session_id: Option<String>,
    pub thread_id: Option<String>,
    pub extra_headers: BTreeMap<String, String>,
    pub compression: ProviderRequestCompression,
    pub turn_state: Option<Arc<OnceLock<String>>>,
    pub connection_key: Option<ProviderConnectionKey>,
    /// Opaque credentials captured when the owning turn was admitted.
    pub credential_snapshot: Option<SharedProviderCredentialSnapshot>,
}

impl PartialEq for ChatTransportOptions {
    fn eq(&self, other: &Self) -> bool {
        self.session_id == other.session_id
            && self.thread_id == other.thread_id
            && self.extra_headers == other.extra_headers
            && self.compression == other.compression
            && self.connection_key == other.connection_key
            && self.turn_state == other.turn_state
    }
}

impl Eq for ChatTransportOptions {}

/// Non-secret environment metadata exposed by the mounted provider for host telemetry.
///
/// The host must not inspect provider configuration fields to derive this information. The
/// adapter owns the provider-specific environment name and only returns presence/configuration
/// booleans.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderAuthEnvironment {
    pub host_api_key_env_present: bool,
    pub host_api_key_env_enabled: bool,
    pub provider_env_key_configured: bool,
    pub provider_env_key_present: Option<bool>,
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
}

impl ListModelsRequest {
    pub fn new(client_version: impl Into<String>) -> Self {
        Self {
            client_version: client_version.into(),
        }
    }
}

/// Input to a provider usage operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageRequest {
    pub model: Option<String>,
}

/// Provider-neutral chat input. Wire request construction remains inside the provider.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub instructions: String,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<Value>,
    pub metadata: BTreeMap<String, Value>,
    pub conversation_id: Option<String>,
    pub session_source: Option<SessionSource>,
    pub tool_choice: String,
    pub parallel_tool_calls: bool,
    pub reasoning: Option<ProviderReasoning>,
    pub store: bool,
    pub stream: bool,
    pub stream_options: Option<ProviderStreamOptions>,
    pub include: Vec<String>,
    pub service_tier: Option<String>,
    pub prompt_cache_key: Option<String>,
    pub text: Option<ProviderTextControls>,
    pub client_metadata: Option<BTreeMap<String, String>>,
    pub access_programs: Option<Value>,
    #[serde(skip)]
    pub transport: ChatTransportOptions,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProviderTextControls {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<Verbosity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ProviderTextFormat>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProviderTextFormat {
    pub r#type: ProviderTextFormatType,
    pub strict: bool,
    pub schema: Value,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTextFormatType {
    JsonSchema,
}

pub fn build_provider_text_controls(
    verbosity: Option<Verbosity>,
    output_schema: &Option<Value>,
    output_schema_strict: bool,
) -> Option<ProviderTextControls> {
    if verbosity.is_none() && output_schema.is_none() {
        return None;
    }

    Some(ProviderTextControls {
        verbosity,
        format: output_schema.as_ref().map(|schema| ProviderTextFormat {
            r#type: ProviderTextFormatType::JsonSchema,
            strict: output_schema_strict,
            schema: schema.clone(),
            name: "genex_output_schema".to_string(),
        }),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderReasoning {
    pub effort: Option<ReasoningEffort>,
    pub summary: Option<ReasoningSummary>,
    pub context: Option<ProviderReasoningContext>,
}

impl serde::Serialize for ProviderReasoning {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        if let Some(effort) = &self.effort {
            map.serialize_entry("effort", effort.as_str())?;
        }
        if let Some(summary) = &self.summary {
            map.serialize_entry("summary", summary)?;
        }
        if let Some(context) = &self.context {
            map.serialize_entry("context", context)?;
        }
        map.end()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderReasoningContext {
    AllTurns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderReasoningSummaryDelivery {
    SequentialCutoff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct ProviderStreamOptions {
    pub reasoning_summary_delivery: ProviderReasoningSummaryDelivery,
}

/// Input used by the provider implementation to construct a WebSocket wire request.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProviderWebsocketPayloadRequest {
    pub request: ChatRequest,
    pub previous_response_id: Option<String>,
    pub input: Vec<ResponseItem>,
    pub generate: Option<bool>,
    pub client_metadata: Option<BTreeMap<String, String>>,
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
    pub extra_headers: BTreeMap<String, String>,
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
    pub headers: BTreeMap<String, String>,
    pub timeout: Duration,
}

/// JSON response returned by a provider-owned backend operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderBackendResponse {
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSafetyBuffering {
    pub use_cases: Vec<String>,
    pub reasons: Vec<String>,
    pub show_buffering_ui: bool,
    pub faster_model: Option<String>,
}

/// Provider-neutral streamed chat events.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderChatEvent {
    Created {
        response_id: Option<String>,
    },
    SafetyBuffering(ProviderSafetyBuffering),
    OutputItemDone(ResponseItem),
    OutputItemAdded(ResponseItem),
    ServerModel(String),
    ModelVerifications(Vec<ModelVerification>),
    ServerReasoningIncluded(bool),
    Completed {
        response_id: String,
        token_usage: Option<TokenUsage>,
        usage_metadata: Option<ResponseUsageMetadata>,
        end_turn: Option<bool>,
    },
    OutputTextDelta(String),
    ToolCallInputDelta {
        item_id: String,
        call_id: Option<String>,
        delta: String,
    },
    ReasoningSummaryDelta {
        delta: String,
        summary_index: i64,
    },
    ReasoningSummaryDone {
        item_id: String,
        text: String,
        summary_index: i64,
    },
    ReasoningContentDelta {
        delta: String,
        content_index: i64,
    },
    ReasoningSummaryPartAdded {
        summary_index: i64,
    },
    RateLimits(RateLimitSnapshot),
    ModelsEtag(String),
}

/// Neutral connection setup for a provider-owned streaming session.
#[derive(Clone)]
pub struct ProviderStreamConnectRequest {
    pub session_id: Option<String>,
    pub thread_id: Option<String>,
    pub extra_headers: BTreeMap<String, String>,
    pub default_headers: BTreeMap<String, String>,
    pub connection_key: ProviderConnectionKey,
    pub credential_snapshot: SharedProviderCredentialSnapshot,
    pub telemetry: Option<Arc<dyn ProviderStreamTelemetry>>,
}

/// Neutral continuation state passed to a provider-owned streaming session.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderStreamContinuation {
    pub previous_response_id: Option<String>,
    pub input: Vec<ResponseItem>,
    pub generate: Option<bool>,
    pub client_metadata: Option<BTreeMap<String, String>>,
}

/// Neutral model-stream request. Wire payload construction remains inside the provider implementation.
#[derive(Clone)]
pub struct ProviderStreamRequest {
    pub request: ChatRequest,
    pub continuation: Option<ProviderStreamContinuation>,
    pub default_headers: BTreeMap<String, String>,
    pub telemetry: Option<Arc<dyn ProviderStreamTelemetry>>,
}

/// Result of a provider-owned persistent stream attempt.
pub enum ProviderStreamOutcome {
    Stream(ProviderChatStream),
    FallbackToHttp,
}

/// Provider-owned streaming session. Implementations own connection reuse and wire payloads.
pub trait ProviderStreamSession: fmt::Debug + Send + Sync {
    fn preconnect(
        &self,
        request: ProviderStreamConnectRequest,
    ) -> ProviderFuture<'_, ProviderResult<()>>;

    fn stream_request(
        &self,
        request: ProviderStreamRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderStreamOutcome>>;
}

pub type SharedProviderStreamSession = Arc<dyn ProviderStreamSession>;

impl From<genex_api::ResponseEvent> for ProviderChatEvent {
    fn from(event: genex_api::ResponseEvent) -> Self {
        match event {
            genex_api::ResponseEvent::Created { response_id } => Self::Created { response_id },
            genex_api::ResponseEvent::SafetyBuffering(buffering) => {
                Self::SafetyBuffering(ProviderSafetyBuffering {
                    use_cases: buffering.use_cases,
                    reasons: buffering.reasons,
                    show_buffering_ui: buffering.show_buffering_ui,
                    faster_model: buffering.faster_model,
                })
            }
            genex_api::ResponseEvent::OutputItemDone(item) => Self::OutputItemDone(item),
            genex_api::ResponseEvent::OutputItemAdded(item) => Self::OutputItemAdded(item),
            genex_api::ResponseEvent::ServerModel(model) => Self::ServerModel(model),
            genex_api::ResponseEvent::ModelVerifications(verifications) => {
                Self::ModelVerifications(verifications)
            }
            genex_api::ResponseEvent::ServerReasoningIncluded(included) => {
                Self::ServerReasoningIncluded(included)
            }
            genex_api::ResponseEvent::Completed {
                response_id,
                token_usage,
                usage_metadata,
                end_turn,
            } => Self::Completed {
                response_id,
                token_usage,
                usage_metadata,
                end_turn,
            },
            genex_api::ResponseEvent::OutputTextDelta(delta) => Self::OutputTextDelta(delta),
            genex_api::ResponseEvent::ToolCallInputDelta {
                item_id,
                call_id,
                delta,
            } => Self::ToolCallInputDelta {
                item_id,
                call_id,
                delta,
            },
            genex_api::ResponseEvent::ReasoningSummaryDelta {
                delta,
                summary_index,
            } => Self::ReasoningSummaryDelta {
                delta,
                summary_index,
            },
            genex_api::ResponseEvent::ReasoningSummaryDone {
                item_id,
                text,
                summary_index,
            } => Self::ReasoningSummaryDone {
                item_id,
                text,
                summary_index,
            },
            genex_api::ResponseEvent::ReasoningContentDelta {
                delta,
                content_index,
            } => Self::ReasoningContentDelta {
                delta,
                content_index,
            },
            genex_api::ResponseEvent::ReasoningSummaryPartAdded { summary_index } => {
                Self::ReasoningSummaryPartAdded { summary_index }
            }
            genex_api::ResponseEvent::RateLimits(snapshot) => Self::RateLimits(snapshot),
            genex_api::ResponseEvent::ModelsEtag(etag) => Self::ModelsEtag(etag),
        }
    }
}

/// Provider-owned model execution, catalog, authentication, and usage contract.
pub trait ModelProvider: fmt::Debug + Send + Sync + 'static {
    fn descriptor(&self) -> ProviderDescriptor;

    fn request_format(&self) -> ProviderRequestFormat {
        ProviderRequestFormat::Responses
    }

    fn websocket_connect_timeout(&self) -> Duration {
        Duration::from_secs(10)
    }

    /// Maximum number of provider stream reconnection attempts for a turn.
    fn stream_max_retries(&self) -> u64 {
        5
    }

    /// Stable cache identity for provider-owned model catalog data.
    fn model_catalog_identity(&self) -> ProviderResult<String> {
        Ok(self.descriptor().provider_kind)
    }

    fn capabilities(&self) -> ProviderCapabilities;

    /// Returns whether this provider includes attestation on model requests.
    fn supports_attestation(&self) -> bool {
        false
    }

    fn chat(&self, request: ChatRequest) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>>;

    /// Opens a provider-owned persistent streaming session when supported.
    fn open_stream_session(
        &self,
    ) -> ProviderFuture<'_, ProviderResult<SharedProviderStreamSession>> {
        Box::pin(async { Err(ProviderError::Unsupported) })
    }

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

    fn auth_environment(&self) -> ProviderAuthEnvironment {
        ProviderAuthEnvironment {
            host_api_key_env_enabled: false,
            ..ProviderAuthEnvironment::default()
        }
    }

    fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>>;

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
    struct FakeCredentialSnapshot;

    impl ProviderCredentialSnapshot for FakeCredentialSnapshot {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

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

        fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>> {
            Box::pin(async {
                Ok(ProviderRequestSnapshot {
                    auth: ProviderRequestAuth {
                        authenticated: false,
                        managed_auth: false,
                        auth_header_attached: false,
                        auth_header_name: None,
                        revision: None,
                        owner_generation: None,
                    },
                    connection_key: ProviderConnectionKey {
                        provider_kind: "fake".to_string(),
                        auth_revision: None,
                    },
                    credential_snapshot: Arc::new(FakeCredentialSnapshot),
                })
            })
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

    #[test]
    fn fake_provider_exposes_neutral_auth_environment_metadata() {
        let environment = FakeProvider.auth_environment();

        assert!(!environment.host_api_key_env_enabled);
        assert!(!environment.host_api_key_env_present);
        assert!(!environment.provider_env_key_configured);
        assert_eq!(environment.provider_env_key_present, None);
    }

    #[tokio::test]
    async fn provider_request_snapshot_is_neutral_and_explicitly_unauthenticated() {
        let provider: SharedModelProvider = Arc::new(FakeProvider);
        let snapshot = provider
            .request_snapshot()
            .await
            .expect("fake provider should expose a request snapshot");
        assert!(!snapshot.auth.authenticated);
        assert!(!snapshot.auth.managed_auth);
        assert_eq!(snapshot.connection_key.provider_kind, "fake");
        assert!(matches!(
            ProviderError::Unauthenticated,
            ProviderError::Unauthenticated
        ));
    }

    #[tokio::test]
    async fn unsupported_stream_session_is_explicit() {
        let error = FakeProvider
            .open_stream_session()
            .await
            .expect_err("fake provider should not silently provide streaming transport");
        assert!(matches!(error, ProviderError::Unsupported));
    }

    #[test]
    fn provider_stream_events_are_owned_by_the_provider_contract() {
        let event = ProviderChatEvent::OutputTextDelta("delta".to_string());
        assert!(matches!(event, ProviderChatEvent::OutputTextDelta(_)));
    }

    #[test]
    fn public_provider_requests_use_neutral_transport_metadata() {
        let mut headers = BTreeMap::new();
        headers.insert("x-test-header".to_string(), "value".to_string());

        let _chat = ChatTransportOptions {
            session_id: Some("session".to_string()),
            thread_id: Some("thread".to_string()),
            extra_headers: headers.clone(),
            compression: ProviderRequestCompression::None,
            turn_state: None,
            connection_key: None,
            credential_snapshot: None,
        };
        let _search = ProviderSearchRequest {
            id: "search".to_string(),
            model: "model".to_string(),
            input: Vec::new(),
            commands: serde_json::json!({}),
            settings: serde_json::json!({}),
            max_output_tokens: None,
            extra_headers: headers.clone(),
        };
        let _backend = ProviderBackendRequest {
            path: "backend".to_string(),
            body: serde_json::json!({}),
            headers,
            timeout: Duration::from_secs(1),
        };
        let _models = ListModelsRequest::new("client");
        let _websocket = ProviderWebsocketConnectRequest {
            session_id: None,
            thread_id: None,
            extra_headers: BTreeMap::new(),
            default_headers: BTreeMap::new(),
            credential_snapshot: None,
            telemetry: None,
        };
    }
}
