use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use genex_api::ApiError;
use genex_api::ChatClient;
use genex_api::ChatCompletionApi;
use genex_api::ChatRequestInput;
use genex_api::Provider;
use genex_api::ResponsesClient;
use genex_api::ResponsesWebsocketClient;
use genex_api::ResponsesWebsocketConnection;
use genex_api::SearchClient;
use genex_api::SearchCommands;
use genex_api::SearchInput;
use genex_api::SearchRequest;
use genex_api::SearchSettings;
use genex_api::SharedAuthProvider;
use genex_api::StandardChatCompletionApi;
use genex_api::TransportError;
use genex_api::WebsocketTelemetry;
use genex_api::auth_header_telemetry;
use genex_api::is_azure_responses_provider;
use genex_http_client::ClientRouteClass;
use genex_http_client::HttpClientFactory;
use genex_http_client::HttpTransport;
use genex_http_client::RequestBody;
use genex_login::CredentialManager;
use genex_login::GenexAuth;
use genex_login::ProviderAuth;
use genex_login::default_client::ClientRedirectPolicy;
use genex_login::default_client::create_client_for_route;
use genex_model_provider_info::ModelProviderInfo;
use genex_model_provider_info::ProviderAuthField;
use genex_model_provider_info::WireApi;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_protocol::account::ProviderAccount;
use genex_protocol::account::ProviderAuthStatus;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result as CoreResult;
use genex_protocol::protocol::RateLimitSnapshot;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

use crate::auth::ProviderAuthScope;
use crate::auth::ResolvedProviderAuth;
use crate::auth::cached_provider_auth;
use crate::auth::credential_manager_for_provider;
use crate::auth::provider_auth;
use crate::auth::resolve_provider_auth;
use crate::auth::resolve_provider_auth_for_scope;
use crate::cloudflare_provider::CloudflareModelProvider;
use crate::contract::CURRENT_PROVIDER_PROTOCOL_VERSION;
use crate::contract::ProviderAuthRecoveryMessages;
use crate::contract::ProviderCapabilities;
use crate::contract::ProviderConnectionKey;
use crate::contract::ProviderDescriptor;
use crate::contract::ProviderRequestAuth;
use crate::contract::ProviderRequestSnapshot;
use crate::contract::ProviderUnauthorizedRecovery;
use crate::contract::ProviderUsage;
use crate::contract::ProviderUsageResult;
use crate::contract::ProviderWebsocketConnectRequest;
use crate::contract::ProviderWebsocketEvent;
use crate::contract::ProviderWebsocketRequest;
use crate::contract::ProviderWireApi;
use crate::contract::RemoteCompactionSupport;
use crate::contract::SharedProviderWebsocketConnection;
use crate::contract::headers_to_http;
use crate::models_endpoint::ModelsEndpoint;

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

/// Provider-neutral credentials supplied by a host login flow.
#[derive(Clone, PartialEq, Eq)]
pub struct ProviderLoginRequest {
    pub api_key: String,
    pub credentials: BTreeMap<String, String>,
}

impl ProviderLoginRequest {
    pub fn new(api_key: impl Into<String>, credentials: BTreeMap<String, String>) -> Self {
        Self {
            api_key: api_key.into(),
            credentials,
        }
    }
}

/// Error classification for provider-owned credential lifecycle operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderAuthError {
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

pub type ProviderAuthResult = std::result::Result<(), ProviderAuthError>;
pub type ProviderDeleteResult = std::result::Result<bool, ProviderAuthError>;
pub type ProviderLogoutResult = std::result::Result<bool, ProviderAuthError>;

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

fn map_api_error_to_provider_error(error: ApiError) -> crate::contract::ProviderError {
    match error {
        ApiError::Transport(TransportError::Http { status, .. })
            if status == http::StatusCode::UNAUTHORIZED =>
        {
            crate::contract::ProviderError::Unauthorized
        }
        ApiError::Transport(TransportError::Http { status, .. })
            if status == http::StatusCode::UPGRADE_REQUIRED =>
        {
            crate::contract::ProviderError::UpgradeRequired
        }
        ApiError::InvalidRequest { message } => {
            crate::contract::ProviderError::InvalidRequest(message)
        }
        ApiError::RateLimitExceeded { delay, .. } => crate::contract::ProviderError::RateLimited {
            retry_after_seconds: delay.map(|delay| delay.as_secs()),
        },
        ApiError::Retryable { message, .. } => {
            crate::contract::ProviderError::Transient(message_or_default(message))
        }
        ApiError::ServerOverloaded => {
            crate::contract::ProviderError::Transient("server overloaded".to_string())
        }
        ApiError::Transport(error) => map_transport_error_to_provider_error(error),
        other => crate::contract::ProviderError::Provider(other.to_string()),
    }
}

fn map_transport_error_to_provider_error(error: TransportError) -> crate::contract::ProviderError {
    match error {
        TransportError::Http { status, .. } if status == http::StatusCode::UNAUTHORIZED => {
            crate::contract::ProviderError::Unauthorized
        }
        TransportError::Timeout => {
            crate::contract::ProviderError::Transient("provider request timed out".to_string())
        }
        other => crate::contract::ProviderError::Transport(other.to_string()),
    }
}

fn message_or_default(message: String) -> String {
    if message.is_empty() {
        "provider request failed".to_string()
    } else {
        message
    }
}

fn classify_api_error(error: &ApiError) -> crate::contract::ProviderError {
    match error {
        ApiError::Transport(TransportError::Http { status, .. })
            if *status == http::StatusCode::UNAUTHORIZED =>
        {
            crate::contract::ProviderError::Unauthorized
        }
        ApiError::Transport(TransportError::Http { status, .. })
            if *status == http::StatusCode::UPGRADE_REQUIRED =>
        {
            crate::contract::ProviderError::UpgradeRequired
        }
        ApiError::InvalidRequest { message } => {
            crate::contract::ProviderError::InvalidRequest(message.clone())
        }
        ApiError::RateLimitExceeded { delay, .. } => crate::contract::ProviderError::RateLimited {
            retry_after_seconds: delay.map(|delay| delay.as_secs()),
        },
        ApiError::Retryable { message, .. } => {
            crate::contract::ProviderError::Transient(message_or_default(message.clone()))
        }
        ApiError::ServerOverloaded => {
            crate::contract::ProviderError::Transient("server overloaded".to_string())
        }
        ApiError::Transport(TransportError::Timeout) => {
            crate::contract::ProviderError::Transient("provider request timed out".to_string())
        }
        ApiError::Transport(error) => crate::contract::ProviderError::Transport(error.to_string()),
        other => crate::contract::ProviderError::Provider(other.to_string()),
    }
}

struct ProviderWebsocketTelemetryAdapter {
    telemetry: Arc<dyn crate::contract::ProviderWebsocketTelemetry>,
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
                ProviderWebsocketEvent::Error(crate::contract::ProviderError::Transport(
                    "unexpected websocket message".to_string(),
                ))
            }
            Ok(Some(Err(error))) => ProviderWebsocketEvent::Error(
                crate::contract::ProviderError::Transport(error.to_string()),
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

impl crate::contract::ProviderWebsocketConnection for RuntimeWebsocketConnection {
    fn is_closed(
        &self,
    ) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<bool>> {
        Box::pin(async move { Ok(self.connection.is_closed().await) })
    }

    fn stream_request(
        &self,
        request: ProviderWebsocketRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderChatStream>,
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
            Ok(
                Box::pin(stream.map(|event| {
                    event
                        .map(crate::contract::ProviderChatEvent::from)
                        .map_err(map_api_error_to_provider_error)
                }))
                    as crate::contract::ProviderChatStream,
            )
        })
    }
}

/// Runtime provider abstraction used by model execution.
///
/// Implementations own provider-specific behavior for a model backend. The
/// `ModelProviderInfo` returned by `info` is the serialized/configured provider
/// metadata used by the default provider-compatible implementation.
pub trait ProviderRuntime: fmt::Debug + Send + Sync + 'static {
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
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<SharedProviderWebsocketConnection>,
    > {
        Box::pin(async move {
            let api_provider = self
                .api_provider()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let api_auth = self
                .api_auth()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let telemetry = request.telemetry.map(|telemetry| {
                Arc::new(ProviderWebsocketTelemetryAdapter { telemetry })
                    as Arc<dyn WebsocketTelemetry>
            });
            let connection = ResponsesWebsocketClient::new(api_provider, api_auth)
                .connect(
                    request.transport.http_client_factory(),
                    headers_to_http(request.extra_headers)?,
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

    /// Streams a provider-owned request through the provider-neutral contract.
    ///
    /// The contract currently carries the normalized Chat Completions input and the serialized
    /// Responses payload. Request construction and HTTP transport are owned here; the remaining
    /// core-owned boundaries are Responses authentication recovery/continuation and WebSocket.
    fn chat(
        &self,
        request: crate::contract::ChatRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderChatStream>,
    > {
        Box::pin(async move {
            let api_provider = self
                .api_provider()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let api_auth = self
                .api_auth()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            match self.api() {
                ProviderApi::Chat(chat_api) => {
                    let client = create_client_for_route(
                        request.transport.transport.http_client_factory(),
                        &api_provider.url_for_path("/chat/completions"),
                        ClientRouteClass::Api,
                        ClientRedirectPolicy::Default,
                    )
                    .map_err(|error| {
                        crate::contract::ProviderError::Transport(error.to_string())
                    })?;
                    let transport = genex_api::ReqwestTransport::from_http_client(client);
                    let chat_request = chat_api
                        .build_request(ChatRequestInput {
                            provider: &api_provider,
                            model: &request.model,
                            instructions: request.instructions.as_deref().unwrap_or_default(),
                            input: &request.input,
                            tools: &request.tools,
                            conversation_id: request.conversation_id,
                            session_source: request.session_source,
                        })
                        .map_err(|error| {
                            crate::contract::ProviderError::InvalidRequest(error.to_string())
                        })?;
                    let stream = ChatClient::with_api(transport, api_provider, api_auth, chat_api)
                        .stream_request(chat_request)
                        .await
                        .map_err(map_api_error_to_provider_error)?;
                    Ok(Box::pin(stream.map(|event| {
                        event
                            .map(crate::contract::ProviderChatEvent::from)
                            .map_err(map_api_error_to_provider_error)
                    }))
                        as crate::contract::ProviderChatStream)
                }
                ProviderApi::Responses => {
                    let payload = request.payload.ok_or_else(|| {
                        crate::contract::ProviderError::InvalidRequest(
                            "Responses provider chat request is missing payload".to_string(),
                        )
                    })?;
                    let client = create_client_for_route(
                        request.transport.transport.http_client_factory(),
                        &api_provider.url_for_path("/responses"),
                        ClientRouteClass::Api,
                        ClientRedirectPolicy::Default,
                    )
                    .map_err(|error| {
                        crate::contract::ProviderError::Transport(error.to_string())
                    })?;
                    let transport = genex_api::ReqwestTransport::from_http_client(client);
                    let compression = match request.transport.compression {
                        crate::contract::ProviderRequestCompression::None => {
                            genex_api::Compression::None
                        }
                        crate::contract::ProviderRequestCompression::Zstd => {
                            genex_api::Compression::Zstd
                        }
                    };
                    let mut headers = headers_to_http(request.transport.extra_headers)?;
                    headers.extend(genex_api::build_session_headers(
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
                    let stream = ResponsesClient::new(transport, api_provider, api_auth)
                        .stream(payload, headers, compression, request.transport.turn_state)
                        .await
                        .map_err(map_api_error_to_provider_error)?;
                    Ok(Box::pin(stream.map(|event| {
                        event
                            .map(crate::contract::ProviderChatEvent::from)
                            .map_err(map_api_error_to_provider_error)
                    }))
                        as crate::contract::ProviderChatStream)
                }
            }
        })
    }

    fn search(
        &self,
        request: crate::contract::ProviderSearchRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderSearchResponse>,
    > {
        Box::pin(async move {
            let api_provider = self
                .api_provider()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let api_auth = self
                .api_auth()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let client = create_client_for_route(
                request.transport.http_client_factory(),
                &api_provider.url_for_path("alpha/search"),
                ClientRouteClass::Api,
                ClientRedirectPolicy::Default,
            )
            .map_err(|error| crate::contract::ProviderError::Transport(error.to_string()))?;
            let commands: SearchCommands =
                serde_json::from_value(request.commands).map_err(|error| {
                    crate::contract::ProviderError::InvalidRequest(error.to_string())
                })?;
            let settings: SearchSettings =
                serde_json::from_value(request.settings).map_err(|error| {
                    crate::contract::ProviderError::InvalidRequest(error.to_string())
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
                genex_api::ReqwestTransport::from_http_client(client),
                api_provider,
                api_auth,
            )
            .search(&search_request, headers_to_http(request.extra_headers)?)
            .await
            .map_err(map_api_error_to_provider_error)?;
            Ok(crate::contract::ProviderSearchResponse {
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
        request: crate::contract::ProviderBackendRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderBackendResponse>,
    > {
        Box::pin(async move {
            let api_provider = self
                .api_provider()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let api_auth = self
                .api_auth()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let client = create_client_for_route(
                request.transport.http_client_factory(),
                &api_provider.url_for_path(&request.path),
                ClientRouteClass::Api,
                ClientRedirectPolicy::Default,
            )
            .map_err(|error| crate::contract::ProviderError::Transport(error.to_string()))?;
            let mut backend_request = api_provider.build_request(http::Method::POST, &request.path);
            backend_request
                .headers
                .extend(headers_to_http(request.headers)?);
            backend_request.body = Some(RequestBody::Json(request.body));
            backend_request.timeout = Some(request.timeout);
            let backend_request = api_auth
                .apply_auth(backend_request)
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))?;
            let response = genex_api::ReqwestTransport::from_http_client(client)
                .execute(backend_request)
                .await
                .map_err(map_transport_error_to_provider_error)?;
            let body = serde_json::from_slice(&response.body).map_err(|error| {
                crate::contract::ProviderError::Provider(format!(
                    "provider backend returned invalid JSON: {error}"
                ))
            })?;
            Ok(crate::contract::ProviderBackendResponse { body })
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

    /// Saves credentials for this mounted provider.
    fn login(&self, request: ProviderLoginRequest) -> ModelProviderFuture<'_, ProviderAuthResult> {
        Box::pin(async move {
            let Some(credential_manager) = self.credential_manager() else {
                return Err(ProviderAuthError::Unavailable);
            };
            let provider_id = self
                .provider_id()
                .unwrap_or(self.info().name.as_str())
                .to_owned();
            credential_manager
                .login(
                    &provider_id,
                    ProviderAuth::api_key_with_credentials(request.api_key, request.credentials),
                )
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
            let provider_id = self
                .provider_id()
                .unwrap_or(self.info().name.as_str())
                .to_owned();
            credential_manager
                .delete(&provider_id)
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

    fn request_snapshot(
        &self,
    ) -> ModelProviderFuture<'_, CoreResult<ProviderRequestSnapshot>> {
        Box::pin(async move {
            let auth = self.auth().await;
            let api_auth = self.api_auth().await?;
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
                    managed_auth: auth
                        .as_ref()
                        .is_some_and(GenexAuth::uses_managed_auth),
                    auth_header_attached: auth_telemetry.attached,
                    auth_header_name: auth_telemetry.name,
                    revision,
                    owner_generation,
                },
                connection_key: ProviderConnectionKey {
                    provider_kind: self.descriptor().provider_kind,
                    auth_revision: revision,
                },
            })
        })
    }
}

/// Transitional adapter from the existing runtime implementation to the public provider-neutral
/// contract. New provider implementations should implement [`crate::ModelProvider`] directly;
/// this blanket adapter exists so the current transport-backed provider can be migrated without
/// changing every execution consumer in one step.
impl<T> crate::contract::ModelProvider for T
where
    T: ProviderRuntime,
{
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderRuntime::descriptor(self)
    }

    fn supports_attestation(&self) -> bool {
        ProviderRuntime::supports_attestation(self)
    }

    fn request_snapshot(
        &self,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<ProviderRequestSnapshot>,
    > {
        Box::pin(async move {
            ProviderRuntime::request_snapshot(self)
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }

    fn wire_api(&self) -> ProviderWireApi {
        ProviderRuntime::wire_api(self)
    }

    fn websocket_connect_timeout(&self) -> Duration {
        ProviderRuntime::websocket_connect_timeout(self)
    }

    fn stream_max_retries(&self) -> u64 {
        ProviderRuntime::stream_max_retries(self)
    }

    fn connect_websocket(
        &self,
        request: ProviderWebsocketConnectRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<SharedProviderWebsocketConnection>,
    > {
        ProviderRuntime::connect_websocket(self, request)
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

    fn model_catalog_identity(&self) -> crate::contract::ProviderResult<String> {
        let auth =
            ProviderRuntime::credential_manager(self).and_then(|manager| manager.auth_cached());
        crate::models_identity::identity(
            ProviderRuntime::provider_id(self),
            ProviderRuntime::info(self),
            auth.as_ref(),
        )
        .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
    }

    fn chat(
        &self,
        request: crate::contract::ChatRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderChatStream>,
    > {
        ProviderRuntime::chat(self, request)
    }

    fn search(
        &self,
        request: crate::contract::ProviderSearchRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderSearchResponse>,
    > {
        ProviderRuntime::search(self, request)
    }

    fn backend(
        &self,
        request: crate::contract::ProviderBackendRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderBackendResponse>,
    > {
        ProviderRuntime::backend(self, request)
    }

    fn auth_recovery_messages(&self) -> Option<crate::contract::ProviderAuthRecoveryMessages> {
        ProviderRuntime::auth_recovery_messages(self)
    }

    fn recover_from_unauthorized(
        &self,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderUnauthorizedRecovery>,
    > {
        let runtime = self;
        Box::pin(async move {
            ProviderRuntime::recover_from_unauthorized(runtime)
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }

    fn list_models(
        &self,
        request: crate::contract::ListModelsRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderModelCatalog>,
    > {
        let client_version = request.client_version;
        let http_client_factory = request.transport.http_client_factory().clone();
        Box::pin(async move {
            ProviderRuntime::list_models(self, &client_version, http_client_factory)
                .await
                .map(|snapshot| crate::contract::ProviderModelCatalog {
                    catalog: snapshot.catalog,
                    identity: snapshot.identity,
                    etag: snapshot.etag,
                })
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }

    fn auth_state(&self) -> crate::contract::ProviderAuthState {
        match ProviderRuntime::account_state(self) {
            Ok(state) => crate::contract::ProviderAuthState {
                status: state.auth_status,
                account_id: state.account_id,
                display_name: state.display_name,
            },
            Err(error) => crate::contract::ProviderAuthState {
                status: ProviderAuthStatus::Invalid,
                account_id: None,
                display_name: Some(error.to_string()),
            },
        }
    }

    fn auth_fields(&self) -> Vec<crate::contract::ProviderAuthField> {
        ProviderRuntime::auth_fields(self)
            .into_iter()
            .map(|field| {
                crate::contract::ProviderAuthField::new(field.id, field.label, field.secret)
            })
            .collect()
    }

    fn account_state(
        &self,
    ) -> crate::contract::ProviderResult<crate::contract::ProviderAccountState> {
        ProviderRuntime::account_state(self)
            .map(|state| crate::contract::ProviderAccountState {
                provider_id: state.provider_id,
                account: state.account,
                account_id: state.account_id,
                display_name: state.display_name,
                auth_status: state.auth_status,
                usage: state.usage,
            })
            .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
    }

    fn login(
        &self,
        credentials: BTreeMap<String, String>,
    ) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<()>> {
        let api_key = credentials.get("api_key").cloned().unwrap_or_default();
        Box::pin(async move {
            ProviderRuntime::login(self, ProviderLoginRequest::new(api_key, credentials))
                .await
                .map_err(|error| match error {
                    ProviderAuthError::Unavailable => crate::contract::ProviderError::Unsupported,
                    ProviderAuthError::Failed(message) => {
                        crate::contract::ProviderError::Provider(message)
                    }
                })
        })
    }

    fn logout(&self) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<()>> {
        Box::pin(async move {
            ProviderRuntime::logout(self)
                .await
                .map(|_| ())
                .map_err(|error| match error {
                    ProviderAuthError::Unavailable => crate::contract::ProviderError::Unsupported,
                    ProviderAuthError::Failed(message) => {
                        crate::contract::ProviderError::Provider(message)
                    }
                })
        })
    }

    fn delete(&self) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<bool>> {
        Box::pin(async move {
            ProviderRuntime::delete(self)
                .await
                .map_err(|error| match error {
                    ProviderAuthError::Unavailable => crate::contract::ProviderError::Unsupported,
                    ProviderAuthError::Failed(message) => {
                        crate::contract::ProviderError::Provider(message)
                    }
                })
        })
    }

    fn usage(
        &self,
        _request: crate::contract::UsageRequest,
    ) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<ProviderUsage>> {
        Box::pin(async move {
            ProviderRuntime::usage(self)
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }
}

/// Adapts the legacy transport-backed runtime to the provider-neutral host contract.
///
/// The adapter intentionally shares the runtime allocation. It is a transition aid while
/// execution moves to `ModelProvider`; it must not create a second credential or provider state.
#[derive(Clone, Debug)]
struct RuntimeProviderContractAdapter {
    runtime: SharedProviderRuntime,
}

impl crate::contract::ModelProvider for RuntimeProviderContractAdapter {
    fn descriptor(&self) -> ProviderDescriptor {
        self.runtime.descriptor()
    }

    fn wire_api(&self) -> ProviderWireApi {
        self.runtime.wire_api()
    }

    fn websocket_connect_timeout(&self) -> Duration {
        self.runtime.websocket_connect_timeout()
    }

    fn stream_max_retries(&self) -> u64 {
        self.runtime.stream_max_retries()
    }

    fn connect_websocket(
        &self,
        request: ProviderWebsocketConnectRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<SharedProviderWebsocketConnection>,
    > {
        ProviderRuntime::connect_websocket(self.runtime.as_ref(), request)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        let mut capabilities = self.runtime.capabilities();
        capabilities.api_key_model_discovery = capabilities.api_key_model_discovery
            || self.runtime.api_key_model_discovery_enabled_by_default()
            || self.runtime.info().env_key.is_some();
        capabilities.command_auth =
            capabilities.command_auth || self.runtime.info().has_command_auth();
        capabilities
    }

    fn model_catalog_identity(&self) -> crate::contract::ProviderResult<String> {
        let auth = self
            .runtime
            .credential_manager()
            .and_then(|manager| manager.auth_cached());
        crate::models_identity::identity(
            self.runtime.provider_id(),
            self.runtime.info(),
            auth.as_ref(),
        )
        .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
    }

    fn chat(
        &self,
        request: crate::contract::ChatRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderChatStream>,
    > {
        let runtime = self.runtime.clone();
        Box::pin(async move { ProviderRuntime::chat(runtime.as_ref(), request).await })
    }

    fn search(
        &self,
        request: crate::contract::ProviderSearchRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderSearchResponse>,
    > {
        let runtime = self.runtime.clone();
        Box::pin(async move { ProviderRuntime::search(runtime.as_ref(), request).await })
    }

    fn backend(
        &self,
        request: crate::contract::ProviderBackendRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderBackendResponse>,
    > {
        let runtime = self.runtime.clone();
        Box::pin(async move { ProviderRuntime::backend(runtime.as_ref(), request).await })
    }

    fn auth_recovery_messages(&self) -> Option<crate::contract::ProviderAuthRecoveryMessages> {
        self.runtime.auth_recovery_messages()
    }

    fn recover_from_unauthorized(
        &self,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderUnauthorizedRecovery>,
    > {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            ProviderRuntime::recover_from_unauthorized(runtime.as_ref())
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }

    fn list_models(
        &self,
        request: crate::contract::ListModelsRequest,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderModelCatalog>,
    > {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .list_models(
                    &request.client_version,
                    request.transport.http_client_factory().clone(),
                )
                .await
                .map(|snapshot| crate::contract::ProviderModelCatalog {
                    catalog: snapshot.catalog,
                    identity: snapshot.identity,
                    etag: snapshot.etag,
                })
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }

    fn auth_state(&self) -> crate::contract::ProviderAuthState {
        match self.runtime.account_state() {
            Ok(state) => crate::contract::ProviderAuthState {
                status: state.auth_status,
                account_id: state.account_id,
                display_name: state.display_name,
            },
            Err(error) => crate::contract::ProviderAuthState {
                status: ProviderAuthStatus::Invalid,
                account_id: None,
                display_name: Some(error.to_string()),
            },
        }
    }

    fn request_snapshot(
        &self,
    ) -> crate::contract::ProviderFuture<
        '_,
        crate::contract::ProviderResult<crate::contract::ProviderRequestSnapshot>,
    > {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            ProviderRuntime::request_snapshot(runtime.as_ref())
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }

    fn auth_fields(&self) -> Vec<crate::contract::ProviderAuthField> {
        self.runtime
            .auth_fields()
            .into_iter()
            .map(|field| {
                crate::contract::ProviderAuthField::new(field.id, field.label, field.secret)
            })
            .collect()
    }

    fn account_state(
        &self,
    ) -> crate::contract::ProviderResult<crate::contract::ProviderAccountState> {
        self.runtime
            .account_state()
            .map(|state| crate::contract::ProviderAccountState {
                provider_id: state.provider_id,
                account: state.account,
                account_id: state.account_id,
                display_name: state.display_name,
                auth_status: state.auth_status,
                usage: state.usage,
            })
            .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
    }

    fn login(
        &self,
        credentials: BTreeMap<String, String>,
    ) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<()>> {
        let api_key = credentials.get("api_key").cloned().unwrap_or_default();
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .login(ProviderLoginRequest::new(api_key, credentials))
                .await
                .map_err(|error| match error {
                    ProviderAuthError::Unavailable => crate::contract::ProviderError::Unsupported,
                    ProviderAuthError::Failed(message) => {
                        crate::contract::ProviderError::Provider(message)
                    }
                })
        })
    }

    fn logout(&self) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<()>> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .logout()
                .await
                .map(|_| ())
                .map_err(|error| match error {
                    ProviderAuthError::Unavailable => crate::contract::ProviderError::Unsupported,
                    ProviderAuthError::Failed(message) => {
                        crate::contract::ProviderError::Provider(message)
                    }
                })
        })
    }

    fn delete(&self) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<bool>> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime.delete().await.map_err(|error| match error {
                ProviderAuthError::Unavailable => crate::contract::ProviderError::Unsupported,
                ProviderAuthError::Failed(message) => {
                    crate::contract::ProviderError::Provider(message)
                }
            })
        })
    }

    fn usage(
        &self,
        _request: crate::contract::UsageRequest,
    ) -> crate::contract::ProviderFuture<'_, crate::contract::ProviderResult<ProviderUsage>> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .usage()
                .await
                .map_err(|error| crate::contract::ProviderError::Provider(error.to_string()))
        })
    }
}

/// Creates the provider-neutral view of an existing runtime provider without duplicating state.
pub fn provider_contract(provider: SharedProviderRuntime) -> crate::contract::SharedModelProvider {
    Arc::new(RuntimeProviderContractAdapter { runtime: provider })
}

pub type ModelProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Shared runtime model provider handle.
pub type SharedProviderRuntime = Arc<dyn ProviderRuntime>;

fn provider_uses_stored_auth_path(provider: &ModelProviderInfo) -> bool {
    provider.env_key.is_none()
        && provider.experimental_bearer_token.is_none()
        && provider.auth.is_none()
}

/// Creates the default runtime model provider for configured provider metadata.
pub fn create_model_provider(
    provider_info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
) -> SharedProviderRuntime {
    let provider_id = provider_info.id.clone();
    create_model_provider_internal(provider_id, provider_info, credential_manager)
}

fn create_model_provider_internal(
    provider_id: Option<String>,
    provider_info: ModelProviderInfo,
    credential_manager: Option<Arc<CredentialManager>>,
) -> SharedProviderRuntime {
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

impl ProviderRuntime for ConfiguredModelProvider {
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

    #[tokio::test]
    async fn provider_contract_adapts_lifecycle_without_new_provider_state() {
        let runtime = create_model_provider(
            ModelProviderInfo::default(),
            /*credential_manager*/ None,
        );
        let provider = provider_contract(runtime);

        assert_eq!(provider.descriptor().provider_kind, "Responses provider");
        assert_eq!(provider.auth_fields()[0].id, "api_key");
        assert!(matches!(
            provider.delete().await,
            Err(crate::ProviderError::Unsupported)
        ));
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

    #[tokio::test]
    async fn provider_exposes_neutral_login_lifecycle() {
        let provider = create_model_provider(
            ModelProviderInfo::default(),
            /*credential_manager*/ None,
        );

        let request = ProviderLoginRequest::new("api-key", std::collections::BTreeMap::new());
        let result = provider.login(request).await;
        assert!(matches!(result, Err(ProviderAuthError::Unavailable)));

        let logout_result: ProviderLogoutResult = provider.logout().await;
        assert!(matches!(logout_result, Err(ProviderAuthError::Unavailable)));
    }

    #[test]
    fn provider_model_catalog_source_preserves_provider_capabilities() {
        let mut info = ModelProviderInfo::create_cloudflare_provider();
        info.base_url = Some("https://example.test".to_string());
        info.env_key = None;
        let provider = create_model_provider(info, /*credential_manager*/ None);
        let source = ProviderModelCatalogSource::new(provider_contract(provider));

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
            Arc::new(ProviderModelCatalogSource::new(provider_contract(
                provider.clone(),
            ))),
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
