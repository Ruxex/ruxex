use std::time::Duration;

use futures::StreamExt;
use ruxex_model_provider::{
    ChatRequest, ModelProvider, ProviderChatEvent, ProviderChatStream, ProviderError,
    ProviderResult,
};
use ruxex_protocol::models::{ContentItem, ResponseItem};
use thiserror::Error;

/// Controls which normal and abnormal stream outcomes are accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatContractConfig {
    /// Require a terminal `Completed` event for a successful stream.
    pub require_completed: bool,
    /// Require assistant text from deltas or completed message content for a successful stream.
    pub require_text: bool,
    /// Accept a provider or transport error as the terminal outcome of a partial stream.
    pub allow_stream_error: bool,
    /// Maximum time a live provider stream may take to finish.
    pub live_timeout: Duration,
}

impl Default for ChatContractConfig {
    fn default() -> Self {
        Self {
            require_completed: true,
            require_text: true,
            allow_stream_error: false,
            live_timeout: Duration::from_secs(30),
        }
    }
}

impl ChatContractConfig {
    pub const fn allow_stream_error(mut self) -> Self {
        self.allow_stream_error = true;
        self
    }

    pub const fn without_required_text(mut self) -> Self {
        self.require_text = false;
        self
    }

    pub const fn with_live_timeout(mut self, timeout: Duration) -> Self {
        self.live_timeout = timeout;
        self
    }
}

/// Summary of a stream that satisfied the provider contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatContractReport {
    pub event_count: usize,
    pub assistant_text: String,
    pub completed: bool,
    pub terminated_by_error: bool,
}

#[derive(Debug, Error, PartialEq)]
pub enum ChatContractError {
    #[error("provider chat request failed before streaming: {error}")]
    ChatStart { error: ProviderError },
    #[error("provider chat stream exceeded the live timeout of {timeout:?}")]
    TimedOut { timeout: Duration },
    #[error("provider stream returned an error at event {index}: {error}")]
    StreamError { index: usize, error: ProviderError },
    #[error("event {index} appeared after the provider stream had errored")]
    EventAfterStreamError { index: usize },
    #[error("event {index} appeared after Completed")]
    EventAfterCompleted { index: usize },
    #[error("text delta at event {index} arrived without an active output item")]
    DeltaWithoutActiveItem { index: usize },
    #[error("{kind} delta at event {index} arrived without an active output item")]
    AuxiliaryDeltaWithoutActiveItem { index: usize, kind: &'static str },
    #[error("output item was added at event {index} while another item was active")]
    ItemAddedWhileActive { index: usize },
    #[error("output item at event {index} has kind {actual:?}, expected active kind {expected:?}")]
    ItemKindMismatch {
        index: usize,
        expected: OutputItemKind,
        actual: OutputItemKind,
    },
    #[error("Completed at event {index} arrived while an output item was active")]
    CompletedWithActiveItem { index: usize },
    #[error("stream ended without Completed")]
    MissingCompleted,
    #[error("stream ended while an output item was still active")]
    ActiveItemAtEnd,
    #[error("successful stream did not produce assistant text")]
    MissingAssistantText,
    #[error("output message at event {index} has role {role:?}, expected assistant")]
    InvalidOutputMessageRole { index: usize, role: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputItemKind {
    Message,
    Tool,
    Reasoning,
    Other,
}

/// Validates a provider-neutral event sequence without making any network request.
pub fn validate_chat_events(
    events: impl IntoIterator<Item = ProviderResult<ProviderChatEvent>>,
    config: &ChatContractConfig,
) -> Result<ChatContractReport, ChatContractError> {
    let mut active_item = None;
    let mut assistant_text = String::new();
    let mut completed = false;
    let mut terminated_by_error = false;
    let mut active_item_had_text_delta = false;
    let mut event_count = 0;

    for (index, event) in events.into_iter().enumerate() {
        event_count = index + 1;
        if terminated_by_error {
            return Err(ChatContractError::EventAfterStreamError { index });
        }
        if completed {
            return Err(ChatContractError::EventAfterCompleted { index });
        }

        let event = match event {
            Ok(event) => event,
            Err(_error) if config.allow_stream_error => {
                terminated_by_error = true;
                continue;
            }
            Err(error) => return Err(ChatContractError::StreamError { index, error }),
        };

        match event {
            ProviderChatEvent::OutputItemAdded(item) => {
                if active_item.is_some() {
                    return Err(ChatContractError::ItemAddedWhileActive { index });
                }
                if let ResponseItem::Message { role, .. } = &item
                    && role != "assistant"
                {
                    return Err(ChatContractError::InvalidOutputMessageRole {
                        index,
                        role: role.clone(),
                    });
                }
                active_item = Some(output_item_kind(&item));
                active_item_had_text_delta = false;
            }
            ProviderChatEvent::OutputTextDelta(delta) => match active_item {
                Some(OutputItemKind::Message) => {
                    assistant_text.push_str(&delta);
                    active_item_had_text_delta = true;
                }
                Some(_) => {
                    return Err(ChatContractError::ItemKindMismatch {
                        index,
                        expected: OutputItemKind::Message,
                        actual: active_item.expect("active item was just matched"),
                    });
                }
                None => return Err(ChatContractError::DeltaWithoutActiveItem { index }),
            },
            ProviderChatEvent::ToolCallInputDelta { .. } => {
                require_active_auxiliary_item(
                    &active_item,
                    index,
                    "tool input",
                    OutputItemKind::Tool,
                )?;
            }
            ProviderChatEvent::ReasoningSummaryDelta { .. }
            | ProviderChatEvent::ReasoningSummaryDone { .. }
            | ProviderChatEvent::ReasoningContentDelta { .. }
            | ProviderChatEvent::ReasoningSummaryPartAdded { .. } => {
                require_active_auxiliary_item(
                    &active_item,
                    index,
                    "reasoning",
                    OutputItemKind::Reasoning,
                )?;
            }
            ProviderChatEvent::OutputItemDone(item) => {
                if let ResponseItem::Message { role, .. } = &item
                    && role != "assistant"
                {
                    return Err(ChatContractError::InvalidOutputMessageRole {
                        index,
                        role: role.clone(),
                    });
                }
                let actual = output_item_kind(&item);
                if let Some(expected) = active_item.take() {
                    if expected != actual {
                        return Err(ChatContractError::ItemKindMismatch {
                            index,
                            expected,
                            actual,
                        });
                    }
                    if actual == OutputItemKind::Message && !active_item_had_text_delta {
                        if let ResponseItem::Message { content, .. } = item {
                            append_output_text(&mut assistant_text, content);
                        }
                    }
                    active_item_had_text_delta = false;
                } else if let ResponseItem::Message { content, .. } = item {
                    append_output_text(&mut assistant_text, content);
                }
            }
            ProviderChatEvent::Completed { .. } => {
                if active_item.is_some() {
                    return Err(ChatContractError::CompletedWithActiveItem { index });
                }
                completed = true;
            }
            ProviderChatEvent::Created { .. }
            | ProviderChatEvent::SafetyBuffering(_)
            | ProviderChatEvent::ServerModel(_)
            | ProviderChatEvent::ModelVerifications(_)
            | ProviderChatEvent::ServerReasoningIncluded(_)
            | ProviderChatEvent::RateLimits(_)
            | ProviderChatEvent::ModelsEtag(_) => {}
        }
    }

    if active_item.is_some() && !terminated_by_error {
        return Err(ChatContractError::ActiveItemAtEnd);
    }
    if config.require_completed && !completed && !terminated_by_error {
        return Err(ChatContractError::MissingCompleted);
    }
    if config.require_text && completed && assistant_text.is_empty() {
        return Err(ChatContractError::MissingAssistantText);
    }

    Ok(ChatContractReport {
        event_count,
        assistant_text,
        completed,
        terminated_by_error,
    })
}

fn append_output_text(assistant_text: &mut String, content: Vec<ContentItem>) {
    for content in content {
        if let ContentItem::OutputText { text } = content {
            assistant_text.push_str(&text);
        }
    }
}

fn require_active_auxiliary_item(
    active_item: &Option<OutputItemKind>,
    index: usize,
    kind: &'static str,
    expected: OutputItemKind,
) -> Result<(), ChatContractError> {
    match active_item {
        None => Err(ChatContractError::AuxiliaryDeltaWithoutActiveItem { index, kind }),
        Some(actual) if *actual != expected => Err(ChatContractError::ItemKindMismatch {
            index,
            expected,
            actual: *actual,
        }),
        Some(_) => Ok(()),
    }
}

fn output_item_kind(item: &ResponseItem) -> OutputItemKind {
    match item {
        ResponseItem::Message { .. } => OutputItemKind::Message,
        ResponseItem::FunctionCall { .. }
        | ResponseItem::CustomToolCall { .. }
        | ResponseItem::LocalShellCall { .. }
        | ResponseItem::ToolSearchCall { .. }
        | ResponseItem::WebSearchCall { .. }
        | ResponseItem::ImageGenerationCall { .. } => OutputItemKind::Tool,
        ResponseItem::Reasoning { .. } => OutputItemKind::Reasoning,
        _ => OutputItemKind::Other,
    }
}

/// Calls a real provider adapter and validates the returned stream with the offline validator.
pub async fn assert_provider_chat_contract<P: ModelProvider + ?Sized>(
    provider: &P,
    request: ChatRequest,
    config: &ChatContractConfig,
) -> Result<ChatContractReport, ChatContractError> {
    let events = match tokio::time::timeout(config.live_timeout, async {
        let stream: ProviderChatStream = provider.chat(request).await?;
        Ok::<_, ProviderError>(stream.collect::<Vec<_>>().await)
    })
    .await
    {
        Ok(Ok(events)) => events,
        Ok(Err(error)) => return Err(ChatContractError::ChatStart { error }),
        Err(_) => {
            return Err(ChatContractError::TimedOut {
                timeout: config.live_timeout,
            });
        }
    };
    validate_chat_events(events, config)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use ruxex_model_provider::{
        ChatRequest, ChatTransportOptions, ModelProvider, ProviderCapabilities, ProviderChatEvent,
        ProviderChatStream, ProviderDescriptor, ProviderError, ProviderFuture,
        ProviderRequestCompression, ProviderResult,
    };
    use ruxex_protocol::account::ProviderAuthStatus;
    use ruxex_protocol::models::{ContentItem, ResponseItem};

    use super::{
        ChatContractConfig, ChatContractError, assert_provider_chat_contract, validate_chat_events,
    };

    fn empty_request() -> ChatRequest {
        ChatRequest {
            model: "test-model".to_string(),
            instructions: String::new(),
            input: Vec::new(),
            tools: Vec::new(),
            metadata: BTreeMap::new(),
            conversation_id: None,
            session_source: None,
            tool_choice: "auto".to_string(),
            parallel_tool_calls: true,
            reasoning: None,
            store: false,
            stream: true,
            stream_options: None,
            include: Vec::new(),
            service_tier: None,
            prompt_cache_key: None,
            text: None,
            client_metadata: None,
            access_programs: None,
            transport: ChatTransportOptions {
                session_id: None,
                thread_id: None,
                extra_headers: BTreeMap::new(),
                compression: ProviderRequestCompression::None,
                turn_state: None,
                connection_key: None,
                credential_snapshot: None,
            },
        }
    }

    #[derive(Debug)]
    enum TestChatBehavior {
        Slow,
        Fail,
    }

    #[derive(Debug)]
    struct TestProvider {
        behavior: TestChatBehavior,
    }

    impl ModelProvider for TestProvider {
        fn descriptor(&self) -> ProviderDescriptor {
            ProviderDescriptor::new("test", "1")
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn chat(
            &self,
            _request: ChatRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>> {
            match self.behavior {
                TestChatBehavior::Slow => Box::pin(async {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    Err(ProviderError::Transport("test timeout".to_string()))
                }),
                TestChatBehavior::Fail => {
                    Box::pin(async { Err(ProviderError::Transport("test chat start".to_string())) })
                }
            }
        }

        fn list_models(
            &self,
            _request: ruxex_model_provider::ListModelsRequest,
        ) -> ProviderFuture<'_, ProviderResult<ruxex_model_provider::ProviderModelCatalog>>
        {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn auth_state(&self) -> ruxex_model_provider::ProviderAuthState {
            ruxex_model_provider::ProviderAuthState {
                status: ProviderAuthStatus::Unauthenticated,
                account_id: None,
                display_name: None,
                managed_auth: false,
                revision: None,
            }
        }

        fn request_snapshot(
            &self,
        ) -> ProviderFuture<'_, ProviderResult<ruxex_model_provider::ProviderRequestSnapshot>>
        {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn account_state(&self) -> ProviderResult<ruxex_model_provider::ProviderAccountState> {
            Err(ProviderError::Unsupported)
        }

        fn login(
            &self,
            _credentials: BTreeMap<String, String>,
        ) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn usage(
            &self,
            _request: ruxex_model_provider::UsageRequest,
        ) -> ProviderFuture<'_, ProviderResult<ruxex_model_provider::ProviderUsage>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }
    }

    fn assistant_item() -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "assistant".to_string(),
            content: Vec::new(),
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        }
    }

    fn tool_item() -> ResponseItem {
        ResponseItem::FunctionCall {
            id: None,
            name: "example_tool".to_string(),
            namespace: None,
            arguments: "{}".to_string(),
            encrypted_function_args: None,
            call_id: "call-1".to_string(),
            internal_chat_message_metadata_passthrough: None,
        }
    }

    fn valid_text_events() -> Vec<ProviderResult<ProviderChatEvent>> {
        vec![
            Ok(ProviderChatEvent::Created {
                response_id: Some("response-1".to_string()),
            }),
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::OutputTextDelta("hello".to_string())),
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "hello".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            })),
            Ok(ProviderChatEvent::Completed {
                response_id: "response-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            }),
        ]
    }

    #[test]
    fn accepts_a_complete_text_lifecycle() {
        let report = validate_chat_events(valid_text_events(), &ChatContractConfig::default())
            .expect("valid text lifecycle should satisfy the contract");

        assert_eq!(report.assistant_text, "hello");
        assert!(report.completed);
    }

    #[test]
    fn rejects_text_delta_without_an_active_item() {
        let events = vec![Ok(ProviderChatEvent::OutputTextDelta("hello".to_string()))];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("delta before item should fail");

        assert!(matches!(
            error,
            ChatContractError::DeltaWithoutActiveItem { index: 0, .. }
        ));
    }

    #[test]
    fn accepts_a_standalone_non_streamed_tool_item() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemDone(
                ResponseItem::FunctionCall {
                    id: None,
                    name: "example_tool".to_string(),
                    namespace: None,
                    arguments: "{}".to_string(),
                    encrypted_function_args: None,
                    call_id: "call-1".to_string(),
                    internal_chat_message_metadata_passthrough: None,
                },
            )),
            Ok(ProviderChatEvent::Completed {
                response_id: "response-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(false),
            }),
        ];

        validate_chat_events(
            events,
            &ChatContractConfig::default().without_required_text(),
        )
        .expect("standalone non-streamed tool item should satisfy the contract");
    }

    #[test]
    fn accepts_message_content_on_done_without_text_deltas() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "hello".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            })),
            Ok(ProviderChatEvent::Completed {
                response_id: "response-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            }),
        ];

        let report = validate_chat_events(events, &ChatContractConfig::default())
            .expect("done message content should satisfy the text requirement");

        assert_eq!(report.assistant_text, "hello");
    }

    #[test]
    fn rejects_completion_with_an_active_item() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::Completed {
                response_id: "response-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            }),
        ];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("completion with active item should fail");

        assert!(matches!(
            error,
            ChatContractError::CompletedWithActiveItem { index: 1 }
        ));
    }

    #[test]
    fn allows_a_configured_partial_stream_error() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::OutputTextDelta("partial".to_string())),
            Err(ProviderError::Transport("connection closed".to_string())),
        ];

        let config = ChatContractConfig::default().allow_stream_error();
        let report = validate_chat_events(events, &config)
            .expect("configured partial stream errors should be accepted");

        assert_eq!(report.assistant_text, "partial");
        assert!(!report.completed);
        assert!(report.terminated_by_error);
    }

    #[test]
    fn rejects_events_after_a_stream_error() {
        let events = vec![
            Err(ProviderError::Transport("connection closed".to_string())),
            Ok(ProviderChatEvent::Completed {
                response_id: "response-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            }),
        ];

        let error =
            validate_chat_events(events, &ChatContractConfig::default().allow_stream_error())
                .expect_err("events after a stream error should fail");

        assert!(matches!(
            error,
            ChatContractError::EventAfterStreamError { index: 1 }
        ));
    }

    #[test]
    fn rejects_a_successful_response_without_text_when_text_is_required() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::OutputItemDone(assistant_item())),
            Ok(ProviderChatEvent::Completed {
                response_id: "response-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            }),
        ];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("successful response without text should fail");

        assert!(matches!(error, ChatContractError::MissingAssistantText));
    }

    #[test]
    fn rejects_an_output_item_added_while_another_item_is_active() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
        ];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("nested output items should fail");

        assert!(matches!(
            error,
            ChatContractError::ItemAddedWhileActive { index: 1 }
        ));
    }

    #[test]
    fn rejects_an_output_item_kind_mismatch() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::OutputItemDone(tool_item())),
        ];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("an item kind change should fail");

        assert!(matches!(
            error,
            ChatContractError::ItemKindMismatch {
                index: 1,
                expected: super::OutputItemKind::Message,
                actual: super::OutputItemKind::Tool,
            }
        ));
    }

    #[test]
    fn rejects_auxiliary_deltas_without_the_matching_active_item() {
        let events = vec![
            Ok(ProviderChatEvent::OutputItemAdded(assistant_item())),
            Ok(ProviderChatEvent::ToolCallInputDelta {
                item_id: "call-1".to_string(),
                call_id: Some("call-1".to_string()),
                delta: "{}".to_string(),
            }),
        ];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("tool deltas on message items should fail");

        assert!(matches!(
            error,
            ChatContractError::ItemKindMismatch {
                index: 1,
                expected: super::OutputItemKind::Tool,
                actual: super::OutputItemKind::Message,
            }
        ));
    }

    #[test]
    fn rejects_auxiliary_deltas_without_any_active_item() {
        let events = vec![Ok(ProviderChatEvent::ToolCallInputDelta {
            item_id: "call-1".to_string(),
            call_id: Some("call-1".to_string()),
            delta: "{}".to_string(),
        })];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("tool deltas without an item should fail");

        assert!(matches!(
            error,
            ChatContractError::AuxiliaryDeltaWithoutActiveItem { index: 0, .. }
        ));
    }

    #[test]
    fn rejects_an_invalid_role_on_output_item_done() {
        let invalid_item = ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: Vec::new(),
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        };
        let events = vec![Ok(ProviderChatEvent::OutputItemDone(invalid_item))];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("non-assistant output messages should fail");

        assert!(matches!(
            error,
            ChatContractError::InvalidOutputMessageRole { index: 0, .. }
        ));
    }

    #[test]
    fn rejects_events_after_completed() {
        let events = vec![
            Ok(ProviderChatEvent::Completed {
                response_id: "response-1".to_string(),
                token_usage: None,
                usage_metadata: None,
                end_turn: Some(true),
            }),
            Ok(ProviderChatEvent::Created { response_id: None }),
        ];

        let error = validate_chat_events(
            events,
            &ChatContractConfig::default().without_required_text(),
        )
        .expect_err("events after completion should fail");

        assert!(matches!(
            error,
            ChatContractError::EventAfterCompleted { index: 1 }
        ));
    }

    #[test]
    fn rejects_a_stream_that_ends_without_completed() {
        let events = vec![Ok(ProviderChatEvent::Created { response_id: None })];

        let error = validate_chat_events(events, &ChatContractConfig::default())
            .expect_err("a stream without completion should fail");

        assert!(matches!(error, ChatContractError::MissingCompleted));
    }

    #[test]
    fn rejects_a_stream_that_ends_with_an_active_item() {
        let events = vec![Ok(ProviderChatEvent::OutputItemAdded(assistant_item()))];

        let error = validate_chat_events(
            events,
            &ChatContractConfig::default().without_required_text(),
        )
        .expect_err("an active item at stream end should fail");

        assert!(matches!(error, ChatContractError::ActiveItemAtEnd));
    }

    #[tokio::test]
    async fn live_runner_times_out_while_starting_chat() {
        let error = assert_provider_chat_contract(
            &TestProvider {
                behavior: TestChatBehavior::Slow,
            },
            empty_request(),
            &ChatContractConfig::default().with_live_timeout(Duration::from_millis(1)),
        )
        .await
        .expect_err("a provider that hangs during chat setup should time out");

        assert!(matches!(error, ChatContractError::TimedOut { .. }));
    }

    #[tokio::test]
    async fn live_runner_reports_chat_start_errors() {
        let error = assert_provider_chat_contract(
            &TestProvider {
                behavior: TestChatBehavior::Fail,
            },
            empty_request(),
            &ChatContractConfig::default(),
        )
        .await
        .expect_err("chat setup errors should be reported");

        assert!(matches!(error, ChatContractError::ChatStart { .. }));
    }
}
