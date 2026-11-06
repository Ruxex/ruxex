use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use http::HeaderMap;
use http::HeaderValue;
use http::StatusCode;
use http::header::AUTHORIZATION;
use ruxex::api::ApiError;
use ruxex::api::AuthProvider;
use ruxex::api::ChatClient;
use ruxex::api::ChatCompletionApi;
use ruxex::api::ChatRequestInput;
use ruxex::api::ModelsClient;
use ruxex::api::Provider;
use ruxex::api::ReqwestTransport;
use ruxex::api::ResponseEvent;
use ruxex::api::SharedAuthProvider;
use ruxex::api::StandardChatCompletionApi;
use ruxex::api::TransportError;
use ruxex::client::create_client_for_route_async;
use ruxex::provider::ChatRequest;
use ruxex::provider::ListModelsRequest;
use ruxex::provider::ModelProvider;
use ruxex::provider::ModelProviderFactory;
use ruxex::provider::ProviderAccountState;
use ruxex::provider::ProviderAuthEnvironment;
use ruxex::provider::ProviderCapabilities;
use ruxex::provider::ProviderChatEvent;
use ruxex::provider::ProviderConnectionKey;
use ruxex::provider::ProviderCredentialSnapshot;
use ruxex::provider::ProviderDescriptor;
use ruxex::provider::ProviderError;
use ruxex::provider::ProviderFuture;
use ruxex::provider::ProviderModelCatalog;
use ruxex::provider::ProviderRequestAuth;
use ruxex::provider::ProviderRequestFormat;
use ruxex::provider::ProviderRequestSnapshot;
use ruxex::provider::ProviderResult;
use ruxex::provider::ProviderUsage;
use ruxex::provider::SharedProviderCredentialSnapshot;
use ruxex::provider::UsageRequest;
use ruxex::provider::auth::CredentialManager;
use ruxex::provider::auth::ProviderAuth;
use ruxex::provider::auth::RuxexAuth;
use ruxex::provider::config::ModelProviderInfo;
use ruxex::provider::protocol::ModelCatalog;
use ruxex::provider::protocol::ModelInfo;
use ruxex::provider::protocol::ProviderAccount;
use ruxex::provider::protocol::ProviderAuthStatus;
use ruxex::provider::transport::ClientRouteClass;
use ruxex::provider::transport::HttpClientFactory;
use ruxex_app_server::AppServerRuntimeBuilder;
use ruxex_cli::ProviderRuntimeFactory;
use ruxex_protocol::models::ResponseItem;
use serde_json::Value;

const CLOUDFLARE_API_ROOT: &str = "https://api.cloudflare.com/client/v4";
const CLOUDFLARE_PROVIDER_ID: &str = "cloudflare";
const CLOUDFLARE_API_TOKEN_ENV_VAR: &str = "CLOUDFLARE_API_TOKEN";
const CLOUDFLARE_ACCOUNT_ID_ENV_VAR: &str = "CLOUDFLARE_ACCOUNT_ID";
const CLOUDFLARE_MODEL_ENV_VAR: &str = "CLOUDFLARE_MODEL";
const CLOUDFLARE_TEXT_GENERATION_TASK: &str = "Text%20Generation";

#[derive(Clone, Debug)]
pub struct CloudflareProvider {
    info: ModelProviderInfo,
    credential_manager: Arc<CredentialManager>,
    http_client_factory: HttpClientFactory,
}

#[derive(Clone, Debug)]
struct CloudflareCredentials {
    token: String,
    account_id: String,
    auth_revision: Option<u64>,
}

#[derive(Debug)]
struct CloudflareCredentialSnapshot(CloudflareCredentials);

impl ProviderCredentialSnapshot for CloudflareCredentialSnapshot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct BearerAuthProvider {
    token: String,
}

impl AuthProvider for BearerAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", self.token)) {
            headers.insert(AUTHORIZATION, value);
        }
    }
}

impl CloudflareProvider {
    pub fn chat_base_url_for_account(account_id: &str) -> String {
        format!("{CLOUDFLARE_API_ROOT}/accounts/{account_id}/ai/v1")
    }

    fn chat_base_url(&self, account_id: &str) -> String {
        self.info
            .base_url
            .clone()
            .unwrap_or_else(|| Self::chat_base_url_for_account(account_id))
    }

    fn models_base_url(&self, account_id: &str) -> String {
        self.info
            .base_url
            .clone()
            .unwrap_or_else(|| format!("{CLOUDFLARE_API_ROOT}/accounts/{account_id}/ai"))
    }

    fn api_provider(&self, base_url: String) -> ProviderResult<Provider> {
        let mut info = self.info.clone();
        info.base_url = Some(base_url);
        info.to_api_provider(None)
            .map_err(|error| ProviderError::Provider(error.to_string()))
    }

    async fn credentials(&self) -> ProviderResult<CloudflareCredentials> {
        let auth = self.credential_manager.auth().await;
        let token = auth
            .as_ref()
            .and_then(RuxexAuth::api_key)
            .map(ToOwned::to_owned)
            .or_else(|| self.info.api_key().ok().flatten())
            .ok_or(ProviderError::Unauthenticated)?;
        let account_id = auth
            .as_ref()
            .and_then(|auth| auth.credential("account_id"))
            .or_else(|| {
                std::env::var(CLOUDFLARE_ACCOUNT_ID_ENV_VAR)
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .ok_or_else(|| {
                ProviderError::InvalidRequest(format!(
                    "{CLOUDFLARE_ACCOUNT_ID_ENV_VAR} must be set for Cloudflare Workers AI"
                ))
            })?;
        let auth_revision = Some(*self.credential_manager.auth_change_receiver().borrow());

        Ok(CloudflareCredentials {
            token,
            account_id,
            auth_revision,
        })
    }

    async fn credentials_for_request(
        &self,
        snapshot: Option<&SharedProviderCredentialSnapshot>,
    ) -> ProviderResult<CloudflareCredentials> {
        if let Some(credentials) = snapshot.and_then(|snapshot| {
            snapshot
                .as_any()
                .downcast_ref::<CloudflareCredentialSnapshot>()
        }) {
            return Ok(credentials.0.clone());
        }
        self.credentials().await
    }

    fn auth_provider(credentials: &CloudflareCredentials) -> SharedAuthProvider {
        Arc::new(BearerAuthProvider {
            token: credentials.token.clone(),
        })
    }

    fn auth_state_from_credentials(
        &self,
        auth: Option<&RuxexAuth>,
    ) -> (ProviderAuthStatus, Option<String>, Option<ProviderAccount>) {
        let token = auth.and_then(RuxexAuth::api_key).is_some()
            || self.info.api_key().ok().flatten().is_some();
        let account_id = auth
            .and_then(|auth| auth.credential("account_id"))
            .or_else(|| {
                std::env::var(CLOUDFLARE_ACCOUNT_ID_ENV_VAR)
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            });
        (
            if token {
                ProviderAuthStatus::Authenticated
            } else {
                ProviderAuthStatus::Unauthenticated
            },
            account_id,
            token.then_some(ProviderAccount::ApiKey),
        )
    }
}

impl ModelProviderFactory for CloudflareProvider {
    fn provider_descriptor() -> ProviderDescriptor {
        ProviderDescriptor::new(
            CLOUDFLARE_PROVIDER_ID,
            ruxex::provider::CURRENT_PROVIDER_PROTOCOL_VERSION,
        )
    }

    fn new(init: ruxex::provider::ProviderInit) -> Self {
        Self {
            info: init.provider_info,
            credential_manager: init.credential_manager,
            http_client_factory: init.http_client_factory,
        }
    }
}

pub fn provider_factory() -> ProviderRuntimeFactory {
    (
        AppServerRuntimeBuilder::build_from_provider_factory::<CloudflareProvider>,
        CloudflareProvider::provider_descriptor(),
    )
}

impl ModelProvider for CloudflareProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        Self::provider_descriptor()
    }

    fn request_format(&self) -> ProviderRequestFormat {
        ProviderRequestFormat::Chat
    }

    fn model_catalog_identity(&self) -> ProviderResult<String> {
        let account_id = self
            .credential_manager
            .auth_cached()
            .and_then(|auth| auth.credential("account_id"))
            .or_else(|| std::env::var(CLOUDFLARE_ACCOUNT_ID_ENV_VAR).ok())
            .ok_or_else(|| {
                ProviderError::InvalidRequest(
                    "Cloudflare account identity is unavailable".to_string(),
                )
            })?;
        Ok(format!("{CLOUDFLARE_PROVIDER_ID}:{account_id}"))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            api_key_model_discovery: true,
            web_search: false,
            supports_tool_strict_mode: self.info.supports_tool_strict_mode,
            websocket: false,
            usage: false,
            ..ProviderCapabilities::default()
        }
    }

    fn chat(
        &self,
        request: ChatRequest,
    ) -> ProviderFuture<'_, ProviderResult<ruxex::provider::ProviderChatStream>> {
        Box::pin(async move {
            let credentials = self
                .credentials_for_request(request.transport.credential_snapshot.as_ref())
                .await?;
            let provider = self.api_provider(self.chat_base_url(&credentials.account_id))?;
            let request_url = provider.url_for_path("chat/completions");
            let client = create_client_for_route_async(
                self.http_client_factory.clone(),
                request_url,
                ClientRouteClass::Api,
            )
            .await
            .map_err(|error| ProviderError::Transport(error.to_string()))?;
            let api = Arc::new(StandardChatCompletionApi);
            let model = cloudflare_model_for_request(
                &request.model,
                configured_cloudflare_model().as_deref(),
            );
            let mut chat_request = api
                .build_request(ChatRequestInput {
                    provider: &provider,
                    model: &model,
                    instructions: &request.instructions,
                    input: &request.input,
                    tools: &request.tools,
                    conversation_id: request.conversation_id,
                    session_source: request.session_source,
                })
                .map_err(map_api_error_to_provider_error)?;
            normalize_cloudflare_chat_request(&mut chat_request.body);
            let stream = ChatClient::with_api(
                ReqwestTransport::from_http_client(client),
                provider,
                Self::auth_provider(&credentials),
                api,
            )
            .stream_request(chat_request)
            .await
            .map_err(map_api_error_to_provider_error)?;

            Ok(Box::pin(stream.map(|event| {
                event
                    .map(provider_chat_event_from_api)
                    .map_err(map_api_error_to_provider_error)
            })) as ruxex::provider::ProviderChatStream)
        })
    }

    fn list_models(
        &self,
        request: ListModelsRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>> {
        Box::pin(async move {
            let credentials = self.credentials().await?;
            let auth = Self::auth_provider(&credentials);
            let configured_model = configured_cloudflare_model();
            let mut models = Vec::new();
            let mut etag = None;

            for page in 1..=100 {
                let provider = self.api_provider(self.models_base_url(&credentials.account_id))?;
                let request_url = format!(
                    "{}?task={CLOUDFLARE_TEXT_GENERATION_TASK}&hide_experimental=true&per_page=100&page={page}&client_version={}",
                    provider.url_for_path("models/search"),
                    request.client_version
                );
                let client = create_client_for_route_async(
                    self.http_client_factory.clone(),
                    request_url.clone(),
                    ClientRouteClass::Api,
                )
                .await
                .map_err(|error| ProviderError::Transport(error.to_string()))?;
                let (body, page_etag) = ModelsClient::new(
                    ReqwestTransport::from_http_client(client),
                    provider,
                    auth.clone(),
                )
                .list_models_raw(request_url, HeaderMap::new(), Some(8 * 1024 * 1024))
                .await
                .map_err(map_api_error_to_provider_error)?;
                let decoded = decode_cloudflare_models(&body, page)?;
                models.extend(decoded.models);
                etag = page_etag.or(etag);
                if !decoded.has_more {
                    break;
                }
            }
            let models = select_configured_cloudflare_models(models, configured_model.as_deref())?;

            Ok(ProviderModelCatalog {
                catalog: ModelCatalog { models },
                identity: format!("{CLOUDFLARE_PROVIDER_ID}:{}", credentials.account_id),
                etag,
            })
        })
    }

    fn auth_state(&self) -> ruxex::provider::ProviderAuthState {
        let auth = self.credential_manager.auth_cached();
        let (status, account_id, _) = self.auth_state_from_credentials(auth.as_ref());
        ruxex::provider::ProviderAuthState {
            status,
            account_id,
            display_name: None,
            managed_auth: false,
            revision: Some(*self.credential_manager.auth_change_receiver().borrow()),
        }
    }

    fn auth_environment(&self) -> ProviderAuthEnvironment {
        ProviderAuthEnvironment {
            host_api_key_env_present: std::env::var_os("RUXEX_API_KEY").is_some(),
            host_api_key_env_enabled: self.credential_manager.api_key_env_enabled(),
            provider_env_key_configured: true,
            provider_env_key_present: Some(
                std::env::var_os(CLOUDFLARE_API_TOKEN_ENV_VAR).is_some(),
            ),
        }
    }

    fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>> {
        Box::pin(async move {
            let credentials = self.credentials().await?;
            let revision = credentials.auth_revision;
            Ok(ProviderRequestSnapshot {
                auth: ProviderRequestAuth {
                    authenticated: true,
                    managed_auth: false,
                    auth_header_attached: true,
                    auth_header_name: Some("authorization"),
                    revision,
                    owner_generation: None,
                },
                connection_key: ProviderConnectionKey {
                    provider_kind: CLOUDFLARE_PROVIDER_ID.to_string(),
                    auth_revision: revision,
                },
                credential_snapshot: Arc::new(CloudflareCredentialSnapshot(credentials)),
            })
        })
    }

    fn account_state(&self) -> ProviderResult<ProviderAccountState> {
        let auth = self.credential_manager.auth_cached();
        let (auth_status, account_id, account) = self.auth_state_from_credentials(auth.as_ref());
        Ok(ProviderAccountState {
            provider_id: CLOUDFLARE_PROVIDER_ID.to_string(),
            account,
            account_id,
            display_name: None,
            auth_status,
            usage: None,
        })
    }

    fn login(
        &self,
        mut credentials: BTreeMap<String, String>,
    ) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async move {
            let api_key = credentials
                .remove("api_key")
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| ProviderError::InvalidRequest("api_key is required".to_string()))?;
            self.credential_manager
                .login(ProviderAuth::api_key_with_credentials(api_key, credentials))
                .await
                .map_err(|error| ProviderError::Provider(error.to_string()))
        })
    }

    fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async move {
            self.credential_manager
                .logout()
                .await
                .map(|_| ())
                .map_err(|error| ProviderError::Provider(error.to_string()))
        })
    }

    fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>> {
        Box::pin(async move {
            self.credential_manager
                .delete()
                .await
                .map_err(|error| ProviderError::Provider(error.to_string()))
        })
    }

    fn usage(&self, _request: UsageRequest) -> ProviderFuture<'_, ProviderResult<ProviderUsage>> {
        Box::pin(async { Ok(ProviderUsage::Unsupported) })
    }
}

fn provider_chat_event_from_api(event: ResponseEvent) -> ProviderChatEvent {
    match event {
        ResponseEvent::Created { response_id } => ProviderChatEvent::Created { response_id },
        ResponseEvent::SafetyBuffering(buffering) => {
            ProviderChatEvent::SafetyBuffering(ruxex::provider::ProviderSafetyBuffering {
                use_cases: buffering.use_cases,
                reasons: buffering.reasons,
                show_buffering_ui: buffering.show_buffering_ui,
                faster_model: buffering.faster_model,
            })
        }
        ResponseEvent::OutputItemDone(item) => {
            ProviderChatEvent::OutputItemDone(normalize_cloudflare_tool_arguments(item))
        }
        ResponseEvent::OutputItemAdded(item) => ProviderChatEvent::OutputItemAdded(item),
        ResponseEvent::ServerModel(model) => ProviderChatEvent::ServerModel(model),
        ResponseEvent::ModelVerifications(verifications) => {
            ProviderChatEvent::ModelVerifications(verifications)
        }
        ResponseEvent::ServerReasoningIncluded(included) => {
            ProviderChatEvent::ServerReasoningIncluded(included)
        }
        ResponseEvent::Completed {
            response_id,
            token_usage,
            usage_metadata,
            end_turn,
        } => ProviderChatEvent::Completed {
            response_id,
            token_usage,
            usage_metadata,
            end_turn,
        },
        ResponseEvent::OutputTextDelta(delta) => ProviderChatEvent::OutputTextDelta(delta),
        ResponseEvent::ToolCallInputDelta {
            item_id,
            call_id,
            delta,
        } => ProviderChatEvent::ToolCallInputDelta {
            item_id,
            call_id,
            delta,
        },
        ResponseEvent::ReasoningSummaryDelta {
            delta,
            summary_index,
        } => ProviderChatEvent::ReasoningSummaryDelta {
            delta,
            summary_index,
        },
        ResponseEvent::ReasoningSummaryDone {
            item_id,
            text,
            summary_index,
        } => ProviderChatEvent::ReasoningSummaryDone {
            item_id,
            text,
            summary_index,
        },
        ResponseEvent::ReasoningContentDelta {
            delta,
            content_index,
        } => ProviderChatEvent::ReasoningContentDelta {
            delta,
            content_index,
        },
        ResponseEvent::ReasoningSummaryPartAdded { summary_index } => {
            ProviderChatEvent::ReasoningSummaryPartAdded { summary_index }
        }
        ResponseEvent::RateLimits(snapshot) => ProviderChatEvent::RateLimits(snapshot),
        ResponseEvent::ModelsEtag(etag) => ProviderChatEvent::ModelsEtag(etag),
    }
}

fn configured_cloudflare_model() -> Option<String> {
    std::env::var(CLOUDFLARE_MODEL_ENV_VAR)
        .ok()
        .map(|model| model.trim().to_owned())
        .filter(|model| !model.is_empty())
}

fn cloudflare_model_for_request(request_model: &str, configured_model: Option<&str>) -> String {
    configured_model
        .filter(|model| !model.trim().is_empty())
        .unwrap_or(request_model)
        .to_owned()
}

fn select_configured_cloudflare_models(
    models: Vec<ModelInfo>,
    configured_model: Option<&str>,
) -> ProviderResult<Vec<ModelInfo>> {
    let Some(configured_model) = configured_model else {
        return Ok(models);
    };

    let selected: Vec<_> = models
        .into_iter()
        .filter(|model| model.slug == configured_model)
        .collect();
    if selected.is_empty() {
        return Err(ProviderError::InvalidRequest(format!(
            "{CLOUDFLARE_MODEL_ENV_VAR}={configured_model} was not found in the Cloudflare model catalog"
        )));
    }

    Ok(selected)
}

fn normalize_cloudflare_tool_arguments(item: ResponseItem) -> ResponseItem {
    let ResponseItem::FunctionCall {
        id,
        namespace,
        name,
        arguments,
        encrypted_function_args,
        call_id,
        internal_chat_message_metadata_passthrough,
    } = item
    else {
        return item;
    };

    let arguments = unwrap_nested_json_arguments(&arguments).unwrap_or(arguments);
    ResponseItem::FunctionCall {
        id,
        namespace,
        name,
        arguments,
        encrypted_function_args,
        call_id,
        internal_chat_message_metadata_passthrough,
    }
}

fn unwrap_nested_json_arguments(arguments: &str) -> Option<String> {
    let Value::String(inner) = serde_json::from_str(arguments).ok()? else {
        return None;
    };

    match serde_json::from_str::<Value>(&inner).ok()? {
        Value::Object(_) | Value::Array(_) => Some(inner),
        _ => None,
    }
}

/// Workers AI models do not consistently accept the full OpenAI Chat Completions
/// message shape during a multi-turn tool call. Keep this compatibility change
/// scoped to Cloudflare instead of changing the shared request builder.
fn normalize_cloudflare_chat_request(body: &mut Value) {
    let Some(messages) = body.get_mut("messages").and_then(Value::as_array_mut) else {
        return;
    };

    for message in messages {
        let Some(message) = message.as_object_mut() else {
            continue;
        };
        let content = message.remove("content").unwrap_or(Value::Null);
        message.insert(
            "content".to_string(),
            normalize_cloudflare_message_content(content),
        );
    }
}

fn normalize_cloudflare_message_content(content: Value) -> Value {
    match content {
        Value::Null => Value::String(String::new()),
        Value::String(_) => content,
        Value::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                match &part {
                    Value::String(value) => text.push_str(value),
                    Value::Object(object) => {
                        if let Some(value) = object.get("text").and_then(Value::as_str) {
                            text.push_str(value);
                        } else {
                            text.push_str(&part.to_string());
                        }
                    }
                    _ => text.push_str(&part.to_string()),
                }
            }
            Value::String(text)
        }
        value => Value::String(value.to_string()),
    }
}

fn map_api_error_to_provider_error(error: ApiError) -> ProviderError {
    match error {
        ApiError::Transport(TransportError::Http { status, .. })
            if status == StatusCode::UNAUTHORIZED =>
        {
            ProviderError::Unauthorized
        }
        ApiError::InvalidRequest { message } => ProviderError::InvalidRequest(message),
        ApiError::RateLimitExceeded { delay, .. } => ProviderError::RateLimited {
            retry_after_seconds: delay.map(|delay| delay.as_secs()),
        },
        ApiError::Transport(TransportError::Timeout) => {
            ProviderError::Transient("Cloudflare request timed out".to_string())
        }
        ApiError::Transport(error) => ProviderError::Transport(error.to_string()),
        other => ProviderError::Provider(other.to_string()),
    }
}

#[derive(Debug)]
struct DecodedCloudflareModels {
    models: Vec<ModelInfo>,
    has_more: bool,
}

fn decode_cloudflare_models(body: &[u8], page: usize) -> ProviderResult<DecodedCloudflareModels> {
    let payload: Value = serde_json::from_slice(body)
        .map_err(|error| ProviderError::InvalidRequest(error.to_string()))?;
    if payload.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(ProviderError::Provider(
            "Cloudflare model search failed".to_string(),
        ));
    }

    let result = payload
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProviderError::InvalidRequest(
                "Cloudflare model search returned no result array".to_string(),
            )
        })?;
    let models = result
        .iter()
        .filter_map(|model| {
            let slug = ["name", "model", "id", "slug"]
                .iter()
                .find_map(|field| model.get(*field).and_then(Value::as_str))?;
            let mut info = ruxex::catalog::model_info_template_from_slug(slug);
            info.visibility = ruxex::provider::protocol::ModelVisibility::List;
            info.description = model
                .get("description")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            Some(info)
        })
        .collect();
    let has_more = payload
        .get("result_info")
        .and_then(|info| info.get("total_pages"))
        .and_then(Value::as_u64)
        .is_some_and(|total_pages| page < total_pages as usize);

    Ok(DecodedCloudflareModels { models, has_more })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruxex::provider::config::WireApi;
    use ruxex::provider::transport::OutboundProxyPolicy;
    use ruxex_model_provider_contract_tests::{ChatContractConfig, assert_provider_chat_contract};
    use ruxex_protocol::models::ContentItem;

    #[test]
    fn cloudflare_normalizes_multiturn_message_content() {
        let mut body = serde_json::json!({
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "run ls"}]},
                {"role": "assistant", "content": null, "tool_calls": []},
                {"role": "tool", "tool_call_id": "call-1", "content": [{"type": "text", "text": "file.txt"}]},
                {"role": "assistant", "tool_calls": []}
            ]
        });

        normalize_cloudflare_chat_request(&mut body);

        assert_eq!(body["messages"][0]["content"], serde_json::json!("run ls"));
        assert_eq!(body["messages"][1]["content"], serde_json::json!(""));
        assert_eq!(
            body["messages"][2]["content"],
            serde_json::json!("file.txt")
        );
        assert_eq!(body["messages"][3]["content"], serde_json::json!(""));
    }

    #[test]
    fn cloudflare_unwraps_nested_tool_arguments() {
        let nested_arguments = serde_json::to_string(&serde_json::json!({
            "cmd": "ls"
        }))
        .expect("tool arguments should serialize");
        let event = provider_chat_event_from_api(ResponseEvent::OutputItemDone(
            ResponseItem::FunctionCall {
                id: None,
                namespace: None,
                name: "exec_command".to_string(),
                arguments: serde_json::to_string(&nested_arguments)
                    .expect("nested tool arguments should serialize"),
                encrypted_function_args: None,
                call_id: "call-1".to_string(),
                internal_chat_message_metadata_passthrough: None,
            },
        ));

        let ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall { arguments, .. }) = event
        else {
            panic!("expected a normalized function call event");
        };

        assert_eq!(arguments, nested_arguments);
    }

    #[test]
    fn cloudflare_chat_base_url_uses_the_account_id() {
        assert_eq!(
            CloudflareProvider::chat_base_url_for_account("account-123"),
            "https://api.cloudflare.com/client/v4/accounts/account-123/ai/v1"
        );
    }

    #[test]
    fn cloudflare_model_search_response_decodes_model_slugs_and_descriptions() {
        let models = decode_cloudflare_models(
            br#"{
                "success": true,
                "result": [{
                    "name": "@cf/meta/llama-3.1-8b-instruct",
                    "description": "Llama 3.1 8B instruct"
                }]
            }"#,
            1,
        )
        .expect("Cloudflare model response should decode")
        .models;

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].slug, "@cf/meta/llama-3.1-8b-instruct");
        assert_eq!(
            models[0].visibility,
            ruxex::provider::protocol::ModelVisibility::List
        );
        assert_eq!(
            models[0].description.as_deref(),
            Some("Llama 3.1 8B instruct")
        );
    }

    #[test]
    fn cloudflare_model_catalog_can_be_limited_to_a_configured_model() {
        let models = decode_cloudflare_models(
            br#"{
                "success": true,
                "result": [
                    {"name": "@cf/meta/llama-3.1-8b-instruct"},
                    {"name": "@cf/meta/llama-3.1-70b-instruct"}
                ]
            }"#,
            1,
        )
        .expect("Cloudflare model response should decode")
        .models;

        let models =
            select_configured_cloudflare_models(models, Some("@cf/meta/llama-3.1-70b-instruct"))
                .expect("configured Cloudflare model should be available");

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].slug, "@cf/meta/llama-3.1-70b-instruct");
    }

    #[test]
    fn cloudflare_model_catalog_rejects_an_unavailable_configured_model() {
        let models = decode_cloudflare_models(
            br#"{
                "success": true,
                "result": [{"name": "@cf/meta/llama-3.1-8b-instruct"}]
            }"#,
            1,
        )
        .expect("Cloudflare model response should decode")
        .models;

        let error = select_configured_cloudflare_models(models, Some("@cf/meta/missing-model"))
            .expect_err("an unavailable configured model should fail clearly");

        assert!(error.to_string().contains("CLOUDFLARE_MODEL"));
        assert!(error.to_string().contains("@cf/meta/missing-model"));
    }

    #[test]
    fn cloudflare_chat_uses_the_configured_model_over_the_client_selection() {
        assert_eq!(
            cloudflare_model_for_request(
                "@cf/meta/llama-3.1-8b-instruct",
                Some("@cf/meta/llama-3.1-70b-instruct"),
            ),
            "@cf/meta/llama-3.1-70b-instruct"
        );
        assert_eq!(
            cloudflare_model_for_request("client-model", None),
            "client-model"
        );
    }

    #[tokio::test]
    #[ignore = "requires CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID"]
    async fn cloudflare_live_chat_satisfies_the_shared_contract() {
        let token = std::env::var(CLOUDFLARE_API_TOKEN_ENV_VAR)
            .expect("CLOUDFLARE_API_TOKEN must be set for the live contract test");
        let _account_id = std::env::var(CLOUDFLARE_ACCOUNT_ID_ENV_VAR)
            .expect("CLOUDFLARE_ACCOUNT_ID must be set for the live contract test");
        let model = std::env::var("RUXEX_TEST_MODEL")
            .unwrap_or_else(|_| "@cf/meta/llama-3.1-8b-instruct".to_string());
        let provider_info = ModelProviderInfo {
            name: "cloudflare-live-contract-test".to_string(),
            wire_api: WireApi::Chat,
            ..ModelProviderInfo::default()
        };
        let provider = CloudflareProvider::new(ruxex::provider::ProviderInit {
            provider_info,
            credential_manager: CredentialManager::from_auth_for_testing(RuxexAuth::from_api_key(
                &token,
            )),
            http_client_factory: HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        });
        let request = ChatRequest {
            model,
            instructions: "Answer briefly.".to_string(),
            input: vec![ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "Reply with the word ready.".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            }],
            tools: vec![],
            metadata: BTreeMap::new(),
            conversation_id: None,
            session_source: None,
            tool_choice: "auto".to_string(),
            parallel_tool_calls: true,
            reasoning: None,
            store: false,
            stream: true,
            stream_options: None,
            include: vec![],
            service_tier: None,
            prompt_cache_key: None,
            text: None,
            client_metadata: None,
            transport: ruxex::provider::ChatTransportOptions {
                session_id: None,
                thread_id: None,
                extra_headers: BTreeMap::new(),
                compression: ruxex::provider::ProviderRequestCompression::None,
                turn_state: None,
                connection_key: None,
                credential_snapshot: None,
            },
        };

        let report = assert_provider_chat_contract(
            &provider,
            request,
            &ChatContractConfig::default().with_live_timeout(std::time::Duration::from_secs(60)),
        )
        .await
        .expect("Cloudflare live chat should satisfy the shared contract");

        assert!(!report.assistant_text.trim().is_empty());
    }
}
