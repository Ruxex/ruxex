use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use ruxex_api::ApiError;
use ruxex_api::ChatClient;
use ruxex_api::ChatCompletionApi;
use ruxex_api::ChatRequestInput;
use ruxex_api::Provider;
use ruxex_api::ResponsesClient;
use ruxex_api::ResponsesWebsocketClient;
use ruxex_api::ResponsesWebsocketConnection;
use ruxex_api::SearchClient;
use ruxex_api::SearchCommands;
use ruxex_api::SearchInput;
use ruxex_api::SearchRequest;
use ruxex_api::SearchSettings;
use ruxex_api::SharedAuthProvider;
use ruxex_api::StandardChatCompletionApi;
use ruxex_api::TransportError;
use ruxex_api::WebsocketTelemetry;
use ruxex_api::auth_header_telemetry;
use ruxex_api::is_azure_responses_provider;
use ruxex_http_client::ClientRouteClass;
use ruxex_http_client::HttpClientFactory;
use ruxex_http_client::HttpTransport;
use ruxex_http_client::OutboundProxyPolicy;
use ruxex_http_client::RequestBody;
use ruxex_login::CredentialManager;
use ruxex_login::ProviderAuth;
use ruxex_login::RUXEX_API_KEY_ENV_VAR;
use ruxex_login::RuxexAuth;
use ruxex_login::default_client::ClientRedirectPolicy;
use ruxex_login::default_client::create_client_for_route;
use ruxex_model_provider_info::ModelProviderInfo;
use ruxex_model_provider_info::WireApi;
use ruxex_models_manager::manager::ModelCatalogSnapshot;
use ruxex_protocol::account::ProviderAccount;
use ruxex_protocol::account::ProviderAuthStatus;
use ruxex_protocol::auth::AuthMode;
use ruxex_protocol::error::Result as CoreResult;
use ruxex_protocol::error::RuxexErr;
use ruxex_protocol::protocol::RateLimitSnapshot;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use ruxex_model_provider::ModelProviderFactory;
use ruxex_model_provider::ModelsEndpoint;
use ruxex_model_provider::ProviderAuthEnvironment;
use ruxex_model_provider::ProviderAuthRecoveryMessages;
use ruxex_model_provider::ProviderCapabilities;
use ruxex_model_provider::ProviderConnectionKey;
use ruxex_model_provider::ProviderCredentialSnapshot;
use ruxex_model_provider::ProviderDescriptor;
use ruxex_model_provider::ProviderRequestAuth;
use ruxex_model_provider::ProviderRequestFormat;
use ruxex_model_provider::ProviderRequestSnapshot;
use ruxex_model_provider::ProviderStreamConnectRequest;
use ruxex_model_provider::ProviderStreamEvent;
use ruxex_model_provider::ProviderStreamOutcome;
use ruxex_model_provider::ProviderStreamRequest;
use ruxex_model_provider::ProviderStreamSession;
use ruxex_model_provider::ProviderStreamTelemetry;
use ruxex_model_provider::ProviderUnauthorizedRecovery;
use ruxex_model_provider::ProviderUsage;
use ruxex_model_provider::ProviderUsageResult;
use ruxex_model_provider::ProviderWebsocketConnectRequest;
use ruxex_model_provider::ProviderWebsocketEvent;
use ruxex_model_provider::ProviderWebsocketPayloadRequest;
use ruxex_model_provider::ProviderWebsocketRequest;
use ruxex_model_provider::ProviderWireApi;
use ruxex_model_provider::RemoteCompactionSupport;
use ruxex_model_provider::SharedProviderCredentialSnapshot;
use ruxex_model_provider::SharedProviderStreamSession;
use ruxex_model_provider::SharedProviderWebsocketConnection;
use ruxex_model_provider::cached_provider_auth;
use ruxex_model_provider::credential_manager_for_provider;
use ruxex_model_provider::headers_to_http;
use ruxex_model_provider::provider_auth;
use ruxex_model_provider::resolve_provider_auth_for_provider;

fn provider_chat_event_from_api(
    event: ruxex_api::ResponseEvent,
) -> ruxex_model_provider::ProviderChatEvent {
    match event {
        ruxex_api::ResponseEvent::Created { response_id } => {
            ruxex_model_provider::ProviderChatEvent::Created { response_id }
        }
        ruxex_api::ResponseEvent::SafetyBuffering(buffering) => {
            ruxex_model_provider::ProviderChatEvent::SafetyBuffering(
                ruxex_model_provider::ProviderSafetyBuffering {
                    use_cases: buffering.use_cases,
                    reasons: buffering.reasons,
                    show_buffering_ui: buffering.show_buffering_ui,
                    faster_model: buffering.faster_model,
                },
            )
        }
        ruxex_api::ResponseEvent::OutputItemDone(item) => {
            ruxex_model_provider::ProviderChatEvent::OutputItemDone(item)
        }
        ruxex_api::ResponseEvent::OutputItemAdded(item) => {
            ruxex_model_provider::ProviderChatEvent::OutputItemAdded(item)
        }
        ruxex_api::ResponseEvent::ServerModel(model) => {
            ruxex_model_provider::ProviderChatEvent::ServerModel(model)
        }
        ruxex_api::ResponseEvent::ModelVerifications(verifications) => {
            ruxex_model_provider::ProviderChatEvent::ModelVerifications(verifications)
        }
        ruxex_api::ResponseEvent::ServerReasoningIncluded(included) => {
            ruxex_model_provider::ProviderChatEvent::ServerReasoningIncluded(included)
        }
        ruxex_api::ResponseEvent::Completed {
            response_id,
            token_usage,
            usage_metadata,
            end_turn,
        } => ruxex_model_provider::ProviderChatEvent::Completed {
            response_id,
            token_usage,
            usage_metadata,
            end_turn,
        },
        ruxex_api::ResponseEvent::OutputTextDelta(delta) => {
            ruxex_model_provider::ProviderChatEvent::OutputTextDelta(delta)
        }
        ruxex_api::ResponseEvent::ToolCallInputDelta {
            item_id,
            call_id,
            delta,
        } => ruxex_model_provider::ProviderChatEvent::ToolCallInputDelta {
            item_id,
            call_id,
            delta,
        },
        ruxex_api::ResponseEvent::ReasoningSummaryDelta {
            delta,
            summary_index,
        } => ruxex_model_provider::ProviderChatEvent::ReasoningSummaryDelta {
            delta,
            summary_index,
        },
        ruxex_api::ResponseEvent::ReasoningSummaryDone {
            item_id,
            text,
            summary_index,
        } => ruxex_model_provider::ProviderChatEvent::ReasoningSummaryDone {
            item_id,
            text,
            summary_index,
        },
        ruxex_api::ResponseEvent::ReasoningContentDelta {
            delta,
            content_index,
        } => ruxex_model_provider::ProviderChatEvent::ReasoningContentDelta {
            delta,
            content_index,
        },
        ruxex_api::ResponseEvent::ReasoningSummaryPartAdded { summary_index } => {
            ruxex_model_provider::ProviderChatEvent::ReasoningSummaryPartAdded { summary_index }
        }
        ruxex_api::ResponseEvent::RateLimits(snapshot) => {
            ruxex_model_provider::ProviderChatEvent::RateLimits(snapshot)
        }
        ruxex_api::ResponseEvent::ModelsEtag(etag) => {
            ruxex_model_provider::ProviderChatEvent::ModelsEtag(etag)
        }
    }
}

/// Current app-visible account state for a model provider.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProviderAccountState {
    pub provider_id: String,
    pub account: Option<ProviderAccount>,
    pub account_id: Option<String>,
    pub display_name: Option<String>,
    pub auth_status: ProviderAuthStatus,
    pub usage: Option<RateLimitSnapshot>,
}

/// Provider-neutral credentials supplied by a host login flow.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct ProviderLoginRequest {
    pub api_key: String,
    pub credentials: BTreeMap<String, String>,
}

impl ProviderLoginRequest {
    pub(super) fn new(api_key: impl Into<String>, credentials: BTreeMap<String, String>) -> Self {
        Self {
            api_key: api_key.into(),
            credentials,
        }
    }
}

/// Error classification for provider-owned credential lifecycle operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProviderAuthError {
    Unavailable,
    Failed(String),
}

impl fmt::Display for ProviderAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => f.write_str("provider credential lifecycle is unavailable"),
            Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ProviderAuthError {}

pub(crate) type ProviderAuthResult = std::result::Result<(), ProviderAuthError>;
pub(crate) type ProviderDeleteResult = std::result::Result<bool, ProviderAuthError>;
pub(crate) type ProviderLogoutResult = std::result::Result<bool, ProviderAuthError>;

/// Error returned when a provider cannot construct its app-visible account state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderAccountError {}

impl fmt::Display for ProviderAccountError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for ProviderAccountError {}

pub(crate) type ProviderAccountResult =
    std::result::Result<ProviderAccountState, ProviderAccountError>;

/// Runtime API selected for model execution.
#[derive(Clone)]
pub(crate) enum ProviderApi {
    /// The existing concrete Responses API implementation.
    Responses,
    /// A provider-specific Chat Completions wire adapter.
    Chat(Arc<dyn ChatCompletionApi>),
}

/// OpenAI Responses wire payload. Core supplies the neutral [`ChatRequest`]; this
/// provider-implementation projection owns the external field names and serialization shape.
#[derive(Debug, Clone, serde::Serialize)]
struct ResponsesPayload {
    model: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    instructions: String,
    input: Vec<ruxex_protocol::models::ResponseItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Value>,
    tool_choice: String,
    parallel_tool_calls: bool,
    reasoning: Option<ruxex_model_provider::ProviderReasoning>,
    store: bool,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<ruxex_model_provider::ProviderStreamOptions>,
    include: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_cache_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<ruxex_model_provider::ProviderTextControls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_metadata: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_programs: Option<Value>,
}

impl From<&ruxex_model_provider::ChatRequest> for ResponsesPayload {
    fn from(request: &ruxex_model_provider::ChatRequest) -> Self {
        Self {
            model: request.model.clone(),
            instructions: request.instructions.clone(),
            input: request.input.clone(),
            tools: (!request.tools.is_empty()).then(|| Value::Array(request.tools.clone())),
            tool_choice: request.tool_choice.clone(),
            parallel_tool_calls: request.parallel_tool_calls,
            reasoning: request.reasoning.clone(),
            store: request.store,
            stream: request.stream,
            stream_options: request.stream_options,
            include: request.include.clone(),
            service_tier: request.service_tier.clone(),
            prompt_cache_key: request.prompt_cache_key.clone(),
            text: request.text.clone(),
            client_metadata: request.client_metadata.clone(),
            access_programs: request.access_programs.clone(),
        }
    }
}

fn map_api_error_to_provider_error(error: ApiError) -> ruxex_model_provider::ProviderError {
    match error {
        ApiError::Transport(TransportError::Http { status, .. })
            if status == http::StatusCode::UNAUTHORIZED =>
        {
            ruxex_model_provider::ProviderError::Unauthorized
        }
        ApiError::Transport(TransportError::Http { status, .. })
            if status == http::StatusCode::UPGRADE_REQUIRED =>
        {
            ruxex_model_provider::ProviderError::UpgradeRequired
        }
        ApiError::InvalidRequest { message } => {
            ruxex_model_provider::ProviderError::InvalidRequest(message)
        }
        ApiError::RateLimitExceeded { delay, .. } => {
            ruxex_model_provider::ProviderError::RateLimited {
                retry_after_seconds: delay.map(|delay| delay.as_secs()),
            }
        }
        ApiError::Retryable { message, .. } => {
            ruxex_model_provider::ProviderError::Transient(message_or_default(message))
        }
        ApiError::ServerOverloaded => {
            ruxex_model_provider::ProviderError::Transient("server overloaded".to_string())
        }
        ApiError::Transport(error) => map_transport_error_to_provider_error(error),
        other => ruxex_model_provider::ProviderError::Provider(other.to_string()),
    }
}

fn map_transport_error_to_provider_error(
    error: TransportError,
) -> ruxex_model_provider::ProviderError {
    match error {
        TransportError::Http { status, .. } if status == http::StatusCode::UNAUTHORIZED => {
            ruxex_model_provider::ProviderError::Unauthorized
        }
        TransportError::Timeout => {
            ruxex_model_provider::ProviderError::Transient("provider request timed out".to_string())
        }
        other => ruxex_model_provider::ProviderError::Transport(other.to_string()),
    }
}

fn message_or_default(message: String) -> String {
    if message.is_empty() {
        "provider request failed".to_string()
    } else {
        message
    }
}

fn classify_api_error(error: &ApiError) -> ruxex_model_provider::ProviderError {
    match error {
        ApiError::Transport(TransportError::Http { status, .. })
            if *status == http::StatusCode::UNAUTHORIZED =>
        {
            ruxex_model_provider::ProviderError::Unauthorized
        }
        ApiError::Transport(TransportError::Http { status, .. })
            if *status == http::StatusCode::UPGRADE_REQUIRED =>
        {
            ruxex_model_provider::ProviderError::UpgradeRequired
        }
        ApiError::InvalidRequest { message } => {
            ruxex_model_provider::ProviderError::InvalidRequest(message.clone())
        }
        ApiError::RateLimitExceeded { delay, .. } => {
            ruxex_model_provider::ProviderError::RateLimited {
                retry_after_seconds: delay.map(|delay| delay.as_secs()),
            }
        }
        ApiError::Retryable { message, .. } => {
            ruxex_model_provider::ProviderError::Transient(message_or_default(message.clone()))
        }
        ApiError::ServerOverloaded => {
            ruxex_model_provider::ProviderError::Transient("server overloaded".to_string())
        }
        ApiError::Transport(TransportError::Timeout) => {
            ruxex_model_provider::ProviderError::Transient("provider request timed out".to_string())
        }
        ApiError::Transport(error) => {
            ruxex_model_provider::ProviderError::Transport(error.to_string())
        }
        other => ruxex_model_provider::ProviderError::Provider(other.to_string()),
    }
}

struct ProviderWebsocketTelemetryAdapter {
    telemetry: Arc<dyn ruxex_model_provider::ProviderWebsocketTelemetry>,
}

impl WebsocketTelemetry for ProviderWebsocketTelemetryAdapter {
    fn on_ws_request(&self, duration: Duration, error: Option<&ApiError>, connection_reused: bool) {
        let error = error.map(classify_api_error);
        self.telemetry
            .on_request(duration, error.as_ref(), connection_reused);
    }

    fn on_ws_event(
        &self,
        result: &std::result::Result<
            Option<std::result::Result<Message, tokio_tungstenite::tungstenite::Error>>,
            ApiError,
        >,
        duration: Duration,
    ) {
        let event = match result {
            Ok(Some(Ok(Message::Text(text)))) => serde_json::from_str::<Value>(text.as_ref())
                .map(ProviderWebsocketEvent::Message)
                .unwrap_or_else(|_| {
                    ProviderWebsocketEvent::Message(Value::String(text.to_string()))
                }),
            Ok(Some(Ok(Message::Ping(_)))) | Ok(Some(Ok(Message::Pong(_)))) => {
                ProviderWebsocketEvent::Ignored
            }
            Ok(Some(Ok(Message::Binary(_))))
            | Ok(Some(Ok(Message::Close(_))))
            | Ok(Some(Ok(Message::Frame(_)))) => {
                ProviderWebsocketEvent::Error(ruxex_model_provider::ProviderError::Transport(
                    "unexpected websocket message".to_string(),
                ))
            }
            Ok(Some(Err(error))) => ProviderWebsocketEvent::Error(
                ruxex_model_provider::ProviderError::Transport(error.to_string()),
            ),
            Ok(None) => ProviderWebsocketEvent::Closed,
            Err(error) => ProviderWebsocketEvent::Error(classify_api_error(error)),
        };
        self.telemetry.on_event(event, duration);
    }
}

#[derive(Debug)]
struct RuntimeWebsocketConnection {
    connection: ResponsesWebsocketConnection,
}

struct StreamTelemetryAdapter {
    telemetry: Arc<dyn ProviderStreamTelemetry>,
}

impl ruxex_model_provider::ProviderWebsocketTelemetry for StreamTelemetryAdapter {
    fn on_request(
        &self,
        duration: Duration,
        error: Option<&ruxex_model_provider::ProviderError>,
        connection_reused: bool,
    ) {
        self.telemetry
            .on_request(duration, error, connection_reused);
    }

    fn on_event(&self, event: ProviderWebsocketEvent, duration: Duration) {
        let event = match event {
            ProviderWebsocketEvent::Message(value) => ProviderStreamEvent::Message(value),
            ProviderWebsocketEvent::Ignored => ProviderStreamEvent::Ignored,
            ProviderWebsocketEvent::Closed => ProviderStreamEvent::Closed,
            ProviderWebsocketEvent::Error(error) => ProviderStreamEvent::Error(error),
        };
        self.telemetry.on_event(event, duration);
    }
}

#[derive(Debug, Default)]
struct RuntimeStreamSessionState {
    connection: Option<SharedProviderWebsocketConnection>,
    connection_key: Option<ProviderConnectionKey>,
    extra_headers: BTreeMap<String, String>,
}

/// Provider-owned persistent streaming state. Core only sees the neutral session interface.
#[derive(Debug)]
pub(super) struct RuntimeStreamSession {
    runtime: SharedProviderRuntime,
    state: Mutex<RuntimeStreamSessionState>,
}

impl RuntimeStreamSession {
    pub(super) fn new(runtime: SharedProviderRuntime) -> Self {
        Self {
            runtime,
            state: Mutex::new(RuntimeStreamSessionState::default()),
        }
    }

    async fn ensure_connection(
        &self,
        request: &ProviderStreamConnectRequest,
        telemetry: Option<Arc<dyn ProviderStreamTelemetry>>,
    ) -> ruxex_model_provider::ProviderResult<(SharedProviderWebsocketConnection, bool)> {
        let mut state = self.state.lock().await;
        if let Some(connection) = state.connection.clone() {
            let same_route = state.connection_key.as_ref() == Some(&request.connection_key)
                && state.extra_headers == request.extra_headers;
            let closed = connection.is_closed().await.unwrap_or(true);
            if same_route && !closed {
                return Ok((connection, true));
            }
        }

        let telemetry = telemetry.map(|telemetry| {
            Arc::new(StreamTelemetryAdapter { telemetry })
                as Arc<dyn ruxex_model_provider::ProviderWebsocketTelemetry>
        });
        let connection = ProviderRuntime::connect_websocket(
            self.runtime.as_ref(),
            ProviderWebsocketConnectRequest {
                session_id: request.session_id.clone(),
                thread_id: request.thread_id.clone(),
                extra_headers: request.extra_headers.clone(),
                default_headers: request.default_headers.clone(),
                credential_snapshot: Some(Arc::clone(&request.credential_snapshot)),
                telemetry,
            },
        )
        .await?;
        state.connection = Some(Arc::clone(&connection));
        state.connection_key = Some(request.connection_key.clone());
        state.extra_headers = request.extra_headers.clone();
        Ok((connection, false))
    }
}

impl ProviderStreamSession for RuntimeStreamSession {
    fn preconnect(
        &self,
        request: ProviderStreamConnectRequest,
    ) -> ruxex_model_provider::ProviderFuture<'_, ruxex_model_provider::ProviderResult<()>> {
        Box::pin(async move {
            self.ensure_connection(&request, request.telemetry.clone())
                .await
                .map(|_| ())
        })
    }

    fn stream_request(
        &self,
        request: ProviderStreamRequest,
    ) -> ruxex_model_provider::ProviderFuture<
        '_,
        ruxex_model_provider::ProviderResult<ProviderStreamOutcome>,
    > {
        Box::pin(async move {
            if ProviderRuntime::wire_api(self.runtime.as_ref()) != ProviderWireApi::Responses {
                return Ok(ProviderStreamOutcome::FallbackToHttp);
            }

            let transport = &request.request.transport;
            let connection_key = transport.connection_key.clone().ok_or_else(|| {
                ruxex_model_provider::ProviderError::InvalidRequest(
                    "provider stream request is missing a connection key".to_string(),
                )
            })?;
            let credential_snapshot = transport
                .credential_snapshot
                .clone()
                .ok_or_else(|| ruxex_model_provider::ProviderError::Unauthenticated)?;
            let connect_request = ProviderStreamConnectRequest {
                session_id: transport.session_id.clone(),
                thread_id: transport.thread_id.clone(),
                extra_headers: transport.extra_headers.clone(),
                default_headers: request.default_headers.clone(),
                connection_key,
                credential_snapshot,
                telemetry: request.telemetry.clone(),
            };
            let (connection, connection_reused) = self
                .ensure_connection(&connect_request, request.telemetry.clone())
                .await?;
            let continuation = request.continuation.unwrap_or_else(|| {
                ruxex_model_provider::ProviderStreamContinuation {
                    previous_response_id: None,
                    input: request.request.input.clone(),
                    generate: None,
                    client_metadata: None,
                }
            });
            let payload = ProviderRuntime::build_websocket_payload(
                self.runtime.as_ref(),
                ProviderWebsocketPayloadRequest {
                    request: request.request.clone(),
                    previous_response_id: continuation.previous_response_id,
                    input: continuation.input,
                    generate: continuation.generate,
                    client_metadata: continuation.client_metadata,
                },
            )?;
            let stream = connection
                .stream_request(ProviderWebsocketRequest {
                    payload,
                    connection_reused,
                    turn_state: request.request.transport.turn_state.clone(),
                })
                .await?;
            Ok(ProviderStreamOutcome::Stream(stream))
        })
    }
}

#[derive(Clone)]
struct RuntimeCredentialSnapshot {
    provider: Provider,
    auth: SharedAuthProvider,
}

impl fmt::Debug for RuntimeCredentialSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeCredentialSnapshot")
            .field("provider", &self.provider.name)
            .finish_non_exhaustive()
    }
}

impl ProviderCredentialSnapshot for RuntimeCredentialSnapshot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn runtime_credentials(
    snapshot: Option<&SharedProviderCredentialSnapshot>,
) -> ruxex_model_provider::ProviderResult<(Provider, SharedAuthProvider)> {
    let snapshot = snapshot.ok_or_else(|| {
        ruxex_model_provider::ProviderError::Provider(
            "provider request is missing its credential snapshot".to_string(),
        )
    })?;
    let snapshot = snapshot
        .as_any()
        .downcast_ref::<RuntimeCredentialSnapshot>()
        .ok_or_else(|| {
            ruxex_model_provider::ProviderError::Provider(
                "provider request credential snapshot belongs to another provider".to_string(),
            )
        })?;
    Ok((snapshot.provider.clone(), Arc::clone(&snapshot.auth)))
}

impl ruxex_model_provider::ProviderWebsocketConnection for RuntimeWebsocketConnection {
    fn is_closed(
        &self,
    ) -> ruxex_model_provider::ProviderFuture<'_, ruxex_model_provider::ProviderResult<bool>> {
        Box::pin(async move { Ok(self.connection.is_closed().await) })
    }

    fn stream_request(
        &self,
        request: ProviderWebsocketRequest,
    ) -> ruxex_model_provider::ProviderFuture<
        '_,
        ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderChatStream>,
    > {
        Box::pin(async move {
            let stream = self
                .connection
                .stream_request_json(
                    request.payload,
                    request.connection_reused,
                    request.turn_state,
                )
                .await
                .map_err(map_api_error_to_provider_error)?;
            Ok(Box::pin(stream.map(|event| {
                event
                    .map(provider_chat_event_from_api)
                    .map_err(map_api_error_to_provider_error)
            })) as ruxex_model_provider::ProviderChatStream)
        })
    }
}

/// Runtime provider abstraction used by model execution.
///
/// Implementations own provider-specific behavior for a model backend. The
/// `ModelProviderInfo` returned by `info` is the serialized/configured provider
/// metadata used by the default provider-compatible implementation.
pub(super) trait ProviderRuntime: fmt::Debug + Send + Sync + 'static {
    /// Returns the configured provider metadata.
    fn info(&self) -> &ModelProviderInfo;

    /// Returns the host-selected outbound transport owned by this provider runtime.
    fn http_client_factory(&self) -> HttpClientFactory {
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
    }

    /// Returns the stable provider identity used to validate a data-root mount.
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor::new(
            self.provider_id().unwrap_or(self.info().name.as_str()),
            CURRENT_PROVIDER_PROTOCOL_VERSION,
        )
    }

    /// Returns non-secret environment metadata for host-owned telemetry.
    fn auth_environment(&self) -> ProviderAuthEnvironment {
        ProviderAuthEnvironment {
            host_api_key_env_present: environment_variable_present(RUXEX_API_KEY_ENV_VAR),
            host_api_key_env_enabled: self
                .credential_manager()
                .is_some_and(|manager| manager.api_key_env_enabled()),
            provider_env_key_configured: self.info().env_key.is_some(),
            provider_env_key_present: self
                .info()
                .env_key
                .as_deref()
                .map(environment_variable_present),
        }
    }

    /// Resolves the runtime API implementation for this provider.
    fn api(&self) -> ProviderApi {
        match self.info().wire_api {
            WireApi::Responses => ProviderApi::Responses,
            WireApi::Chat => ProviderApi::Chat(Arc::new(StandardChatCompletionApi)),
        }
    }

    fn wire_api(&self) -> ProviderWireApi {
        match self.info().wire_api {
            WireApi::Responses => ProviderWireApi::Responses,
            WireApi::Chat => ProviderWireApi::Chat,
        }
    }

    fn websocket_connect_timeout(&self) -> Duration {
        self.info().websocket_connect_timeout()
    }

    fn stream_max_retries(&self) -> u64 {
        self.info().stream_max_retries()
    }

    fn connect_websocket(
        &self,
        request: ProviderWebsocketConnectRequest,
    ) -> ruxex_model_provider::ProviderFuture<
        '_,
        ruxex_model_provider::ProviderResult<SharedProviderWebsocketConnection>,
    > {
        let http_client_factory = self.http_client_factory();
        Box::pin(async move {
            let (api_provider, api_auth) =
                runtime_credentials(request.credential_snapshot.as_ref())?;
            let telemetry = request.telemetry.map(|telemetry| {
                Arc::new(ProviderWebsocketTelemetryAdapter { telemetry })
                    as Arc<dyn WebsocketTelemetry>
            });
            let mut extra_headers = headers_to_http(request.extra_headers)?;
            extra_headers.extend(ruxex_api::build_session_headers(
                request.session_id,
                request.thread_id,
            ));
            let connection = ResponsesWebsocketClient::new(api_provider, api_auth)
                .connect(
                    &http_client_factory,
                    extra_headers,
                    headers_to_http(request.default_headers)?,
                    None,
                    telemetry,
                )
                .await
                .map_err(map_api_error_to_provider_error)?;
            Ok(Arc::new(RuntimeWebsocketConnection { connection })
                as SharedProviderWebsocketConnection)
        })
    }

    fn build_websocket_payload(
        &self,
        request: ProviderWebsocketPayloadRequest,
    ) -> ruxex_model_provider::ProviderResult<Value> {
        let mut payload =
            serde_json::to_value(ResponsesPayload::from(&request.request)).map_err(|error| {
                ruxex_model_provider::ProviderError::InvalidRequest(format!(
                    "failed to encode provider websocket request: {error}"
                ))
            })?;
        let object = payload.as_object_mut().ok_or_else(|| {
            ruxex_model_provider::ProviderError::InvalidRequest(
                "provider websocket request must be an object".to_string(),
            )
        })?;
        object.insert(
            "type".to_string(),
            Value::String("response.create".to_string()),
        );
        object.insert(
            "input".to_string(),
            serde_json::to_value(request.input).map_err(|error| {
                ruxex_model_provider::ProviderError::InvalidRequest(error.to_string())
            })?,
        );
        if let Some(previous_response_id) = request.previous_response_id {
            object.insert(
                "previous_response_id".to_string(),
                Value::String(previous_response_id),
            );
        }
        if let Some(generate) = request.generate {
            object.insert("generate".to_string(), Value::Bool(generate));
        }
        if let Some(client_metadata) = request.client_metadata {
            object.insert(
                "client_metadata".to_string(),
                serde_json::to_value(client_metadata).map_err(|error| {
                    ruxex_model_provider::ProviderError::InvalidRequest(error.to_string())
                })?,
            );
        }
        Ok(payload)
    }

    /// Streams a provider-owned request through the provider-neutral contract.
    ///
    /// The contract carries normalized chat input. Request construction and HTTP transport are
    /// owned here; the remaining core-owned boundaries are Responses authentication
    /// recovery/continuation and WebSocket policy.
    fn chat(
        &self,
        request: ruxex_model_provider::ChatRequest,
    ) -> ruxex_model_provider::ProviderFuture<
        '_,
        ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderChatStream>,
    > {
        let http_client_factory = self.http_client_factory();
        Box::pin(async move {
            let (api_provider, api_auth) =
                runtime_credentials(request.transport.credential_snapshot.as_ref())?;
            match self.api() {
                ProviderApi::Chat(chat_api) => {
                    let client = create_client_for_route(
                        &http_client_factory,
                        &api_provider.url_for_path("/chat/completions"),
                        ClientRouteClass::Api,
                        ClientRedirectPolicy::Default,
                    )
                    .map_err(|error| {
                        ruxex_model_provider::ProviderError::Transport(error.to_string())
                    })?;
                    let transport = ruxex_api::ReqwestTransport::from_http_client(client);
                    let chat_request = chat_api
                        .build_request(ChatRequestInput {
                            provider: &api_provider,
                            model: &request.model,
                            instructions: &request.instructions,
                            input: &request.input,
                            tools: &request.tools,
                            conversation_id: request.conversation_id,
                            session_source: request.session_source,
                        })
                        .map_err(|error| {
                            ruxex_model_provider::ProviderError::InvalidRequest(error.to_string())
                        })?;
                    let stream = ChatClient::with_api(transport, api_provider, api_auth, chat_api)
                        .stream_request(chat_request)
                        .await
                        .map_err(map_api_error_to_provider_error)?;
                    Ok(Box::pin(stream.map(|event| {
                        event
                            .map(provider_chat_event_from_api)
                            .map_err(map_api_error_to_provider_error)
                    }))
                        as ruxex_model_provider::ProviderChatStream)
                }
                ProviderApi::Responses => {
                    let payload = ResponsesPayload::from(&request);
                    let client = create_client_for_route(
                        &http_client_factory,
                        &api_provider.url_for_path("/responses"),
                        ClientRouteClass::Api,
                        ClientRedirectPolicy::Default,
                    )
                    .map_err(|error| {
                        ruxex_model_provider::ProviderError::Transport(error.to_string())
                    })?;
                    let transport = ruxex_api::ReqwestTransport::from_http_client(client);
                    let compression = match request.transport.compression {
                        ruxex_model_provider::ProviderRequestCompression::None => {
                            ruxex_api::Compression::None
                        }
                        ruxex_model_provider::ProviderRequestCompression::Zstd => {
                            ruxex_api::Compression::Zstd
                        }
                    };
                    let mut headers = headers_to_http(request.transport.extra_headers)?;
                    headers.extend(ruxex_api::build_session_headers(
                        request.transport.session_id,
                        request.transport.thread_id.clone(),
                    ));
                    if let Some(thread_id) = request.transport.thread_id {
                        if let Ok(value) = http::HeaderValue::from_str(&thread_id) {
                            headers.insert(
                                http::header::HeaderName::from_static("x-client-request-id"),
                                value,
                            );
                        }
                    }
                    let payload = serde_json::to_value(payload).map_err(|error| {
                        ruxex_model_provider::ProviderError::InvalidRequest(error.to_string())
                    })?;
                    let stream = ResponsesClient::new(transport, api_provider, api_auth)
                        .stream(payload, headers, compression, request.transport.turn_state)
                        .await
                        .map_err(map_api_error_to_provider_error)?;
                    Ok(Box::pin(stream.map(|event| {
                        event
                            .map(provider_chat_event_from_api)
                            .map_err(map_api_error_to_provider_error)
                    }))
                        as ruxex_model_provider::ProviderChatStream)
                }
            }
        })
    }

    fn search(
        &self,
        request: ruxex_model_provider::ProviderSearchRequest,
    ) -> ruxex_model_provider::ProviderFuture<
        '_,
        ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderSearchResponse>,
    > {
        let http_client_factory = self.http_client_factory();
        Box::pin(async move {
            let api_provider = self.api_provider().await.map_err(|error| {
                ruxex_model_provider::ProviderError::Provider(error.to_string())
            })?;
            let api_auth = self.api_auth().await.map_err(|error| {
                ruxex_model_provider::ProviderError::Provider(error.to_string())
            })?;
            let client = create_client_for_route(
                &http_client_factory,
                &api_provider.url_for_path("alpha/search"),
                ClientRouteClass::Api,
                ClientRedirectPolicy::Default,
            )
            .map_err(|error| ruxex_model_provider::ProviderError::Transport(error.to_string()))?;
            let commands: SearchCommands =
                serde_json::from_value(request.commands).map_err(|error| {
                    ruxex_model_provider::ProviderError::InvalidRequest(error.to_string())
                })?;
            let settings: SearchSettings =
                serde_json::from_value(request.settings).map_err(|error| {
                    ruxex_model_provider::ProviderError::InvalidRequest(error.to_string())
                })?;
            let search_request = SearchRequest {
                id: request.id,
                model: request.model,
                reasoning: None,
                input: Some(SearchInput::Items(request.input)),
                commands: Some(commands),
                settings: Some(settings),
                max_output_tokens: request.max_output_tokens,
            };
            let response = SearchClient::new(
                ruxex_api::ReqwestTransport::from_http_client(client),
                api_provider,
                api_auth,
            )
            .search(&search_request, headers_to_http(request.extra_headers)?)
            .await
            .map_err(map_api_error_to_provider_error)?;
            Ok(ruxex_model_provider::ProviderSearchResponse {
                output: response.output,
                results: response.results,
            })
        })
    }

    /// Executes a provider-owned JSON backend operation.
    ///
    /// The request path remains a provider/backend contract. The host supplies only the
    /// feature-specific JSON and headers; endpoint resolution, authentication, routing, and
    /// transport stay inside the runtime provider.
    fn backend(
        &self,
        request: ruxex_model_provider::ProviderBackendRequest,
    ) -> ruxex_model_provider::ProviderFuture<
        '_,
        ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderBackendResponse>,
    > {
        let http_client_factory = self.http_client_factory();
        Box::pin(async move {
            let api_provider = self.api_provider().await.map_err(|error| {
                ruxex_model_provider::ProviderError::Provider(error.to_string())
            })?;
            let api_auth = self.api_auth().await.map_err(|error| {
                ruxex_model_provider::ProviderError::Provider(error.to_string())
            })?;
            let client = create_client_for_route(
                &http_client_factory,
                &api_provider.url_for_path(&request.path),
                ClientRouteClass::Api,
                ClientRedirectPolicy::Default,
            )
            .map_err(|error| ruxex_model_provider::ProviderError::Transport(error.to_string()))?;
            let mut backend_request = api_provider.build_request(http::Method::POST, &request.path);
            backend_request
                .headers
                .extend(headers_to_http(request.headers)?);
            backend_request.body = Some(RequestBody::Json(request.body));
            backend_request.timeout = Some(request.timeout);
            let backend_request = api_auth
                .apply_auth(backend_request)
                .await
                .map_err(|error| {
                    ruxex_model_provider::ProviderError::Provider(error.to_string())
                })?;
            let response = ruxex_api::ReqwestTransport::from_http_client(client)
                .execute(backend_request)
                .await
                .map_err(map_transport_error_to_provider_error)?;
            let body = serde_json::from_slice(&response.body).map_err(|error| {
                ruxex_model_provider::ProviderError::Provider(format!(
                    "provider backend returned invalid JSON: {error}"
                ))
            })?;
            Ok(ruxex_model_provider::ProviderBackendResponse { body })
        })
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

    fn model_catalog_auth(&self) -> ruxex_model_provider::ProviderModelCatalogAuth {
        match self
            .credential_manager()
            .and_then(|manager| manager.auth_mode())
        {
            Some(AuthMode::Headers) => ruxex_model_provider::ProviderModelCatalogAuth {
                uses_managed_auth: true,
                has_api_key_auth: false,
            },
            Some(AuthMode::ApiKey) => ruxex_model_provider::ProviderModelCatalogAuth {
                uses_managed_auth: false,
                has_api_key_auth: true,
            },
            None => ruxex_model_provider::ProviderModelCatalogAuth::default(),
        }
    }

    /// Returns whether requests made through this provider should include attestation.
    fn supports_attestation(&self) -> bool {
        false
    }

    /// Returns the provider-scoped auth manager for provider-implementation-internal operations.
    ///
    /// Core must use the provider-neutral auth lifecycle and request snapshot instead.
    /// The provider implementation retains this narrow accessor because its model-catalog and
    /// unauthorized-recovery implementations still consume the provider-owned manager.
    fn credential_manager(&self) -> Option<Arc<CredentialManager>>;

    /// Returns the configured provider ID used for provider-scoped credentials.
    fn auth_revision(&self) -> Option<u64> {
        self.credential_manager()
            .map(|manager| *manager.auth_change_receiver().borrow())
    }

    fn managed_auth(&self) -> bool {
        self.credential_manager()
            .and_then(|manager| manager.auth_mode())
            .is_some_and(|mode| mode == AuthMode::Headers)
    }

    fn provider_id(&self) -> Option<&str> {
        None
    }

    /// Returns lifecycle messages when provider-owned authentication recovery is active.
    fn auth_recovery_messages(&self) -> Option<ProviderAuthRecoveryMessages> {
        None
    }

    /// Attempts provider-owned authentication recovery using the mounted auth manager.
    ///
    /// The recovery state machine remains inside the provider implementation so Core does not need
    /// to know which credential source or refresh protocol the provider uses.
    fn recover_from_unauthorized(
        &self,
    ) -> ModelProviderFuture<'_, ruxex_protocol::error::Result<ProviderUnauthorizedRecovery>> {
        let credential_manager = self.credential_manager();
        Box::pin(async move {
            let Some(credential_manager) = credential_manager else {
                return Ok(ProviderUnauthorizedRecovery::NotConfigured);
            };
            let mut recovery = credential_manager.unauthorized_recovery();
            if !recovery.has_next() {
                return Ok(ProviderUnauthorizedRecovery::NotConfigured);
            }
            recovery
                .next()
                .await
                .map(|_| ProviderUnauthorizedRecovery::Recovered)
                .map_err(|error| RuxexErr::Fatal(error.to_string()))
        })
    }

    /// Returns the current provider-scoped auth value, if one is configured.
    fn auth(&self) -> ModelProviderFuture<'_, Option<RuxexAuth>>;

    /// Returns the current app-visible account state for this provider.
    fn account_state(&self) -> ProviderAccountResult;

    /// Saves credentials for this mounted provider.
    fn login(&self, request: ProviderLoginRequest) -> ModelProviderFuture<'_, ProviderAuthResult> {
        Box::pin(async move {
            let Some(credential_manager) = self.credential_manager() else {
                return Err(ProviderAuthError::Unavailable);
            };
            credential_manager
                .login(ProviderAuth::api_key_with_credentials(
                    request.api_key,
                    request.credentials,
                ))
                .await
                .map_err(|err| ProviderAuthError::Failed(err.to_string()))
        })
    }

    /// Removes persisted and in-memory credentials for this mounted provider.
    fn delete(&self) -> ModelProviderFuture<'_, ProviderDeleteResult> {
        Box::pin(async move {
            let Some(credential_manager) = self.credential_manager() else {
                return Err(ProviderAuthError::Unavailable);
            };
            credential_manager
                .delete()
                .await
                .map_err(|err| ProviderAuthError::Failed(err.to_string()))
        })
    }

    /// Logs out from this mounted provider.
    fn logout(&self) -> ModelProviderFuture<'_, ProviderLogoutResult> {
        Box::pin(async move {
            let Some(credential_manager) = self.credential_manager() else {
                return Err(ProviderAuthError::Unavailable);
            };
            credential_manager
                .logout()
                .await
                .map_err(|err| ProviderAuthError::Failed(err.to_string()))
        })
    }

    /// Returns provider-owned usage data when the provider exposes such a capability.
    fn usage(&self) -> ModelProviderFuture<'_, ProviderUsageResult> {
        Box::pin(async { Ok(ProviderUsage::Unsupported) })
    }

    /// Returns provider configuration adapted for the API client.
    fn api_provider(&self) -> ModelProviderFuture<'_, ruxex_protocol::error::Result<Provider>> {
        Box::pin(async move {
            let auth = self.auth().await;
            self.info()
                .to_api_provider(auth.as_ref().map(RuxexAuth::auth_mode))
        })
    }

    /// Returns the auth provider used to attach request credentials.
    fn api_auth(
        &self,
    ) -> ModelProviderFuture<'_, ruxex_protocol::error::Result<SharedAuthProvider>> {
        Box::pin(async move {
            let auth = self.auth().await;
            resolve_provider_auth_for_provider(auth.as_ref(), self.info())
        })
    }

    fn request_snapshot(&self) -> ModelProviderFuture<'_, CoreResult<ProviderRequestSnapshot>> {
        Box::pin(async move {
            let auth = self.auth().await;
            let api_provider = self
                .info()
                .to_api_provider(auth.as_ref().map(RuxexAuth::auth_mode))?;
            let api_auth = resolve_provider_auth_for_provider(auth.as_ref(), self.info())?;
            let auth_telemetry = auth_header_telemetry(api_auth.as_ref());
            let (revision, owner_generation) = self
                .credential_manager()
                .map(|manager| {
                    let revision = *manager.auth_change_receiver().borrow();
                    let owner_generation = manager
                        .auth_change_state_receiver()
                        .borrow()
                        .owner_generation;
                    (Some(revision), Some(owner_generation))
                })
                .unwrap_or((None, None));
            Ok(ProviderRequestSnapshot {
                auth: ProviderRequestAuth {
                    authenticated: auth.is_some(),
                    managed_auth: auth.as_ref().is_some_and(RuxexAuth::uses_managed_auth),
                    auth_header_attached: auth_telemetry.attached,
                    auth_header_name: auth_telemetry.name,
                    revision,
                    owner_generation,
                },
                connection_key: ProviderConnectionKey {
                    provider_kind: self.descriptor().provider_kind,
                    auth_revision: revision,
                },
                credential_snapshot: Arc::new(RuntimeCredentialSnapshot {
                    provider: api_provider,
                    auth: api_auth,
                }),
            })
        })
    }
}

macro_rules! impl_model_provider_for_runtime {
    ($provider:ty) => {
        impl ruxex_model_provider::ModelProvider for $provider {
            fn descriptor(&self) -> ProviderDescriptor {
                ProviderRuntime::descriptor(self)
            }

            fn request_format(&self) -> ProviderRequestFormat {
                match ProviderRuntime::wire_api(self) {
                    ProviderWireApi::Responses => ProviderRequestFormat::Responses,
                    ProviderWireApi::Chat => ProviderRequestFormat::Chat,
                }
            }

            fn websocket_connect_timeout(&self) -> Duration {
                ProviderRuntime::websocket_connect_timeout(self)
            }

            fn stream_max_retries(&self) -> u64 {
                ProviderRuntime::stream_max_retries(self)
            }

            fn model_catalog_identity(&self) -> ruxex_model_provider::ProviderResult<String> {
                let auth = ProviderRuntime::credential_manager(self)
                    .and_then(|manager| manager.auth_cached());
                ruxex_model_provider::provider_models_identity::identity(
                    ProviderRuntime::provider_id(self),
                    ProviderRuntime::info(self),
                    auth.as_ref(),
                )
                .map_err(|error| ruxex_model_provider::ProviderError::Provider(error.to_string()))
            }

            fn capabilities(&self) -> ProviderCapabilities {
                let mut capabilities = ProviderRuntime::capabilities(self);
                capabilities.api_key_model_discovery = capabilities.api_key_model_discovery
                    || ProviderRuntime::api_key_model_discovery_enabled_by_default(self)
                    || ProviderRuntime::info(self).env_key.is_some();
                capabilities.command_auth =
                    capabilities.command_auth || ProviderRuntime::info(self).has_command_auth();
                capabilities
            }

            fn model_catalog_auth(&self) -> ruxex_model_provider::ProviderModelCatalogAuth {
                ProviderRuntime::model_catalog_auth(self)
            }

            fn supports_attestation(&self) -> bool {
                ProviderRuntime::supports_attestation(self)
            }

            fn chat(
                &self,
                request: ruxex_model_provider::ChatRequest,
            ) -> ruxex_model_provider::ProviderFuture<
                '_,
                ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderChatStream>,
            > {
                ProviderRuntime::chat(self, request)
            }

            fn open_stream_session(
                &self,
            ) -> ruxex_model_provider::ProviderFuture<
                '_,
                ruxex_model_provider::ProviderResult<SharedProviderStreamSession>,
            > {
                if ProviderRuntime::wire_api(self) != ProviderWireApi::Responses
                    || !ProviderRuntime::capabilities(self).websocket
                {
                    return Box::pin(async { Err(ruxex_model_provider::ProviderError::Unsupported) });
                }

                let runtime: SharedProviderRuntime = Arc::new(self.clone());
                Box::pin(async move {
                    Ok(Arc::new(RuntimeStreamSession::new(runtime)) as SharedProviderStreamSession)
                })
            }

            fn search(
                &self,
                request: ruxex_model_provider::ProviderSearchRequest,
            ) -> ruxex_model_provider::ProviderFuture<
                '_,
                ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderSearchResponse>,
            > {
                ProviderRuntime::search(self, request)
            }

            fn backend(
                &self,
                request: ruxex_model_provider::ProviderBackendRequest,
            ) -> ruxex_model_provider::ProviderFuture<
                '_,
                ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderBackendResponse>,
            > {
                ProviderRuntime::backend(self, request)
            }

            fn auth_recovery_messages(
                &self,
            ) -> Option<ruxex_model_provider::ProviderAuthRecoveryMessages> {
                ProviderRuntime::auth_recovery_messages(self)
            }

            fn recover_from_unauthorized(
                &self,
            ) -> ruxex_model_provider::ProviderFuture<
                '_,
                ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderUnauthorizedRecovery>,
            > {
                Box::pin(async move {
                    ProviderRuntime::recover_from_unauthorized(self)
                        .await
                        .map_err(|error| {
                            ruxex_model_provider::ProviderError::Provider(error.to_string())
                        })
                })
            }

            fn list_models(
                &self,
                request: ruxex_model_provider::ListModelsRequest,
            ) -> ruxex_model_provider::ProviderFuture<
                '_,
                ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderModelCatalog>,
            > {
                let client_version = request.client_version;
                let http_client_factory = ProviderRuntime::http_client_factory(self);
                Box::pin(async move {
                    ProviderRuntime::list_models(self, &client_version, http_client_factory)
                        .await
                        .map(|snapshot| ruxex_model_provider::ProviderModelCatalog {
                            catalog: snapshot.catalog,
                            identity: snapshot.identity,
                            etag: snapshot.etag,
                        })
                        .map_err(|error| {
                            ruxex_model_provider::ProviderError::Provider(error.to_string())
                        })
                })
            }

            fn auth_state(&self) -> ruxex_model_provider::ProviderAuthState {
                match ProviderRuntime::account_state(self) {
                    Ok(state) => ruxex_model_provider::ProviderAuthState {
                        status: state.auth_status,
                        account_id: state.account_id,
                        display_name: state.display_name,
                        managed_auth: ProviderRuntime::managed_auth(self),
                        revision: ProviderRuntime::auth_revision(self),
                    },
                    Err(error) => ruxex_model_provider::ProviderAuthState {
                        status: ProviderAuthStatus::Invalid,
                        account_id: None,
                        display_name: Some(error.to_string()),
                        managed_auth: false,
                        revision: None,
                    },
                }
            }

            fn auth_environment(&self) -> ProviderAuthEnvironment {
                ProviderRuntime::auth_environment(self)
            }

            fn request_snapshot(
                &self,
            ) -> ruxex_model_provider::ProviderFuture<
                '_,
                ruxex_model_provider::ProviderResult<ProviderRequestSnapshot>,
            > {
                Box::pin(async move {
                    ProviderRuntime::request_snapshot(self)
                        .await
                        .map_err(|error| {
                            ruxex_model_provider::ProviderError::Provider(error.to_string())
                        })
                })
            }

            fn account_state(
                &self,
            ) -> ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderAccountState> {
                ProviderRuntime::account_state(self)
                    .map(|state| ruxex_model_provider::ProviderAccountState {
                        provider_id: state.provider_id,
                        account: state.account,
                        account_id: state.account_id,
                        display_name: state.display_name,
                        auth_status: state.auth_status,
                        usage: state.usage,
                    })
                    .map_err(|error| ruxex_model_provider::ProviderError::Provider(error.to_string()))
            }

            fn login(
                &self,
                credentials: BTreeMap<String, String>,
            ) -> ruxex_model_provider::ProviderFuture<'_, ruxex_model_provider::ProviderResult<()>> {
                let api_key = credentials.get("api_key").cloned().unwrap_or_default();
                Box::pin(async move {
                    ProviderRuntime::login(self, ProviderLoginRequest::new(api_key, credentials))
                        .await
                        .map_err(map_provider_auth_error)
                })
            }

            fn logout(
                &self,
            ) -> ruxex_model_provider::ProviderFuture<'_, ruxex_model_provider::ProviderResult<()>> {
                Box::pin(async move {
                    ProviderRuntime::logout(self)
                        .await
                        .map(|_| ())
                        .map_err(map_provider_auth_error)
                })
            }

            fn delete(
                &self,
            ) -> ruxex_model_provider::ProviderFuture<'_, ruxex_model_provider::ProviderResult<bool>> {
                Box::pin(async move {
                    ProviderRuntime::delete(self)
                        .await
                        .map_err(map_provider_auth_error)
                })
            }

            fn usage(
                &self,
                _request: ruxex_model_provider::UsageRequest,
            ) -> ruxex_model_provider::ProviderFuture<'_, ruxex_model_provider::ProviderResult<ProviderUsage>>
            {
                ProviderRuntime::usage(self)
            }
        }
    };
}

fn map_provider_auth_error(error: ProviderAuthError) -> ruxex_model_provider::ProviderError {
    match error {
        ProviderAuthError::Unavailable => ruxex_model_provider::ProviderError::Unsupported,
        ProviderAuthError::Failed(message) => {
            ruxex_model_provider::ProviderError::Provider(message)
        }
    }
}

impl_model_provider_for_runtime!(ResponsesModelProvider);

fn environment_variable_present(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => !value.trim().is_empty(),
        Err(std::env::VarError::NotUnicode(_)) => true,
        Err(std::env::VarError::NotPresent) => false,
    }
}

pub(crate) type ModelProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Shared runtime model provider handle.
pub(super) type SharedProviderRuntime = Arc<dyn ProviderRuntime>;

#[cfg(test)]
fn create_runtime_provider(
    provider_info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
) -> SharedProviderRuntime {
    Arc::new(ResponsesModelProvider::new(
        provider_info,
        credential_manager,
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
    ))
}

/// Creates the explicit Responses-compatible provider for configured metadata.
pub fn create_responses_provider(
    provider_info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
) -> ruxex_model_provider::SharedModelProvider {
    create_responses_provider_with_http_client_factory(
        provider_info,
        credential_manager,
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
    )
}

/// Creates the explicit Responses-compatible provider with the host's transport policy.
pub fn create_responses_provider_with_http_client_factory(
    provider_info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
    http_client_factory: HttpClientFactory,
) -> ruxex_model_provider::SharedModelProvider {
    Arc::new(ResponsesModelProvider::new(
        provider_info,
        credential_manager,
        http_client_factory,
    ))
}

/// Responses-compatible provider backed by configured `ModelProviderInfo`.
#[derive(Clone, Debug)]
pub struct ResponsesModelProvider {
    provider_id: Option<String>,
    info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
    http_client_factory: HttpClientFactory,
}

impl ResponsesModelProvider {
    fn new(
        provider_info: ModelProviderInfo,
        credential_manager: Option<Arc<CredentialManager>>,
        http_client_factory: HttpClientFactory,
    ) -> Self {
        let credential_manager =
            credential_manager_for_provider(credential_manager, &provider_info);
        Self {
            provider_id: provider_info.id.clone(),
            info: provider_info,
            credential_manager,
            http_client_factory,
        }
    }
}

impl ModelProviderFactory for ResponsesModelProvider {
    fn provider_descriptor() -> ProviderDescriptor {
        ProviderDescriptor::new("responses", CURRENT_PROVIDER_PROTOCOL_VERSION)
    }

    fn new(init: ruxex_model_provider::ProviderInit) -> Self {
        Self::new(
            init.provider_info,
            Some(init.credential_manager),
            init.http_client_factory,
        )
    }
}

impl ProviderRuntime for ResponsesModelProvider {
    fn http_client_factory(&self) -> HttpClientFactory {
        self.http_client_factory.clone()
    }

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
            websocket: self.info.supports_websockets,
            supports_tool_strict_mode: self.info.supports_tool_strict_mode,
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

    fn auth(&self) -> ModelProviderFuture<'_, Option<RuxexAuth>> {
        Box::pin(
            async move { provider_auth(self.credential_manager.as_ref(), None, &self.info).await },
        )
    }

    fn account_state(&self) -> ProviderAccountResult {
        let auth = self
            .credential_manager
            .as_ref()
            .and_then(|credential_manager| cached_provider_auth(credential_manager));
        let auth_invalid = auth.as_ref().is_some_and(|auth| {
            self.credential_manager
                .as_ref()
                .is_some_and(|credential_manager| {
                    credential_manager.refresh_failure_for_auth(auth).is_some()
                })
        });
        let configured_auth = self.info.experimental_bearer_token.is_some()
            || matches!(self.info.api_key(), Ok(Some(_)));
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
                    cached_provider_auth(credential_manager).and_then(|auth| auth.get_account_id())
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

    use pretty_assertions::assert_eq;
    use ruxex_http_client::HttpClientFactory;
    use ruxex_http_client::OutboundProxyPolicy;
    use ruxex_model_provider_info::WireApi;
    use ruxex_models_manager::manager::ModelCatalogSource;
    use ruxex_models_manager::manager::RefreshStrategy;
    use ruxex_models_manager::manager::build_models_manager;
    use ruxex_protocol::config_types::ModelProviderAuthInfo;
    use ruxex_protocol::model_types::ModelCatalog;
    use ruxex_protocol::model_types::ModelInfo;
    use serde_json::json;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::header_regex;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    use super::*;
    use ruxex_model_provider::ProviderModelCatalogSource;

    #[test]
    fn factory_returns_provider_neutral_contract() {
        let _provider: ruxex_model_provider::SharedModelProvider = create_responses_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );
    }

    #[derive(Debug)]
    struct ForeignCredentialSnapshot;

    impl ProviderCredentialSnapshot for ForeignCredentialSnapshot {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn runtime_credentials_require_the_provider_owned_snapshot() {
        assert!(matches!(
            runtime_credentials(None),
            Err(ruxex_model_provider::ProviderError::Provider(message))
                if message.contains("missing its credential snapshot")
        ));

        let foreign: SharedProviderCredentialSnapshot = Arc::new(ForeignCredentialSnapshot);
        assert!(matches!(
            runtime_credentials(Some(&foreign)),
            Err(ruxex_model_provider::ProviderError::Provider(message))
                if message.contains("belongs to another provider")
        ));
    }

    #[test]
    fn configured_provider_selects_chat_runtime_api() {
        let provider = ResponsesModelProvider::new(
            ModelProviderInfo {
                id: Some("chat-provider".to_string()),
                wire_api: WireApi::Chat,
                ..ModelProviderInfo::create_responses_provider(None)
            },
            /*credential_manager*/ None,
            HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        );

        assert!(matches!(provider.api(), ProviderApi::Chat(_)));
    }

    #[test]
    fn provider_descriptor_uses_explicit_provider_id_and_round_trips() {
        let provider = create_responses_provider(
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
        let provider = create_responses_provider(
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
        let provider = create_responses_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        let capabilities = provider.capabilities();
        assert!(capabilities.model_listing);
        assert!(capabilities.streaming);
        assert!(!capabilities.usage);
        assert!(!capabilities.interactive_login);
    }

    #[tokio::test]
    async fn configured_provider_implements_neutral_lifecycle_directly() {
        let provider = create_responses_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        assert_eq!(provider.descriptor().provider_kind, "Responses provider");
        assert!(matches!(
            provider.delete().await,
            Err(ruxex_model_provider::ProviderError::Unsupported)
        ));
    }

    #[test]
    fn responses_provider_declares_history_notes_capability() {
        let provider = create_responses_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        assert!(provider.capabilities().history_notes);
    }

    #[test]
    fn configured_provider_reports_standalone_web_search_capability() {
        let provider = create_responses_provider(
            ModelProviderInfo {
                supports_standalone_web_search: true,
                ..ModelProviderInfo::default()
            },
            /*credential_manager*/ None,
        );

        assert!(provider.capabilities().standalone_web_search);
    }

    #[test]
    fn configured_provider_reports_tool_strict_mode_capability() {
        let provider = create_responses_provider(
            ModelProviderInfo {
                supports_tool_strict_mode: true,
                ..ModelProviderInfo::default()
            },
            /*credential_manager*/ None,
        );

        assert!(provider.capabilities().supports_tool_strict_mode);
    }

    #[tokio::test]
    async fn provider_exposes_neutral_login_lifecycle() {
        let provider = create_responses_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        let result = provider
            .login(std::collections::BTreeMap::from([(
                "api_key".to_string(),
                "api-key".to_string(),
            )]))
            .await;
        assert!(matches!(
            result,
            Err(ruxex_model_provider::ProviderError::Unsupported)
        ));

        let logout_result = provider.logout().await;
        assert!(matches!(
            logout_result,
            Err(ruxex_model_provider::ProviderError::Unsupported)
        ));
    }

    #[test]
    fn provider_model_catalog_source_preserves_provider_capabilities() {
        let mut info = ModelProviderInfo::create_responses_provider(None);
        info.base_url = Some("https://example.test".to_string());
        // Use an environment variable that is present in the test process so
        // identity generation does not fail before the capability assertions.
        info.env_key = Some("PATH".to_string());
        let provider = create_responses_provider(info, /*credential_manager*/ None);
        let source = ProviderModelCatalogSource::new(provider);

        assert!(source.supports_api_key_models());
        assert!(source.supports_remote_models());
        assert!(!source.has_command_auth());
        assert!(source.identity().is_some());
    }

    #[test]
    fn responses_provider_keeps_concrete_runtime_api() {
        let provider = create_runtime_provider(
            ModelProviderInfo::create_responses_provider(None),
            /*credential_manager*/ None,
        );

        assert!(matches!(provider.api(), ProviderApi::Responses));
    }

    #[test]
    fn provider_implementation_exposes_auth_environment_without_provider_info() {
        let mut info = ModelProviderInfo::create_responses_provider(None);
        info.env_key = Some("RUXEX_TEST_PROVIDER_KEY".to_string());
        let provider = create_responses_provider(info, /*credential_manager*/ None);

        let environment = provider.auth_environment();

        assert!(!environment.host_api_key_env_enabled);
        assert!(environment.provider_env_key_configured);
        assert_eq!(environment.provider_env_key_present, Some(false));
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

    fn test_ruxex_home() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ruxex-model-provider-test-{}", std::process::id()))
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
            supports_tool_strict_mode: false,
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
    async fn provider_auth_uses_shared_runtime_credentials() {
        let home = tempfile::tempdir().expect("temporary auth home");
        let credential_manager = Arc::new(
            CredentialManager::new(
                home.path().to_path_buf(),
                false,
                ruxex_login::AuthCredentialsStoreMode::File,
                None,
                None,
                ruxex_login::AuthKeyringBackendKind::default(),
                ruxex_login::AuthRouteConfig::from_http_client_factory(HttpClientFactory::new(
                    OutboundProxyPolicy::ReqwestDefault,
                )),
            )
            .await,
        );
        credential_manager
            .login(ruxex_login::ProviderAuth::api_key("a-secret"))
            .await
            .expect("provider A login");

        let provider_a = create_runtime_provider(
            ModelProviderInfo {
                id: Some("provider-a".to_string()),
                ..ModelProviderInfo::create_responses_provider(None)
            },
            Some(Arc::clone(&credential_manager)),
        );
        let provider_b = create_runtime_provider(
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
        assert_eq!(
            provider_b
                .api_auth()
                .await
                .expect("provider B auth")
                .to_auth_headers()
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer a-secret")
        );
    }

    #[test]
    fn responses_provider_enables_remote_compaction() {
        let provider = create_responses_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*credential_manager*/ None,
        );

        assert_eq!(
            provider.capabilities(),
            ProviderCapabilities {
                history_notes: true,
                remote_compaction: RemoteCompactionSupport::V2,
                websocket: true,
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
            let provider =
                create_responses_provider(provider_info, /*credential_manager*/ None);
            assert_eq!(provider.capabilities().remote_compaction, expected);
        }
    }

    #[test]
    fn create_responses_provider_builds_command_credential_manager_without_base_manager() {
        let provider = create_runtime_provider(
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
        let provider = create_runtime_provider(
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
        let provider = create_runtime_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            Some(CredentialManager::from_auth_for_testing(
                RuxexAuth::from_api_key("provider-api-key"),
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
        let provider = create_responses_provider(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*credential_manager*/ None,
        );

        assert_eq!(
            provider
                .usage(ruxex_model_provider::UsageRequest { model: None })
                .await,
            Ok(ProviderUsage::Unsupported)
        );
    }

    #[test]
    fn custom_non_responses_provider_returns_no_account_state() {
        let provider = create_runtime_provider(
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
        let credential_manager =
            CredentialManager::from_auth_for_testing(RuxexAuth::from_api_key("ambient-api-key"));
        let provider = create_responses_provider(provider_info, Some(credential_manager.clone()));

        let manager = build_models_manager(
            test_ruxex_home(),
            Arc::new(ProviderModelCatalogSource::new(provider)),
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
