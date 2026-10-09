//! Session- and turn-scoped helpers for talking to model provider APIs.
//!
//! `ModelClient` is intended to live for the lifetime of a Genex session and holds the stable
//! configuration and state needed to talk to the mounted provider (auth, conversation id,
//! and transport fallback state).
//!
//! Per-turn settings (model selection, reasoning controls, telemetry context, and turn metadata)
//! are passed explicitly to streaming and unary methods so that the turn lifetime is visible at the
//! call site.
//!
//! A [`ModelClientSession`] is created per turn and is used to stream one or more Responses API
//! requests during that turn. It caches a Responses WebSocket connection (opened lazily) and stores
//! per-turn state such as the `x-genex-turn-state` token used for sticky routing.
//! Cached connections, incremental response state, and turn routing are discarded when auth
//! ownership changes.
//!
//! WebSocket prewarm is a v2-only `response.create` with `generate=false`; it waits for completion
//! so the next request can reuse the same connection and `previous_response_id`.
//!
//! Turn execution performs prewarm as a best-effort step before the first stream request so the
//! subsequent request can reuse the same connection.
//!
//! ## Retry-Budget Tradeoff
//!
//! WebSocket prewarm is treated as the first websocket connection attempt for a turn. If it
//! fails, normal stream retry/fallback logic handles recovery on the same turn.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use crate::GenexResponsesHeaders;
use async_channel::Sender;
use genex_api::ApiError;
use genex_api::Compression;
use genex_api::Reasoning;
use genex_api::ReasoningContext;
use genex_api::RequestTelemetry;
use genex_api::ResponseCreateWsRequest;
use genex_api::ResponsesApiRequest;
use genex_api::ResponsesOptions as ApiResponsesOptions;
use genex_api::ResponsesWsRequest;
use genex_api::SseTelemetry;
use genex_api::StreamOptions;
use genex_api::TransportError;
use genex_api::build_session_headers;
use genex_api::create_text_param_for_request;
use genex_api::response_create_client_metadata;
use genex_http_client::HttpClientFactory;
use genex_login::default_client::add_originator_header;
use genex_otel::SessionTelemetry;
use genex_otel::WEBSOCKET_CONTINUATION_COUNT_METRIC;
use genex_otel::current_span_w3c_trace_context;
use genex_protocol::ResponseItemId;

use eventsource_stream::Event;
use eventsource_stream::EventStreamError;
use futures::StreamExt;
use genex_protocol::ThreadId;
use genex_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use genex_protocol::config_types::Verbosity as VerbosityConfig;
use genex_protocol::model_types::ModelInfo;
use genex_protocol::model_types::ReasoningEffort as ReasoningEffortConfig;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::AuthRecoveryEvent;
use genex_protocol::protocol::Event as ProtocolEvent;
use genex_protocol::protocol::EventMsg;
use genex_protocol::protocol::SessionSource;
use genex_protocol::protocol::W3cTraceContext;
use genex_rollout_trace::InferenceTraceAttempt;
use genex_rollout_trace::InferenceTraceContext;
use genex_tools::create_tools_json_for_chat_completions_api;
use genex_tools::create_tools_json_for_responses_api;
use genex_tools::create_tools_json_for_responses_lite;
use genex_tools::create_tools_raw_json_for_responses_api;
use http::HeaderMap as ApiHeaderMap;
use http::HeaderValue;
use http::StatusCode;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::instrument;
use tracing::trace;
use tracing::warn;
use uuid::Uuid;

use crate::attestation::AttestationContext;
use crate::attestation::AttestationProvider;
use crate::attestation::X_OAI_ATTESTATION_HEADER;
use crate::client_common::Prompt;
use crate::client_common::ResponseEvent;
use crate::client_common::ResponseStream;
use crate::context::BaseInstructionsFragment;
use crate::context::ContextualUserFragment;
use crate::cyber_access_program;
use crate::responses_metadata::GenexResponsesMetadata;
use genex_model_provider::ProviderConnectionKey;
use genex_model_provider::ProviderChatEvent;
use genex_model_provider::ProviderError;
use genex_model_provider::ProviderRequestAuth;
use genex_model_provider::ProviderUnauthorizedRecovery;
use genex_model_provider::ProviderWebsocketConnectRequest;
use genex_model_provider::ProviderWebsocketEvent;
use genex_model_provider::ProviderWebsocketRequest;
use genex_model_provider::ProviderWebsocketTelemetry;
use genex_model_provider::ProviderWireApi;
use genex_model_provider::SharedModelProvider;
use genex_model_provider::SharedProviderWebsocketConnection;
use genex_model_provider::ProviderTransportContext;
#[cfg(test)]
use genex_model_provider_info::DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result;
use genex_response_debug_context::extract_response_debug_context;
use genex_response_debug_context::telemetry_transport_error_message;

pub const X_GENEX_INSTALLATION_ID_HEADER: &str = "x-genex-installation-id";
pub const X_GENEX_ROUTING_HINT_HEADER: &str = "x-genex-routing-hint";
pub const X_GENEX_TURN_STATE_HEADER: &str = "x-genex-turn-state";
pub const X_GENEX_TURN_METADATA_HEADER: &str = "x-genex-turn-metadata";
pub const X_GENEX_PARENT_THREAD_ID_HEADER: &str = "x-genex-parent-thread-id";
pub const X_GENEX_WINDOW_ID_HEADER: &str = "x-genex-window-id";
pub const X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER: &str =
    "x-responsesapi-include-timing-metrics";
const X_GENEX_WS_STREAM_REQUEST_START_MS_CLIENT_METADATA_KEY: &str =
    "x-genex-ws-stream-request-start-ms";
const WS_REQUEST_HEADER_RESPONSES_LITE_CLIENT_METADATA_KEY: &str =
    "ws_request_header_x_provider_internal_genex_responses_lite";
const X_PROVIDER_INTERNAL_RESPONSES_LITE_HEADER: &str = "x-provider-internal-genex-responses-lite";
#[cfg(test)]
pub(crate) const WEBSOCKET_CONNECT_TIMEOUT: Duration =
    Duration::from_millis(DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS);

fn session_telemetry_for_request(
    session_telemetry: &SessionTelemetry,
    request: &ResponsesApiRequest,
) -> SessionTelemetry {
    session_telemetry.clone().with_inference_request(
        request.service_tier.as_deref(),
        request
            .reasoning
            .as_ref()
            .and_then(|reasoning| reasoning.effort.as_ref()),
    )
}

/// Session-scoped state shared by all [`ModelClient`] clones.
///
/// This is intentionally kept minimal so `ModelClient` does not need to hold a full `Config`. Most
/// configuration is per turn and is passed explicitly to streaming/unary methods.
#[derive(Debug)]
struct ModelClientState {
    thread_id: ThreadId,
    provider: SharedModelProvider,
    session_source: SessionSource,
    originator: String,
    model_verbosity: Option<VerbosityConfig>,
    content_item_kinds_enabled: bool,
    enable_request_compression: bool,
    include_timing_metrics: bool,
    beta_features_header: Option<String>,
    concurrent_reasoning_summaries_enabled: bool,
    include_attestation: bool,
    attestation_provider: Option<Arc<dyn AttestationProvider>>,
    disable_websockets: AtomicBool,
    cached_websocket_session: StdMutex<WebsocketSession>,
}

/// Resolved API client setup for a single request attempt.
///
/// Keeping this as a single bundle ensures prewarm and normal request paths
/// share the same auth/provider setup flow.
struct CurrentClientSetup {
    auth: ProviderRequestAuth,
    auth_owner_generation: Option<u64>,
    connection_key: ProviderConnectionKey,
}

#[derive(Clone, Copy)]
struct RequestRouteTelemetry {
    endpoint: &'static str,
}

impl RequestRouteTelemetry {
    fn for_endpoint(endpoint: &'static str) -> Self {
        Self { endpoint }
    }
}

/// A session-scoped client for model-provider API calls.
///
/// This holds configuration and state that should be shared across turns within a Genex session
/// (auth, mounted provider, thread id, and transport fallback state).
///
/// WebSocket fallback is session-scoped: once a turn activates the HTTP fallback, subsequent turns
/// will also use HTTP for the remainder of the session.
///
/// Turn-scoped settings (model selection, reasoning controls, telemetry context, and turn
/// metadata) are passed explicitly to the relevant methods to keep turn lifetime visible at the
/// call site.
#[derive(Debug, Clone)]
pub struct ModelClient {
    state: Arc<ModelClientState>,
    prompt_cache_key_override: Option<String>,
    genex_responses_headers: Option<Arc<GenexResponsesHeaders>>,
    event_sender: Option<Sender<ProtocolEvent>>,
    http_client_factory: HttpClientFactory,
    restored_history: bool,
}

/// A turn-scoped streaming session created from a [`ModelClient`].
///
/// The session establishes a Responses WebSocket connection lazily and reuses it across multiple
/// requests within the turn. It also caches per-turn state:
///
/// - The last full request, so subsequent calls can reuse incremental websocket request payloads
///   only when the current request is an incremental extension of the previous one.
/// - The `x-genex-turn-state` sticky-routing token, which must be replayed for all requests within
///   the same turn.
///
/// Create a fresh `ModelClientSession` for each Genex turn. Reusing it across turns would replay
/// the previous turn's sticky-routing token into the next turn, which violates the client/server
/// contract and can cause routing bugs.
pub struct ModelClientSession {
    client: ModelClient,
    websocket_session: WebsocketSession,
    /// Turn state for sticky routing.
    ///
    /// This is an `OnceLock` that stores the turn state value received from the server
    /// on turn start via the `x-genex-turn-state` response header. Once set, this value
    /// should be sent back to the server in the `x-genex-turn-state` request header for
    /// all subsequent requests within the same turn to maintain sticky routing.
    ///
    /// This is a contract between the client and server: we receive it at turn start,
    /// keep sending it unchanged between turn requests (e.g., for retries, incremental
    /// appends, or continuation requests), and must not send it between different turns.
    /// An auth ownership change clears it so the new owner gets fresh routing state.
    turn_state: Arc<OnceLock<String>>,
}

#[derive(Debug, Clone)]
struct LastResponse {
    response_id: String,
    items_added: Vec<ResponseItem>,
}

struct WebsocketContinuation {
    response_id: String,
    items: Vec<ResponseItem>,
    from_untraced_warmup: bool,
}

#[derive(Debug, Default)]
struct WebsocketSession {
    connection: Option<SharedProviderWebsocketConnection>,
    responses_headers: ApiHeaderMap,
    /// Owner of the cached state, including before a connection is opened.
    auth_owner_generation: Option<u64>,
    connection_key: Option<ProviderConnectionKey>,
    last_request: Option<ResponsesApiRequest>,
    last_response_rx: Option<oneshot::Receiver<LastResponse>>,
    last_response_from_untraced_warmup: bool,
    connection_reused: StdMutex<bool>,
    continuation_reset_reason: Option<&'static str>,
}

// This is intentionally not a `PartialEq` implementation: request equality includes `input` and
// `client_metadata`, while websocket reuse compares input and late tool-result metadata separately.
// Access programs are authorized per response, including continuations, without replaying input.
// Keep the destructuring exhaustive so new request fields require an explicit reuse decision.
fn responses_request_properties_match(
    previous: &ResponsesApiRequest,
    current: &ResponsesApiRequest,
) -> bool {
    let ResponsesApiRequest {
        model: previous_model,
        instructions: previous_instructions,
        input: _,
        tools: previous_tools,
        tool_choice: previous_tool_choice,
        parallel_tool_calls: previous_parallel_tool_calls,
        reasoning: previous_reasoning,
        store: previous_store,
        stream: previous_stream,
        stream_options: _,
        include: previous_include,
        service_tier: previous_service_tier,
        prompt_cache_key: previous_prompt_cache_key,
        text: previous_text,
        client_metadata: _,
        access_programs: _,
    } = previous;
    let ResponsesApiRequest {
        model: current_model,
        instructions: current_instructions,
        input: _,
        tools: current_tools,
        tool_choice: current_tool_choice,
        parallel_tool_calls: current_parallel_tool_calls,
        reasoning: current_reasoning,
        store: current_store,
        stream: current_stream,
        stream_options: _,
        include: current_include,
        service_tier: current_service_tier,
        prompt_cache_key: current_prompt_cache_key,
        text: current_text,
        client_metadata: _,
        access_programs: _,
    } = current;

    previous_model == current_model
        && previous_instructions == current_instructions
        && previous_tools == current_tools
        && previous_tool_choice == current_tool_choice
        && previous_parallel_tool_calls == current_parallel_tool_calls
        && previous_reasoning == current_reasoning
        && previous_store == current_store
        && previous_stream == current_stream
        // Stream options control delivery for this response, not the context
        // referenced by `previous_response_id`.
        && previous_include == current_include
        && previous_service_tier == current_service_tier
        && previous_prompt_cache_key == current_prompt_cache_key
        && previous_text == current_text
}

fn response_items_equal_ignoring_internal_metadata(
    previous: &ResponseItem,
    current: &ResponseItem,
) -> bool {
    if previous == current {
        return true;
    }

    // Late results update an already-sent output. A delta cannot carry that update.
    if !previous.has_same_tool_result_metadata(current) {
        return false;
    }

    let mut previous = previous.clone();
    previous.clear_internal_chat_message_metadata_passthrough();
    let mut current = current.clone();
    current.clear_internal_chat_message_metadata_passthrough();
    previous == current
}

impl WebsocketSession {
    fn reset(&mut self, reason: Option<&'static str>) {
        // Per-socket backend metrics call a resend after reconnect "initial".
        // Retain the loss reason across reconnects/turns until the next send.
        let continuation_reset_reason = self
            .continuation_reset_reason
            .or_else(|| self.last_request.as_ref().and(reason));
        *self = Self {
            auth_owner_generation: self.auth_owner_generation,
            continuation_reset_reason,
            ..Default::default()
        };
    }

    fn set_connection_reused(&self, connection_reused: bool) {
        *self
            .connection_reused
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = connection_reused;
    }

    fn connection_reused(&self) -> bool {
        *self
            .connection_reused
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

enum WebsocketStreamOutcome {
    Stream(ResponseStream),
    FallbackToHttp,
}

impl ModelClient {
    /// Creates a session-scoped model client from the runtime's shared provider.
    ///
    /// A runtime owns one provider instance. Sessions must retain that instance
    /// instead of reconstructing a provider from persisted or client-side
    /// configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_provider(
        model_provider: SharedModelProvider,
        thread_id: ThreadId,
        session_source: SessionSource,
        originator: String,
        model_verbosity: Option<VerbosityConfig>,
        content_item_kinds_enabled: bool,
        enable_request_compression: bool,
        include_timing_metrics: bool,
        beta_features_header: Option<String>,
        concurrent_reasoning_summaries_enabled: bool,
        attestation_provider: Option<Arc<dyn AttestationProvider>>,
        http_client_factory: HttpClientFactory,
    ) -> Self {
        Self::new_with_provider_inner(
            model_provider,
            thread_id,
            session_source,
            originator,
            model_verbosity,
            content_item_kinds_enabled,
            enable_request_compression,
            include_timing_metrics,
            beta_features_header,
            concurrent_reasoning_summaries_enabled,
            attestation_provider,
            http_client_factory,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_provider_inner(
        model_provider: SharedModelProvider,
        thread_id: ThreadId,
        session_source: SessionSource,
        originator: String,
        model_verbosity: Option<VerbosityConfig>,
        content_item_kinds_enabled: bool,
        enable_request_compression: bool,
        include_timing_metrics: bool,
        beta_features_header: Option<String>,
        concurrent_reasoning_summaries_enabled: bool,
        attestation_provider: Option<Arc<dyn AttestationProvider>>,
        http_client_factory: HttpClientFactory,
    ) -> Self {
        let include_attestation = model_provider.supports_attestation();
        Self {
            state: Arc::new(ModelClientState {
                thread_id,
                provider: model_provider,
                session_source,
                originator,
                model_verbosity,
                content_item_kinds_enabled,
                enable_request_compression,
                include_timing_metrics,
                beta_features_header,
                concurrent_reasoning_summaries_enabled,
                include_attestation,
                attestation_provider,
                disable_websockets: AtomicBool::new(false),
                cached_websocket_session: StdMutex::new(WebsocketSession::default()),
            }),
            prompt_cache_key_override: None,
            genex_responses_headers: None,
            event_sender: None,
            http_client_factory,
            restored_history: false,
        }
    }

    pub(crate) fn with_restored_history(mut self, restored_history: bool) -> Self {
        self.restored_history = restored_history;
        self
    }

    pub(crate) fn with_session_context(
        mut self,
        prompt_cache_key_override: Option<String>,
        event_sender: Sender<ProtocolEvent>,
        genex_responses_headers: Option<Arc<GenexResponsesHeaders>>,
    ) -> Self {
        self.prompt_cache_key_override = prompt_cache_key_override;
        self.event_sender = Some(event_sender);
        self.genex_responses_headers = genex_responses_headers;
        self
    }

    fn prompt_cache_key(&self, responses_metadata: &GenexResponsesMetadata) -> String {
        if let Some(prompt_cache_key) = &self.prompt_cache_key_override {
            return prompt_cache_key.clone();
        }

        if let SessionSource::Internal(source) = &self.state.session_source
            && let Some(parent_thread_id) = responses_metadata.parent_thread_id
        {
            return format!("{source}:{parent_thread_id}");
        }

        responses_metadata.session_id.clone()
    }

    // Hosted Responses clients may derive cache affinity from the session-id header.
    // Keep the actual session identity in turn metadata, hooks, and history/notes requests.
    fn responses_session_id(&self, metadata: &GenexResponsesMetadata) -> String {
        if self.state.session_source.is_non_root_agent() {
            metadata.session_id.clone()
        } else {
            self.prompt_cache_key(metadata)
        }
    }

    /// Creates a fresh turn-scoped streaming session.
    ///
    /// This constructor does not perform network I/O itself; the session opens a websocket lazily
    /// when the first stream request is issued.
    pub fn new_session(&self) -> ModelClientSession {
        let auth_owner_generation = None;
        let mut websocket_session = self.take_cached_websocket_session();
        if websocket_session.auth_owner_generation != auth_owner_generation {
            // Drop the old owner's cache before this turn can establish fresh routing state.
            websocket_session.reset(Some("other"));
            websocket_session.auth_owner_generation = auth_owner_generation;
        }
        ModelClientSession {
            client: self.clone(),
            websocket_session,
            turn_state: Arc::new(OnceLock::new()),
        }
    }

    fn take_cached_websocket_session(&self) -> WebsocketSession {
        let mut cached_websocket_session = self
            .state
            .cached_websocket_session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *cached_websocket_session)
    }

    fn store_cached_websocket_session(&self, websocket_session: WebsocketSession) {
        *self
            .state
            .cached_websocket_session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = websocket_session;
    }

    pub(crate) fn force_http_fallback(
        &self,
        session_telemetry: &SessionTelemetry,
        _model_info: &ModelInfo,
    ) -> bool {
        let websocket_enabled = self.responses_websocket_enabled();
        let activated =
            websocket_enabled && !self.state.disable_websockets.swap(true, Ordering::Relaxed);
        if activated {
            warn!("falling back to HTTP");
            session_telemetry.counter(
                "genex.transport.fallback_to_http",
                /*inc*/ 1,
                &[("from_wire_api", "responses_websocket")],
            );
        }

        self.store_cached_websocket_session(WebsocketSession::default());
        activated
    }

    fn build_responses_compatibility_headers(
        &self,
        responses_metadata: &GenexResponsesMetadata,
    ) -> ApiHeaderMap {
        let extra_headers = responses_metadata.compatibility_headers();
        extra_headers
    }

    fn build_ws_client_metadata(
        &self,
        responses_metadata: &GenexResponsesMetadata,
        use_responses_lite: bool,
    ) -> HashMap<String, String> {
        let mut client_metadata = responses_metadata.client_metadata();
        if use_responses_lite {
            client_metadata.insert(
                WS_REQUEST_HEADER_RESPONSES_LITE_CLIENT_METADATA_KEY.to_string(),
                "true".to_string(),
            );
        }
        client_metadata
    }

    async fn generate_attestation_header_for(&self) -> Option<HeaderValue> {
        if !self.state.include_attestation {
            return None;
        }

        self.state
            .attestation_provider
            .as_ref()?
            .header_for_request(AttestationContext {
                thread_id: self.state.thread_id,
            })
            .await
    }

    fn build_reasoning(
        &self,
        model_info: &ModelInfo,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
    ) -> Reasoning {
        Reasoning {
            effort: effort
                .or_else(|| model_info.default_reasoning_level.clone())
                .map(|effort| model_info.resolve_reasoning_effort(effort)),
            summary: (model_info.supports_reasoning_summary_parameter
                && summary != ReasoningSummaryConfig::None)
                .then_some(summary),
            // When Responses Lite is disabled, omit context so Responses uses the default,
            // which is currently `current_turn`.
            context: model_info
                .use_responses_lite
                .then_some(ReasoningContext::AllTurns),
        }
    }

    pub(crate) fn build_responses_request(
        &self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<String>,
        responses_metadata: &GenexResponsesMetadata,
    ) -> Result<ResponsesApiRequest> {
        let mut input = prompt.get_formatted_input_for_request(model_info);
        let is_responses_provider =
            matches!(self.state.provider.wire_api(), ProviderWireApi::Responses);
        let (instructions, tools) = if model_info.use_responses_lite {
            // These prompt-only items are rebuilt on every request. Hash their visible payloads
            // within the thread so retries and resumed sessions preserve their identity.
            let prefix_namespace = Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                self.state.thread_id.to_string().as_bytes(),
            );
            let tools = if self.state.provider.capabilities().namespace_tools {
                create_tools_json_for_responses_lite(&prompt.tools)?
            } else {
                create_tools_json_for_responses_api(&prompt.tools)?
            };
            let mut prefix = vec![ResponseItem::AdditionalTools {
                id: Some(ResponseItemId::with_suffix(
                    "at",
                    Uuid::new_v5(&prefix_namespace, &serde_json::to_vec(&tools)?),
                )),
                role: "developer".to_string(),
                tools,
            }];
            if !prompt.base_instructions.text.is_empty() {
                let mut instructions = ContextualUserFragment::into(BaseInstructionsFragment(
                    prompt.base_instructions.text.clone(),
                ));
                instructions.set_id(Some(ResponseItemId::with_suffix(
                    "msg",
                    Uuid::new_v5(&prefix_namespace, prompt.base_instructions.text.as_bytes()),
                )));
                prefix.push(instructions);
            }
            input.splice(0..0, prefix);
            (String::new(), None)
        } else {
            (
                prompt.base_instructions.text.clone(),
                Some(create_tools_raw_json_for_responses_api(&prompt.tools)?.into()),
            )
        };
        if !is_responses_provider {
            for item in &mut input {
                item.clear_internal_chat_message_metadata_passthrough();
                if let ResponseItem::FunctionCall {
                    encrypted_function_args,
                    ..
                } = item
                {
                    *encrypted_function_args = None;
                }
            }
        }
        let reasoning = self.build_reasoning(model_info, effort, summary);
        let stream_options = (self.state.concurrent_reasoning_summaries_enabled
            && is_responses_provider
            && reasoning.summary.is_some())
        .then_some(StreamOptions {
            reasoning_summary_delivery: genex_api::ReasoningSummaryDelivery::SequentialCutoff,
        });
        let include = vec!["reasoning.encrypted_content".to_string()];
        let verbosity = if model_info.support_verbosity {
            self.state.model_verbosity.or(model_info.default_verbosity)
        } else {
            if self.state.model_verbosity.is_some() {
                warn!(
                    "model_verbosity is set but ignored as the model does not support verbosity: {}",
                    model_info.slug
                );
            }
            None
        };
        let text = create_text_param_for_request(
            verbosity,
            &prompt.output_schema,
            prompt.output_schema_strict,
        );
        let prompt_cache_key = Some(self.prompt_cache_key(responses_metadata));
        let service_tier = model_info.service_tier_for_request(service_tier);
        let request = ResponsesApiRequest {
            model: model_info.slug.clone(),
            instructions,
            input,
            tools,
            tool_choice: "auto".to_string(),
            parallel_tool_calls: prompt.parallel_tool_calls && !model_info.use_responses_lite,
            reasoning: Some(reasoning),
            store: false,
            stream: true,
            stream_options,
            include,
            service_tier,
            prompt_cache_key,
            text,
            client_metadata: Some(responses_metadata.client_metadata()),
            access_programs: None,
        };
        Ok(request)
    }

    fn filter_tool_result_metadata(input: &mut [ResponseItem]) {
        // Check the resolved destination only when sending, not for local budget estimates.
        // HTTP and WS (including v2 compaction) share this raw-metadata-only filter.
        // Internal tool-result metadata is only accepted by a trusted first-party endpoint.
        // This provider-neutral fork has no such endpoint, so always strip it before dispatch.
        let result_metadata_allowed = false;
        if !result_metadata_allowed {
            for item in input {
                item.clear_tool_result_metadata();
            }
        }
    }

    fn prepare_response_items_for_request(&self, input: &mut [ResponseItem]) {
        for item in input {
            if item.id().is_some_and(|id| !id.is_prefixed()) {
                item.set_id(/*new_id*/ None);
            }
            if !self.state.content_item_kinds_enabled {
                item.clear_content_item_kinds();
            }
        }
    }

    /// Returns whether the Responses-over-WebSocket transport is active for this session.
    ///
    /// WebSocket use is controlled by provider capability and session-scoped fallback state.
    pub fn responses_websocket_enabled(&self) -> bool {
        if !self.state.provider.capabilities().websocket
            || self.state.disable_websockets.load(Ordering::Relaxed)
        {
            return false;
        }

        true
    }

    /// Returns auth + provider configuration resolved from the current session auth state.
    ///
    /// This centralizes setup used by both prewarm and normal request paths so they stay in
    /// lockstep when auth/provider resolution changes.
    async fn current_client_setup(&self) -> Result<CurrentClientSetup> {
        let snapshot = self
            .state
            .provider
            .request_snapshot()
            .await
            .map_err(|error| GenexErr::Fatal(error.to_string()))?;
        if !snapshot.auth.authenticated {
            return Err(GenexErr::Fatal("provider is unauthenticated".to_string()));
        }
        Ok(CurrentClientSetup {
            auth_owner_generation: snapshot.auth.owner_generation,
            auth: snapshot.auth,
            connection_key: snapshot.connection_key,
        })
    }

    fn responses_headers(&self, auth: &ProviderRequestAuth, model: &str) -> ApiHeaderMap {
        self.genex_responses_headers
            .as_ref()
            .filter(|config| config.model == model && self.uses_managed_auth(auth))
            .map(|config| config.headers.clone())
            .unwrap_or_default()
    }

    fn uses_managed_auth(&self, auth: &ProviderRequestAuth) -> bool {
        auth.managed_auth
    }

    fn build_routing_hint_header(
        &self,
        auth: &ProviderRequestAuth,
        model: &str,
        service_tier: Option<&str>,
    ) -> Option<HeaderValue> {
        if !self.uses_managed_auth(auth) {
            return None;
        }

        let routing_hint = match service_tier {
            Some(tier) => format!("model={model};tier={tier}"),
            None => format!("model={model}"),
        };
        HeaderValue::from_str(&routing_hint).ok()
    }

    pub(crate) async fn prewarm_auth(&self) -> Result<()> {
        self.current_client_setup().await.map(|_| ())
    }

    /// Opens a websocket connection using the same header and telemetry wiring as normal turns.
    ///
    /// Both startup prewarm and in-turn `needs_new` reconnects call this path so handshake
    /// behavior remains consistent across both flows.
    #[allow(clippy::too_many_arguments)]
    async fn connect_websocket(
        &self,
        session_telemetry: &SessionTelemetry,
        responses_metadata: &GenexResponsesMetadata,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
        responses_headers: &ApiHeaderMap,
    ) -> genex_model_provider::ProviderResult<SharedProviderWebsocketConnection> {
        let mut headers = self.build_websocket_headers(responses_metadata).await;
        headers.extend(responses_headers.clone());
        let websocket_telemetry = ModelClientSession::build_websocket_telemetry(
            session_telemetry,
            auth_context.clone(),
            request_route_telemetry,
        );
        let websocket_connect_timeout = self.state.provider.websocket_connect_timeout();
        let start = Instant::now();
        let result = match tokio::time::timeout(
            websocket_connect_timeout,
            self.state
                .provider
                .connect_websocket(ProviderWebsocketConnectRequest {
                    transport: ProviderTransportContext::from_http_client_factory(
                        self.http_client_factory.clone(),
                    ),
                    extra_headers: neutral_headers(headers),
                    default_headers: neutral_headers(
                        genex_login::default_client::default_headers(),
                    ),
                    telemetry: Some(websocket_telemetry),
                }),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(ProviderError::Transient(
                "provider websocket connection timed out".to_string(),
            )),
        };
        let error_message = result.as_ref().err().map(ToString::to_string);
        session_telemetry.record_websocket_connect(
            start.elapsed(),
            None,
            error_message.as_deref(),
            auth_context.auth_header_attached,
            auth_context.auth_header_name,
            auth_context.retry_after_unauthorized,
            auth_context.recovery_mode,
            auth_context.recovery_phase,
            request_route_telemetry.endpoint,
            /*connection_reused*/ false,
            None,
            None,
        );
        result
    }

    /// Builds websocket handshake headers for both prewarm and turn-time reconnect.
    async fn build_websocket_headers(
        &self,
        responses_metadata: &GenexResponsesMetadata,
    ) -> ApiHeaderMap {
        let mut headers = build_responses_headers(
            self.state.beta_features_header.as_deref(),
            /*turn_state*/ None,
        );
        add_originator_header(&mut headers, self.state.originator.as_str());
        if let Ok(header_value) = HeaderValue::from_str(&responses_metadata.thread_id) {
            headers.insert("x-client-request-id", header_value);
        }
        headers.extend(build_session_headers(
            Some(self.responses_session_id(responses_metadata)),
            Some(responses_metadata.thread_id.to_string()),
        ));
        headers.extend(self.build_responses_compatibility_headers(responses_metadata));
        if let Some(routing_hint) = &responses_metadata.routing_hint {
            headers.insert(X_GENEX_ROUTING_HINT_HEADER, routing_hint.clone());
        }
        if let Some(header_value) = self.generate_attestation_header_for().await {
            headers.insert(X_OAI_ATTESTATION_HEADER, header_value);
        }
        if self.state.include_timing_metrics {
            headers.insert(
                X_RESPONSESAPI_INCLUDE_TIMING_METRICS_HEADER,
                HeaderValue::from_static("true"),
            );
        }
        headers
    }
}

impl Drop for ModelClientSession {
    fn drop(&mut self) {
        let websocket_session = std::mem::take(&mut self.websocket_session);
        self.client
            .store_cached_websocket_session(websocket_session);
    }
}

impl ModelClientSession {
    #[allow(clippy::too_many_arguments)]
    /// Builds shared Responses API transport options and request-body options.
    ///
    /// Keeping option construction in one place ensures request-scoped headers are consistent
    /// regardless of transport choice.
    async fn build_responses_options(
        &self,
        responses_metadata: &GenexResponsesMetadata,
        compression: Compression,
        use_responses_lite: bool,
    ) -> ApiResponsesOptions {
        ApiResponsesOptions {
            session_id: Some(self.client.responses_session_id(responses_metadata)),
            thread_id: Some(responses_metadata.thread_id.to_string()),
            extra_headers: {
                let mut headers = build_responses_headers(
                    self.client.state.beta_features_header.as_deref(),
                    Some(&self.turn_state),
                );
                add_originator_header(&mut headers, self.client.state.originator.as_str());
                headers.extend(
                    self.client
                        .build_responses_compatibility_headers(responses_metadata),
                );
                if let Some(header_value) = self.client.generate_attestation_header_for().await {
                    headers.insert(X_OAI_ATTESTATION_HEADER, header_value);
                }
                add_responses_lite_header(&mut headers, use_responses_lite);
                headers
            },
            compression,
            turn_state: Some(Arc::clone(&self.turn_state)),
        }
    }

    /// Checks whether the current request is an incremental extension of the previous request.
    /// We only reuse an incremental input delta when non-input request fields are unchanged and
    /// `input` extends or equals the previous known input. Server-returned output items
    /// are treated as part of the baseline so we do not resend them.
    fn get_incremental_items(
        &self,
        request: &ResponsesApiRequest,
        last_response: &LastResponse,
    ) -> Option<Vec<ResponseItem>> {
        let previous_request = self.websocket_session.last_request.as_ref()?;
        if !responses_request_properties_match(previous_request, request) {
            trace!("incremental request failed, websocket reuse properties didn't match");
            return None;
        }

        let response_items = &last_response.items_added;
        let previous_items_len = previous_request
            .input
            .len()
            .checked_add(response_items.len())?;
        let Some((request_items_to_compare, incremental_items)) =
            request.input.split_at_checked(previous_items_len)
        else {
            trace!("incremental request failed, incompatible request length");
            return None;
        };
        let previous_items = previous_request.input.iter().chain(response_items);
        if !previous_items
            .zip(request_items_to_compare)
            .all(|(previous, current)| {
                response_items_equal_ignoring_internal_metadata(previous, current)
            })
        {
            trace!("incremental request failed, items didn't match");
            return None;
        }
        Some(incremental_items.to_vec())
    }

    fn get_last_response(&mut self) -> Option<LastResponse> {
        self.websocket_session
            .last_response_rx
            .take()
            .and_then(|mut receiver| match receiver.try_recv() {
                Ok(last_response) => Some(last_response),
                Err(TryRecvError::Closed) | Err(TryRecvError::Empty) => None,
            })
    }

    fn prepare_websocket_request(
        &mut self,
        request: &ResponsesApiRequest,
    ) -> Option<WebsocketContinuation> {
        let last_response = self.get_last_response()?;
        let items = self.get_incremental_items(request, &last_response)?;

        if last_response.response_id.is_empty() {
            trace!("incremental request failed, no previous response id");
            return None;
        }

        Some(WebsocketContinuation {
            response_id: last_response.response_id,
            items,
            from_untraced_warmup: self.websocket_session.last_response_from_untraced_warmup,
        })
    }

    /// Opportunistically preconnects a websocket for this turn-scoped client session.
    ///
    /// This performs only connection setup; it never sends prompt payloads.
    pub async fn preconnect_websocket(
        &mut self,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        responses_metadata: &GenexResponsesMetadata,
    ) -> std::result::Result<(), ApiError> {
        if !self.client.responses_websocket_enabled() {
            return Ok(());
        }
        let client_setup = self.client.current_client_setup().await.map_err(|err| {
            ApiError::Stream(format!(
                "failed to build websocket prewarm client setup: {err}"
            ))
        })?;
        let connection_key = client_setup.connection_key.clone();
        if self.websocket_session.connection.is_some()
            && self.websocket_session.connection_key.as_ref() == Some(&connection_key)
        {
            return Ok(());
        }
        self.websocket_session.reset(Some("other"));
        let auth_context = AuthRequestTelemetryContext::new(
            &client_setup.auth,
            PendingUnauthorizedRetry::default(),
        );
        let responses_headers = self
            .client
            .responses_headers(&client_setup.auth, &model_info.slug);
        self.websocket_connection(WebsocketConnectParams {
            session_telemetry,
            connection_key,
            auth_owner_generation: client_setup.auth_owner_generation,
            responses_metadata,
            auth_context,
            request_route_telemetry: RequestRouteTelemetry::for_endpoint("/responses"),
            responses_headers: &responses_headers,
        })
        .await
        .map_err(provider_error_to_api_error)?;
        Ok(())
    }
    /// Returns a websocket connection for this turn.
    #[instrument(
        name = "model_client.websocket_connection",
        level = "info",
        skip_all,
        fields(
            provider = %self.client.state.provider.descriptor().provider_kind,
            wire_api = ?self.client.state.provider.wire_api(),
            transport = "responses_websocket",
            api.path = "/responses",
            turn.has_metadata_header = params.responses_metadata.has_turn_metadata()
        )
    )]
    async fn websocket_connection(
        &mut self,
        params: WebsocketConnectParams<'_>,
    ) -> genex_model_provider::ProviderResult<&SharedProviderWebsocketConnection> {
        let WebsocketConnectParams {
            session_telemetry,
            connection_key,
            auth_owner_generation,
            responses_metadata,
            auth_context,
            request_route_telemetry,
            responses_headers,
        } = params;
        let reset_reason = match self.websocket_session.connection.as_ref() {
            Some(_)
                if self.websocket_session.responses_headers != *responses_headers
                    || self.websocket_session.connection_key.as_ref() != Some(&connection_key) =>
            {
                Some("other")
            }
            Some(conn) => match conn.is_closed().await {
                Ok(true) => Some("connection_closed"),
                Ok(false) => None,
                Err(_) => Some("connection_error"),
            },
            None => None,
        };
        let needs_new = self.websocket_session.connection.is_none() || reset_reason.is_some();
        // Resolving an external auth provider can change ownership during client setup.
        let owner_changed = self.websocket_session.auth_owner_generation != auth_owner_generation;
        if owner_changed {
            self.turn_state = Arc::new(OnceLock::new());
        }

        if needs_new || owner_changed {
            self.websocket_session.reset(if owner_changed {
                Some("other")
            } else {
                reset_reason
            });
            let new_conn = self
                .client
                .connect_websocket(
                    session_telemetry,
                    responses_metadata,
                    auth_context,
                    request_route_telemetry,
                    responses_headers,
                )
                .await?;
            self.websocket_session.connection = Some(new_conn);
            self.websocket_session.responses_headers = responses_headers.clone();
            self.websocket_session.auth_owner_generation = auth_owner_generation;
            self.websocket_session.connection_key = Some(connection_key);
            self.websocket_session
                .set_connection_reused(/*connection_reused*/ false);
        } else {
            self.websocket_session
                .set_connection_reused(/*connection_reused*/ true);
        }

        self.websocket_session.connection.as_ref().ok_or_else(|| {
            ProviderError::Transport("websocket connection is unavailable".to_string())
        })
    }

    fn responses_request_compression(&self, auth: &ProviderRequestAuth) -> Compression {
        if self.client.state.enable_request_compression
            && auth.managed_auth
            && matches!(
                self.client.state.provider.wire_api(),
                ProviderWireApi::Responses
            )
        {
            Compression::Zstd
        } else {
            Compression::None
        }
    }

    /// Streams a turn via the legacy Chat Completions API.
    ///
    /// Providers opt into this isolated transport with `wire_api = "chat"`; the default
    /// Responses transport does not pass through this path.
    #[allow(clippy::too_many_arguments)]
    async fn stream_chat_completions(
        &self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        responses_metadata: &GenexResponsesMetadata,
        inference_trace: &InferenceTraceContext,
    ) -> Result<ResponseStream> {
        if prompt.output_schema.is_some() {
            return Err(genex_protocol::error::GenexErr::UnsupportedOperation(
                "output_schema is not supported for Chat Completions API".to_string(),
            ));
        }

        let tools = create_tools_json_for_chat_completions_api(&prompt.tools)?;
        let request = genex_model_provider::ChatRequest {
            model: model_info.slug.clone(),
            instructions: Some(prompt.base_instructions.text.clone()),
            input: prompt.input.clone(),
            tools,
            metadata: std::collections::BTreeMap::new(),
            conversation_id: Some(responses_metadata.session_id.clone()),
            session_source: Some(self.client.state.session_source.clone()),
            payload: None,
            transport: genex_model_provider::ChatTransportOptions {
                transport: ProviderTransportContext::from_http_client_factory(
                    self.client.http_client_factory.clone(),
                ),
                session_id: None,
                thread_id: None,
                extra_headers: std::collections::BTreeMap::new(),
                compression: genex_model_provider::ProviderRequestCompression::None,
                turn_state: None,
            },
        };
        let provider_stream = self
            .client
            .state
            .provider
            .chat(request)
            .await
            .map_err(|error| GenexErr::Fatal(error.to_string()))?;
        let api_stream = provider_stream.map(|event| event.map(provider_event_to_response_event));
        let (stream, _) = map_response_events(
            None,
            api_stream,
            session_telemetry.clone(),
            inference_trace.start_attempt(),
        );
        Ok(stream)
    }

    /// Streams a turn via the provider Responses API.
    ///
    /// Handles reasoning summaries, verbosity, and the `text` controls used for output schemas.
    #[allow(clippy::too_many_arguments)]
    #[instrument(
        name = "model_client.stream_responses_api",
        level = "info",
        skip_all,
        fields(
            model = %model_info.slug,
            wire_api = ?self.client.state.provider.wire_api(),
            transport = "responses_http",
            http.method = "POST",
            api.path = tracing::field::Empty,
            turn.has_metadata_header = responses_metadata.has_turn_metadata()
        )
    )]
    async fn stream_responses_api(
        &self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<String>,
        responses_metadata: &GenexResponsesMetadata,
        inference_trace: &InferenceTraceContext,
    ) -> Result<ResponseStream> {
        let mut provider_auth_recovery_attempted = false;
        loop {
            let client_setup = self.client.current_client_setup().await?;
            let responses_headers = self
                .client
                .responses_headers(&client_setup.auth, &model_info.slug);
            tracing::Span::current().record("api.path", "/responses");
            let compression = self.responses_request_compression(&client_setup.auth);
            let mut options = self
                .build_responses_options(
                    responses_metadata,
                    compression,
                    model_info.use_responses_lite,
                )
                .await;

            let mut request = self.client.build_responses_request(
                prompt,
                model_info,
                effort.clone(),
                summary,
                service_tier.clone(),
                responses_metadata,
            )?;
            ModelClient::filter_tool_result_metadata(&mut request.input);
            if let Some(header_value) = self.client.build_routing_hint_header(
                &client_setup.auth,
                &request.model,
                request.service_tier.as_deref(),
            ) {
                options
                    .extra_headers
                    .insert(X_GENEX_ROUTING_HINT_HEADER, header_value);
            }
            request.access_programs =
                cyber_access_program::for_program(prompt.cyber_access_program);
            self.client
                .prepare_response_items_for_request(&mut request.input);
            let request_session_telemetry =
                session_telemetry_for_request(session_telemetry, &request);
            options.extra_headers.extend(responses_headers);
            let inference_trace_attempt = inference_trace.start_attempt();
            inference_trace_attempt.add_request_headers(&mut options.extra_headers);
            inference_trace_attempt.record_started(&request);
            let payload = serde_json::to_value(&request).map_err(|error| {
                GenexErr::Fatal(format!("failed to encode responses request: {error}"))
            })?;
            let provider_request = genex_model_provider::ChatRequest {
                model: request.model.clone(),
                instructions: Some(request.instructions.clone()),
                input: Vec::new(),
                tools: Vec::new(),
                metadata: std::collections::BTreeMap::new(),
                conversation_id: Some(self.client.responses_session_id(responses_metadata)),
                session_source: Some(self.client.state.session_source.clone()),
                payload: Some(payload),
                transport: genex_model_provider::ChatTransportOptions {
                    transport: ProviderTransportContext::from_http_client_factory(
                        self.client.http_client_factory.clone(),
                    ),
                    session_id: options.session_id.clone(),
                    thread_id: options.thread_id.clone(),
                    extra_headers: neutral_headers(options.extra_headers),
                    compression: match options.compression {
                        Compression::None => genex_model_provider::ProviderRequestCompression::None,
                        Compression::Zstd => genex_model_provider::ProviderRequestCompression::Zstd,
                    },
                    turn_state: options.turn_state,
                },
            };
            let provider_result = self.client.state.provider.chat(provider_request).await;
            match provider_result {
                Ok(provider_stream) => {
            let api_stream = provider_stream.map(|event| {
                event.map(provider_event_to_response_event)
            });
                    let (stream, _) = map_response_events(
                        None,
                        api_stream,
                        request_session_telemetry,
                        inference_trace_attempt,
                    );
                    return Ok(stream);
                }
                Err(genex_model_provider::ProviderError::Unauthorized) => {
                    let transport = TransportError::Http {
                        status: StatusCode::UNAUTHORIZED,
                        url: None,
                        headers: None,
                        body: None,
                    };
                    let _recovery = handle_unauthorized(
                        transport,
                        &mut provider_auth_recovery_attempted,
                        session_telemetry,
                        &self.client.state.provider,
                        self.client.event_sender.as_ref(),
                        responses_metadata.turn_id.as_deref(),
                    )
                    .await?;
                    continue;
                }
                Err(error) => return Err(GenexErr::Fatal(error.to_string())),
            }
        }
    }

    /// Streams a turn via the Responses API over WebSocket transport.
    #[allow(clippy::too_many_arguments)]
    #[instrument(
        name = "model_client.stream_responses_websocket",
        level = "info",
        skip_all,
        fields(
            model = %model_info.slug,
            wire_api = ?self.client.state.provider.wire_api(),
            transport = "responses_websocket",
            api.path = tracing::field::Empty,
            turn.has_metadata_header = responses_metadata.has_turn_metadata(),
            websocket.warmup = warmup
        )
    )]
    async fn stream_responses_websocket(
        &mut self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<String>,
        responses_metadata: &GenexResponsesMetadata,
        warmup: bool,
        request_trace: Option<W3cTraceContext>,
        inference_trace: &InferenceTraceContext,
    ) -> Result<WebsocketStreamOutcome> {
        let provider = Arc::clone(&self.client.state.provider);
        let mut provider_auth_recovery_attempted = false;
        let mut pending_retry = PendingUnauthorizedRetry::default();
        loop {
            let client_setup = self.client.current_client_setup().await?;
            let responses_headers = self
                .client
                .responses_headers(&client_setup.auth, &model_info.slug);
            tracing::Span::current().record("api.path", "/responses");
            let request_auth_context =
                AuthRequestTelemetryContext::new(&client_setup.auth, pending_retry);
            let mut request = self.client.build_responses_request(
                prompt,
                model_info,
                effort.clone(),
                summary,
                service_tier.clone(),
                responses_metadata,
            )?;
            ModelClient::filter_tool_result_metadata(&mut request.input);
            request.access_programs =
                cyber_access_program::for_program(prompt.cyber_access_program);
            let mut websocket_metadata = responses_metadata.clone();
            websocket_metadata.routing_hint = self.client.build_routing_hint_header(
                &client_setup.auth,
                &request.model,
                request.service_tier.as_deref(),
            );
            let request_session_telemetry = if warmup {
                // `generate=false` prewarm is connection setup, not an inference request.
                session_telemetry.clone()
            } else {
                session_telemetry_for_request(session_telemetry, &request)
            };
            match self
                .websocket_connection(WebsocketConnectParams {
                    session_telemetry,
                    connection_key: client_setup.connection_key.clone(),
                    auth_owner_generation: client_setup.auth_owner_generation,
                    responses_metadata: &websocket_metadata,
                    auth_context: request_auth_context,
                    request_route_telemetry: RequestRouteTelemetry::for_endpoint("/responses"),
                    responses_headers: &responses_headers,
                })
                .await
            {
                Ok(_) => {}
                Err(ProviderError::UpgradeRequired) => {
                    return Ok(WebsocketStreamOutcome::FallbackToHttp);
                }
                Err(ProviderError::Unauthorized) => {
                    let unauthorized_transport = TransportError::Http {
                        status: StatusCode::UNAUTHORIZED,
                        url: None,
                        headers: None,
                        body: None,
                    };
                    pending_retry = PendingUnauthorizedRetry::from_recovery(
                        handle_unauthorized(
                            unauthorized_transport,
                            &mut provider_auth_recovery_attempted,
                            session_telemetry,
                            &provider,
                            self.client.event_sender.as_ref(),
                            responses_metadata.turn_id.as_deref(),
                        )
                        .await?,
                    );
                    continue;
                }
                Err(err) => return Err(provider_error_to_genex_error(err)),
            }

            // Measure the complete logical request, not only the websocket delta.
            let mut client_metadata = self
                .client
                .build_ws_client_metadata(responses_metadata, model_info.use_responses_lite);
            if let Some(turn_state) = self.turn_state.get() {
                client_metadata.insert(X_GENEX_TURN_STATE_HEADER.to_string(), turn_state.clone());
            }
            let continuation = self.prepare_websocket_request(&request);
            let (mode, reason) = if continuation.is_some() {
                ("incremental", "incremental")
            } else {
                let reason = self
                    .websocket_session
                    .continuation_reset_reason
                    .take()
                    .unwrap_or(if self.websocket_session.last_request.is_some() {
                        "other"
                    } else if self.client.restored_history {
                        "restored_history"
                    } else {
                        "no_previous_request"
                    });
                ("full", reason)
            };
            let previous_response_id_from_untraced_warmup = continuation
                .as_ref()
                .is_some_and(|c| c.from_untraced_warmup);
            let inference_trace_attempt = if warmup {
                // Prewarm sends `generate=false`; it is connection setup, not a
                // model inference attempt that should appear in rollout traces.
                InferenceTraceAttempt::disabled()
            } else {
                inference_trace.start_attempt()
            };
            if previous_response_id_from_untraced_warmup {
                // The transport can reuse an untraced warmup response id and omit the
                // already-sent input, but rollout replay needs the logical model-visible
                // request rather than the compressed websocket delta.
                inference_trace_attempt.record_started(&request);
            }

            let (previous_response_id, mut incremental_items) = match continuation {
                Some(continuation) => (Some(continuation.response_id), Some(continuation.items)),
                None => (None, None),
            };
            let original_item_ids = if let Some(incremental_items) = &mut incremental_items {
                self.client
                    .prepare_response_items_for_request(incremental_items);
                None
            } else {
                let original_item_ids = request
                    .input
                    .iter()
                    .map(|item| item.id().cloned())
                    .collect::<Vec<_>>();
                self.client
                    .prepare_response_items_for_request(&mut request.input);
                Some(original_item_ids)
            };
            let ws_payload = ResponseCreateWsRequest {
                previous_response_id,
                input: incremental_items.as_deref().unwrap_or(&request.input),
                generate: if warmup { Some(false) } else { None },
                client_metadata: response_create_client_metadata(
                    Some(client_metadata),
                    request_trace.as_ref(),
                ),
                ..ResponseCreateWsRequest::from(&request)
            };
            let mut ws_request = ResponsesWsRequest::ResponseCreate(ws_payload);
            stamp_ws_stream_request_start_ms(&mut ws_request);
            if !previous_response_id_from_untraced_warmup {
                inference_trace_attempt.record_started(&ws_request);
            }

            let websocket_connection =
                self.websocket_session.connection.as_ref().ok_or_else(|| {
                    GenexErr::Fatal("websocket connection is unavailable".to_string())
                })?;
            request_session_telemetry.counter(
                WEBSOCKET_CONTINUATION_COUNT_METRIC,
                /*inc*/ 1,
                &[
                    ("mode", mode),
                    ("reason", reason),
                    ("phase", if warmup { "warmup" } else { "generation" }),
                ],
            );
            let websocket_payload = serde_json::to_value(&ws_request).map_err(|err| {
                GenexErr::Fatal(format!("failed to encode websocket request: {err}"))
            })?;
            let stream_result = websocket_connection
                .stream_request(ProviderWebsocketRequest {
                    payload: websocket_payload,
                    connection_reused: self.websocket_session.connection_reused(),
                    turn_state: Some(Arc::clone(&self.turn_state)),
                })
                .await;
            if let Some(original_item_ids) = original_item_ids {
                for (item, original_item_id) in request.input.iter_mut().zip(original_item_ids) {
                    item.set_id(original_item_id);
                }
            }
            self.websocket_session.last_request = Some(request);
            self.websocket_session.last_response_from_untraced_warmup = warmup;
            let stream_result = stream_result.map_err(|err| {
                let err = provider_error_to_genex_error(err);
                inference_trace_attempt.record_failed(&err, None, /*output_items*/ &[]);
                err
            })?;
            let api_stream = stream_result.map(|event| {
                event.map(provider_event_to_response_event)
            });
            let (stream, last_request_rx) = map_response_events(
                None,
                api_stream,
                request_session_telemetry,
                inference_trace_attempt,
            );
            self.websocket_session.last_response_rx = Some(last_request_rx);
            return Ok(WebsocketStreamOutcome::Stream(stream));
        }
    }

    /// Builds telemetry for the Responses API WebSocket transport.
    fn build_websocket_telemetry(
        session_telemetry: &SessionTelemetry,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
    ) -> Arc<dyn ProviderWebsocketTelemetry> {
        let telemetry = Arc::new(ApiTelemetry::new(
            session_telemetry.clone(),
            auth_context,
            request_route_telemetry,
        ));
        let websocket_telemetry: Arc<dyn ProviderWebsocketTelemetry> = telemetry;
        websocket_telemetry
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn prewarm_websocket(
        &mut self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<String>,
        responses_metadata: &GenexResponsesMetadata,
    ) -> Result<()> {
        if !self.client.responses_websocket_enabled() {
            return Ok(());
        }
        if self.websocket_session.last_request.is_some() {
            return Ok(());
        }

        let disabled_trace = InferenceTraceContext::disabled();
        match self
            .stream_responses_websocket(
                prompt,
                model_info,
                session_telemetry,
                effort,
                summary,
                service_tier,
                responses_metadata,
                /*warmup*/ true,
                current_span_w3c_trace_context(),
                &disabled_trace,
            )
            .await
        {
            Ok(WebsocketStreamOutcome::Stream(mut stream)) => {
                // Wait for the v2 warmup request to complete before sending the first turn request.
                while let Some(event) = stream.next().await {
                    match event {
                        Ok(ResponseEvent::Completed { .. }) => break,
                        Err(err) => return Err(err),
                        _ => {}
                    }
                }
                Ok(())
            }
            Ok(WebsocketStreamOutcome::FallbackToHttp) => {
                self.try_switch_fallback_transport(session_telemetry, model_info);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Streams a single model request within the current turn.
    ///
    /// The caller is responsible for passing per-turn settings explicitly (model selection,
    /// reasoning settings, telemetry context, and turn metadata). This method will prefer the
    /// Responses WebSocket transport when the provider supports it and it remains healthy, and will
    /// fall back to the HTTP Responses API transport otherwise. The trace context may be enabled or
    /// disabled, but is always explicit so transport paths do not need separate trace/no-trace
    /// branches.
    pub async fn stream(
        &mut self,
        prompt: &Prompt,
        model_info: &ModelInfo,
        session_telemetry: &SessionTelemetry,
        effort: Option<ReasoningEffortConfig>,
        summary: ReasoningSummaryConfig,
        service_tier: Option<String>,
        responses_metadata: &GenexResponsesMetadata,
        inference_trace: &InferenceTraceContext,
    ) -> Result<ResponseStream> {
        match self.client.state.provider.wire_api() {
            ProviderWireApi::Responses => {
                if self.client.responses_websocket_enabled() {
                    let request_trace = current_span_w3c_trace_context();
                    match self
                        .stream_responses_websocket(
                            prompt,
                            model_info,
                            session_telemetry,
                            effort.clone(),
                            summary,
                            service_tier.clone(),
                            responses_metadata,
                            /*warmup*/ false,
                            request_trace,
                            inference_trace,
                        )
                        .await?
                    {
                        WebsocketStreamOutcome::Stream(stream) => return Ok(stream),
                        WebsocketStreamOutcome::FallbackToHttp => {
                            self.try_switch_fallback_transport(session_telemetry, model_info);
                        }
                    }
                }

                self.stream_responses_api(
                    prompt,
                    model_info,
                    session_telemetry,
                    effort,
                    summary,
                    service_tier,
                    responses_metadata,
                    inference_trace,
                )
                .await
            }
            ProviderWireApi::Chat => {
                self.stream_chat_completions(
                    prompt,
                    model_info,
                    session_telemetry,
                    responses_metadata,
                    inference_trace,
                )
                .await
            }
        }
    }

    /// Permanently disables WebSockets for this Genex session and resets WebSocket state.
    ///
    /// This is used after exhausting the provider retry budget, to force subsequent requests onto
    /// the HTTP transport.
    ///
    /// Returns `true` if this call activated fallback, or `false` if fallback was already active.
    pub(crate) fn try_switch_fallback_transport(
        &mut self,
        session_telemetry: &SessionTelemetry,
        model_info: &ModelInfo,
    ) -> bool {
        let activated = self
            .client
            .force_http_fallback(session_telemetry, model_info);
        self.websocket_session = WebsocketSession::default();
        activated
    }
}

/// Stamp a ResponsesWsRequest with the current time.
///
/// Meant to be called just before sending the request over the socket, to capture realistic
/// transport timing.
fn stamp_ws_stream_request_start_ms(request: &mut ResponsesWsRequest<'_>) {
    let ResponsesWsRequest::ResponseCreate(payload) = request;
    payload
        .client_metadata
        .get_or_insert_with(HashMap::new)
        .insert(
            X_GENEX_WS_STREAM_REQUEST_START_MS_CLIENT_METADATA_KEY.to_string(),
            crate::turn_timing::now_unix_timestamp_ms().to_string(),
        );
}

fn neutral_headers(headers: ApiHeaderMap) -> std::collections::BTreeMap<String, String> {
    headers
        .into_iter()
        .filter_map(|(name, value)| {
            let name = name?;
            let value = value.to_str().ok()?.to_string();
            Some((name.to_string(), value))
        })
        .collect()
}

/// Builds the extra headers attached to Responses API requests.
///
/// These headers implement Genex-specific conventions:
///
/// - `x-genex-beta-features`: comma-separated beta feature keys enabled for the session.
/// - `x-genex-turn-state`: sticky routing token captured earlier in the turn.
fn build_responses_headers(
    beta_features_header: Option<&str>,
    turn_state: Option<&Arc<OnceLock<String>>>,
) -> ApiHeaderMap {
    let mut headers = ApiHeaderMap::new();
    if let Some(value) = beta_features_header
        && !value.is_empty()
        && let Ok(header_value) = HeaderValue::from_str(value)
    {
        headers.insert("x-genex-beta-features", header_value);
    }
    if let Some(turn_state) = turn_state
        && let Some(state) = turn_state.get()
        && let Ok(header_value) = HeaderValue::from_str(state)
    {
        headers.insert(X_GENEX_TURN_STATE_HEADER, header_value);
    }
    headers
}

fn add_responses_lite_header(headers: &mut ApiHeaderMap, use_responses_lite: bool) {
    if use_responses_lite {
        headers.insert(
            X_PROVIDER_INTERNAL_RESPONSES_LITE_HEADER,
            HeaderValue::from_static("true"),
        );
    }
}

const RESPONSE_STREAM_CHANNEL_CAPACITY: usize = 1600;
const STREAM_DROPPED_REASON: &str = "response stream dropped before provider terminal event";

fn provider_event_to_response_event(event: ProviderChatEvent) -> ResponseEvent {
    match event {
        ProviderChatEvent::Created { response_id } => ResponseEvent::Created { response_id },
        ProviderChatEvent::SafetyBuffering(buffering) => {
            ResponseEvent::SafetyBuffering(genex_api::SafetyBuffering {
                use_cases: buffering.use_cases,
                reasons: buffering.reasons,
                show_buffering_ui: buffering.show_buffering_ui,
                faster_model: buffering.faster_model,
            })
        }
        ProviderChatEvent::OutputItemDone(item) => ResponseEvent::OutputItemDone(item),
        ProviderChatEvent::OutputItemAdded(item) => ResponseEvent::OutputItemAdded(item),
        ProviderChatEvent::ServerModel(model) => ResponseEvent::ServerModel(model),
        ProviderChatEvent::ModelVerifications(verifications) => {
            ResponseEvent::ModelVerifications(verifications)
        }
        ProviderChatEvent::ServerReasoningIncluded(included) => {
            ResponseEvent::ServerReasoningIncluded(included)
        }
        ProviderChatEvent::Completed {
            response_id,
            token_usage,
            usage_metadata,
            end_turn,
        } => ResponseEvent::Completed {
            response_id,
            token_usage,
            usage_metadata,
            end_turn,
        },
        ProviderChatEvent::OutputTextDelta(delta) => ResponseEvent::OutputTextDelta(delta),
        ProviderChatEvent::ToolCallInputDelta {
            item_id,
            call_id,
            delta,
        } => ResponseEvent::ToolCallInputDelta {
            item_id,
            call_id,
            delta,
        },
        ProviderChatEvent::ReasoningSummaryDelta {
            delta,
            summary_index,
        } => ResponseEvent::ReasoningSummaryDelta {
            delta,
            summary_index,
        },
        ProviderChatEvent::ReasoningSummaryDone {
            item_id,
            text,
            summary_index,
        } => ResponseEvent::ReasoningSummaryDone {
            item_id,
            text,
            summary_index,
        },
        ProviderChatEvent::ReasoningContentDelta {
            delta,
            content_index,
        } => ResponseEvent::ReasoningContentDelta {
            delta,
            content_index,
        },
        ProviderChatEvent::ReasoningSummaryPartAdded { summary_index } => {
            ResponseEvent::ReasoningSummaryPartAdded { summary_index }
        }
        ProviderChatEvent::RateLimits(snapshot) => ResponseEvent::RateLimits(snapshot),
        ProviderChatEvent::ModelsEtag(etag) => ResponseEvent::ModelsEtag(etag),
    }
}

fn map_response_events<S>(
    upstream_request_id: Option<String>,
    api_stream: S,
    session_telemetry: SessionTelemetry,
    inference_trace_attempt: InferenceTraceAttempt,
) -> (ResponseStream, oneshot::Receiver<LastResponse>)
where
    S: futures::Stream<Item = std::result::Result<ResponseEvent, ProviderError>>
        + Unpin
        + Send
        + 'static,
{
    let (tx_event, rx_event) =
        mpsc::channel::<Result<ResponseEvent>>(RESPONSE_STREAM_CHANNEL_CAPACITY);
    let (tx_last_response, rx_last_response) = oneshot::channel::<LastResponse>();
    let consumer_dropped = CancellationToken::new();
    let consumer_dropped_for_stream = consumer_dropped.clone();

    tokio::spawn(async move {
        let mut logged_error = false;
        let mut tx_last_response = Some(tx_last_response);
        let mut items_added: Vec<ResponseItem> = Vec::new();
        let (request_start, mut ttft_ms) = (Instant::now(), None);
        let mut api_stream = api_stream;
        let upstream_request_id = upstream_request_id.as_deref();
        loop {
            let event = tokio::select! {
                _ = consumer_dropped.cancelled() => {
                    inference_trace_attempt.record_cancelled(
                        STREAM_DROPPED_REASON,
                        upstream_request_id,
                        &items_added,
                    );
                    return;
                }
                event = api_stream.next() => event,
            };
            let Some(event) = event else {
                break;
            };
            match event {
                Ok(ResponseEvent::OutputItemDone(item)) => {
                    items_added.push(item.clone());
                    if tx_event
                        .send(Ok(ResponseEvent::OutputItemDone(item)))
                        .await
                        .is_err()
                    {
                        inference_trace_attempt.record_cancelled(
                            STREAM_DROPPED_REASON,
                            upstream_request_id,
                            &items_added,
                        );
                        return;
                    }
                }
                Ok(ResponseEvent::Completed {
                    response_id,
                    token_usage,
                    usage_metadata,
                    end_turn,
                }) => {
                    if let Some(usage) = &token_usage {
                        session_telemetry.sse_event_completed(usage, ttft_ms);
                    }
                    inference_trace_attempt.record_completed(
                        &response_id,
                        upstream_request_id,
                        &token_usage,
                        &items_added,
                    );
                    if let Some(sender) = tx_last_response.take() {
                        let _ = sender.send(LastResponse {
                            response_id: response_id.clone(),
                            items_added: std::mem::take(&mut items_added),
                        });
                    }
                    if tx_event
                        .send(Ok(ResponseEvent::Completed {
                            response_id,
                            token_usage,
                            usage_metadata,
                            end_turn,
                        }))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                Ok(event) => {
                    if matches!(&event, ResponseEvent::OutputItemAdded(_)) && ttft_ms.is_none() {
                        ttft_ms = Some(
                            i64::try_from(request_start.elapsed().as_millis()).unwrap_or(i64::MAX),
                        );
                    }
                    if tx_event.send(Ok(event)).await.is_err() {
                        inference_trace_attempt.record_cancelled(
                            STREAM_DROPPED_REASON,
                            upstream_request_id,
                            &items_added,
                        );
                        return;
                    }
                }
                Err(err) => {
                    let mapped = provider_error_to_genex_error(err);
                    inference_trace_attempt.record_failed(
                        &mapped,
                        upstream_request_id,
                        &items_added,
                    );
                    if !logged_error {
                        session_telemetry.see_event_completed_failed(&mapped);
                        logged_error = true;
                    }
                    if tx_event.send(Err(mapped)).await.is_err() {
                        return;
                    }
                }
            }
        }
        inference_trace_attempt.record_failed(
            "stream closed before response.completed",
            upstream_request_id,
            &items_added,
        );
    });

    (
        ResponseStream {
            rx_event,
            consumer_dropped: consumer_dropped_for_stream,
        },
        rx_last_response,
    )
}

/// Handles a 401 response by optionally refreshing external credentials once.
///
/// When refresh succeeds, the caller should retry the API call; otherwise
/// the mapped `GenexErr` is returned to the caller.
#[derive(Clone, Copy, Debug)]
struct UnauthorizedRecoveryExecution {
    mode: &'static str,
    phase: &'static str,
}

#[derive(Clone, Copy, Debug, Default)]
struct PendingUnauthorizedRetry {
    retry_after_unauthorized: bool,
    recovery_mode: Option<&'static str>,
    recovery_phase: Option<&'static str>,
}

impl PendingUnauthorizedRetry {
    fn from_recovery(recovery: UnauthorizedRecoveryExecution) -> Self {
        Self {
            retry_after_unauthorized: true,
            recovery_mode: Some(recovery.mode),
            recovery_phase: Some(recovery.phase),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct AuthRequestTelemetryContext {
    auth_header_attached: bool,
    auth_header_name: Option<&'static str>,
    retry_after_unauthorized: bool,
    recovery_mode: Option<&'static str>,
    recovery_phase: Option<&'static str>,
}

impl AuthRequestTelemetryContext {
    fn new(auth: &ProviderRequestAuth, retry: PendingUnauthorizedRetry) -> Self {
        Self {
            auth_header_attached: auth.auth_header_attached,
            auth_header_name: auth.auth_header_name,
            retry_after_unauthorized: retry.retry_after_unauthorized,
            recovery_mode: retry.recovery_mode,
            recovery_phase: retry.recovery_phase,
        }
    }
}

struct WebsocketConnectParams<'a> {
    session_telemetry: &'a SessionTelemetry,
    connection_key: ProviderConnectionKey,
    auth_owner_generation: Option<u64>,
    responses_metadata: &'a GenexResponsesMetadata,
    auth_context: AuthRequestTelemetryContext,
    request_route_telemetry: RequestRouteTelemetry,
    responses_headers: &'a ApiHeaderMap,
}

fn emit_auth_recovery_event(
    event_sender: Option<&Sender<ProtocolEvent>>,
    turn_id: Option<&str>,
    provider_name: &str,
    message: &str,
    event: fn(AuthRecoveryEvent) -> EventMsg,
) {
    if let (Some(sender), Some(turn_id)) = (event_sender, turn_id) {
        let _ = sender.try_send(ProtocolEvent {
            id: turn_id.to_string(),
            msg: event(AuthRecoveryEvent {
                provider: provider_name.to_string(),
                message: message.to_string(),
            }),
        });
    }
}

async fn handle_unauthorized(
    transport: TransportError,
    provider_auth_recovery_attempted: &mut bool,
    session_telemetry: &SessionTelemetry,
    provider: &SharedModelProvider,
    event_sender: Option<&Sender<ProtocolEvent>>,
    turn_id: Option<&str>,
) -> Result<UnauthorizedRecoveryExecution> {
    if !*provider_auth_recovery_attempted {
        *provider_auth_recovery_attempted = true;
        let messages = provider.auth_recovery_messages();
        if let Some(messages) = messages {
            emit_auth_recovery_event(
                event_sender,
                turn_id,
                &provider.descriptor().provider_kind,
                messages.started,
                EventMsg::AuthRecoveryStarted,
            );
        }
        match provider.recover_from_unauthorized().await {
            Ok(ProviderUnauthorizedRecovery::Recovered) => {
                if let Some(messages) = messages {
                    emit_auth_recovery_event(
                        event_sender,
                        turn_id,
                        &provider.descriptor().provider_kind,
                        messages.succeeded,
                        EventMsg::AuthRecoveryCompleted,
                    );
                }
                return Ok(UnauthorizedRecoveryExecution {
                    mode: "provider",
                    phase: "provider_refresh",
                });
            }
            Ok(ProviderUnauthorizedRecovery::NotConfigured) => {}
            Err(error) => {
                let error = provider_recovery_error_to_genex_error(error);
                warn!(
                    error = %error,
                    "provider authentication recovery failed"
                );
                return Err(error);
            }
        }
    }
    session_telemetry.record_auth_recovery(
        "none",
        "none",
        "recovery_not_run",
        None,
        None,
        Some("provider_recovery_not_configured"),
        /*auth_state_changed*/ None,
    );

    Err(GenexErr::Fatal(transport.to_string()))
}

fn provider_recovery_error_to_genex_error(error: genex_model_provider::ProviderError) -> GenexErr {
    match error {
        genex_model_provider::ProviderError::Transient(message)
        | genex_model_provider::ProviderError::Transport(message) => {
            GenexErr::Io(std::io::Error::other(message))
        }
        genex_model_provider::ProviderError::RateLimited { .. } => {
            GenexErr::Io(std::io::Error::other("provider rate limit reached"))
        }
        other => GenexErr::Fatal(other.to_string()),
    }
}

fn provider_error_to_genex_error(error: ProviderError) -> GenexErr {
    provider_recovery_error_to_genex_error(error)
}

fn provider_error_to_api_error(error: ProviderError) -> ApiError {
    match error {
        ProviderError::UpgradeRequired => ApiError::Transport(TransportError::Http {
            status: StatusCode::UPGRADE_REQUIRED,
            url: None,
            headers: None,
            body: None,
        }),
        other => ApiError::Stream(other.to_string()),
    }
}

struct ApiTelemetry {
    session_telemetry: SessionTelemetry,
    auth_context: AuthRequestTelemetryContext,
    request_route_telemetry: RequestRouteTelemetry,
}

impl ApiTelemetry {
    fn new(
        session_telemetry: SessionTelemetry,
        auth_context: AuthRequestTelemetryContext,
        request_route_telemetry: RequestRouteTelemetry,
    ) -> Self {
        Self {
            session_telemetry,
            auth_context,
            request_route_telemetry,
        }
    }
}

impl RequestTelemetry for ApiTelemetry {
    fn on_request(
        &self,
        attempt: u64,
        status: Option<StatusCode>,
        error: Option<&TransportError>,
        duration: Duration,
    ) {
        let error_message = error.map(telemetry_transport_error_message);
        let status = status.map(|s| s.as_u16());
        let debug = error
            .map(extract_response_debug_context)
            .unwrap_or_default();
        self.session_telemetry.record_api_request(
            attempt,
            status,
            error_message.as_deref(),
            duration,
            self.auth_context.auth_header_attached,
            self.auth_context.auth_header_name,
            self.auth_context.retry_after_unauthorized,
            self.auth_context.recovery_mode,
            self.auth_context.recovery_phase,
            self.request_route_telemetry.endpoint,
            debug.request_id.as_deref(),
            debug.cf_ray.as_deref(),
        );
    }
}

impl SseTelemetry for ApiTelemetry {
    fn on_sse_poll(
        &self,
        result: &std::result::Result<
            Option<std::result::Result<Event, EventStreamError<TransportError>>>,
            tokio::time::error::Elapsed,
        >,
        duration: Duration,
    ) {
        self.session_telemetry.log_sse_event(result, duration);
    }
}

impl ProviderWebsocketTelemetry for ApiTelemetry {
    fn on_request(
        &self,
        duration: Duration,
        error: Option<&ProviderError>,
        connection_reused: bool,
    ) {
        let error_message = error.map(ToString::to_string);
        self.session_telemetry.record_websocket_request(
            duration,
            error_message.as_deref(),
            connection_reused,
        );
    }

    fn on_event(&self, event: ProviderWebsocketEvent, duration: Duration) {
        let result = match event {
            ProviderWebsocketEvent::Message(payload) => {
                Ok(Some(Ok(Message::Text(payload.to_string().into()))))
            }
            ProviderWebsocketEvent::Error(error) => Err(ApiError::Stream(error.to_string())),
            ProviderWebsocketEvent::Closed => Ok(None),
            ProviderWebsocketEvent::Ignored => return,
        };
        self.session_telemetry
            .record_websocket_event(&result, duration);
    }
}
