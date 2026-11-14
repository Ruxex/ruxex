use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::stream;
use ruxex::provider::auth::{CredentialManager, ProviderAuth};
use ruxex::provider::protocol::{
    ModelCatalog, ModelInfo, ProviderAccount, ProviderAuthStatus, ResponseItem,
    ResponseUsageMetadata, TokenUsage,
};
use ruxex::provider::{
    ChatRequest, ListModelsRequest, ModelProvider, ModelProviderFactory, ProviderAccountState,
    ProviderAuthEnvironment, ProviderCapabilities, ProviderChatEvent, ProviderConnectionKey,
    ProviderCredentialSnapshot, ProviderDescriptor, ProviderError, ProviderFuture, ProviderInit,
    ProviderModelCatalog, ProviderRequestAuth, ProviderRequestSnapshot, ProviderResult,
    ProviderUsage, UsageRequest,
};
use ruxex_app_server::{AppServerRuntime, AppServerRuntimeBuilder};
use ruxex_arg0::Arg0DispatchPaths;
use ruxex_cli::APP_SERVER_DAEMON_MODE_ARG;
use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use ruxex_protocol::models::ContentItem;

pub const DUMMY_PROVIDER_ID: &str = "dummy";
pub const FIXED_CHAT_ANSWER: &str = "This is a fixed response from the dummy provider.";
const DUMMY_MODEL_CATALOG_IDENTITY: &str = "dummy:fixed-v1";
const DUMMY_HOME_ENV: &str = "RUXEX_DUMMY_HOME";
const FIXED_TOOL_ANSWER: &str =
    "The example tool result was received. This is a fixed tool integration response.";
const FIXED_COMMAND_ANSWER: &str =
    "The command result was received. This is a fixed command integration response.";
const FIXED_APPROVAL_ANSWER: &str = "The approved command result was received.";
const FIXED_APPROVAL_DENIED_ANSWER: &str = "The approval was denied by the client.";
const FIXED_APPROVAL_CANCELLED_ANSWER: &str = "The approval was cancelled by the client.";
const FIXED_USER_INPUT_ANSWER: &str = "The fixed user input was received.";
const FIXED_PERMISSIONS_ANSWER: &str = "The fixed permissions response was received.";
const FIXED_GOAL_SET_ANSWER: &str = "The fixture review goal was set.";
const FIXED_GOAL_COMPLETED_ANSWER: &str = "The fixture review goal was completed.";
const FIXED_PLAN_UNAVAILABLE: &str = "The update_plan tool is unavailable in this request.";
const FIXED_PLAN_ANSWER: &str = "The fixed plan fixture is complete.";
const FIXED_REASONING_SUMMARY: &str =
    "I will keep this fixture response concise and show the summary separately.";
const FIXED_REASONING_ANSWER: &str = "This response follows the fixed reasoning summary.";
const FIXED_SUBAGENT_UNAVAILABLE: &str = "The spawn_agent tool is unavailable in this request.";
const FIXED_SUBAGENT_ANSWER: &str = "The Example subagent fixture is complete.";
const FIXED_PARALLEL_SUBAGENT_ANSWER: &str = "The Example parallel subagent fixture is complete.";
const FIXED_MCP_ANSWER: &str =
    "The MCP tool result was received. This is a fixed MCP integration response.";
const FIXED_MCP_UNAVAILABLE: &str = "The Example Provider MCP tool is unavailable.";
const FIXED_MARKDOWN_ANSWER: &str = r###"# Markdown rendering fixture

This response is a **bold** and *italic* visual check with ~~strikethrough~~.
It also includes `inline code` and a [Ruxex](https://example.com) link.

## Lists and quote

1. Ordered item
2. Another ordered item

- Unordered item
- Another unordered item

- [x] Completed task
- [ ] Pending task

> A blockquote for checking indentation and text color.

## Table

| Feature | Rendering |
| --- | --- |
| Table | GFM |
| Mermaid | Diagram |

---

## Code blocks

```rust
fn fixed_fixture() -> &'static str {
    "Markdown and Mermaid"
}
```

```diff
- old fixture
+ new fixture
  unchanged line
```

## Mermaid

```mermaid
graph TD
  Start[Start] --> Markdown[Markdown]
  Markdown --> Mermaid[Mermaid]
```
"###;
const FIXED_COMMAND: &str = "printf 'example-provider command output\\n'";
const FIXED_APPROVAL_COMMAND: &str = "printf 'approval-required command output\\n'";
const FIXED_APPROVAL_JUSTIFICATION: &str = "Run the example provider approval command.";
const LEGACY_DEFAULT_DUMMY_CONFIG: &str =
    "approval_policy = \"never\"\nsandbox_mode = \"read-only\"\n";
const DEFAULT_DUMMY_CONFIG: &str = "approval_policy = \"on-request\"\nsandbox_mode = \"read-only\"\n\n[features]\nrequest_permissions_tool = true\n";
const FIXED_WORKFLOW_START: &str = "Starting the sequential workflow.";
const FIXED_WORKFLOW_AFTER_TOOL: &str = "The first tool completed; running the command step.";
const FIXED_WORKFLOW_ANSWER: &str = "The sequential workflow completed.";
const FIXED_PARALLEL_ANSWER: &str = "The parallel tools completed.";
const FIXED_LONG_STREAM_ANSWER: &str = r#"# Long stream fixture

This is a deterministic long response for client rendering tests. It contains
enough text to exercise incremental layout, scrolling, and message grouping.

## Details

- The response is split into bounded text deltas.
- The content is fixed across every run.
- Slow mode changes timing, not content.

```rust
fn fixed_fixture_step(index: usize) -> usize {
    index.saturating_add(1)
}
```

The final paragraph confirms that the long stream reached completion."#;
const FIXED_LS_COMMAND: &str = "ls";
const FIXED_SLEEP_COMMAND: &str =
    "echo 'example-provider before sleep'; sleep 3; echo 'example-provider after sleep'";
const FIXED_LARGE_COMMAND: &str =
    "for i in $(seq 1 120); do printf 'example-provider output line %03d\\n' \"$i\"; done";
const FIXED_FAIL_COMMAND: &str = "printf 'example-provider stderr\\n' >&2; exit 7";
const FIXED_TIMEOUT_COMMAND: &str = "sleep 5";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DummyScenario {
    Basic,
    Tool,
    Command,
    Approval,
    ApprovalDeny,
    ApprovalCancel,
    Workflow,
    ParallelTools,
    LongStream,
    UserInput,
    Permissions,
    Goal,
    Mcp,
    Plan,
    Reasoning,
    Markdown,
    Subagent,
    ParallelSubagents,
    Multi,
    Error,
    RateLimit,
    Disconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DummyCommandVariant {
    Fixed,
    List,
    Sleep,
    Large,
    Fail,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScenarioSelection {
    scenario: DummyScenario,
    command_variant: DummyCommandVariant,
    slow: bool,
    stream: bool,
    multi_turn: usize,
    continuation: bool,
    step: usize,
}

#[derive(Clone, Debug)]
pub struct DummyProvider {
    credential_manager: Arc<CredentialManager>,
}

#[derive(Debug)]
struct DummyCredentialSnapshot;

impl ProviderCredentialSnapshot for DummyCredentialSnapshot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ModelProviderFactory for DummyProvider {
    fn provider_descriptor() -> ProviderDescriptor {
        ProviderDescriptor::new(DUMMY_PROVIDER_ID, CURRENT_PROVIDER_PROTOCOL_VERSION)
    }

    fn new(init: ProviderInit) -> Self {
        Self {
            credential_manager: init.credential_manager,
        }
    }
}

impl ModelProvider for DummyProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        Self::provider_descriptor()
    }

    fn model_catalog_identity(&self) -> ProviderResult<String> {
        Ok(DUMMY_MODEL_CATALOG_IDENTITY.to_string())
    }

    fn request_format(&self) -> ruxex::provider::ProviderRequestFormat {
        ruxex::provider::ProviderRequestFormat::Chat
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            api_key_model_discovery: true,
            web_search: false,
            external_web_access: false,
            namespace_tools: true,
            ..ProviderCapabilities::default()
        }
    }

    fn model_catalog_auth(&self) -> ruxex::provider::ProviderModelCatalogAuth {
        ruxex::provider::ProviderModelCatalogAuth {
            has_api_key_auth: true,
            ..Default::default()
        }
    }

    fn chat(
        &self,
        request: ChatRequest,
    ) -> ProviderFuture<'_, ProviderResult<ruxex::provider::ProviderChatStream>> {
        Box::pin(async move {
            if !self.is_authenticated() {
                return Err(ProviderError::Unauthenticated);
            }

            let selection = select_scenario(&request.input)?;
            match selection.scenario {
                DummyScenario::Error => Err(ProviderError::Transient(
                    "example provider transient error".to_string(),
                )),
                DummyScenario::RateLimit => Err(ProviderError::RateLimited {
                    retry_after_seconds: Some(3),
                }),
                DummyScenario::Mcp
                    if !selection.continuation && !has_example_mcp_tool(&request.tools) =>
                {
                    Ok(text_chat_stream(selection, FIXED_MCP_UNAVAILABLE))
                }
                DummyScenario::Plan => Ok(plan_scenario_chat_stream(
                    selection,
                    has_request_tool(&request.tools, "update_plan"),
                )),
                DummyScenario::Subagent | DummyScenario::ParallelSubagents => {
                    Ok(subagent_scenario_chat_stream(
                        selection,
                        spawn_agent_tool_form(&request.tools),
                        &request.input,
                    ))
                }
                _ => Ok(scenario_chat_stream(
                    selection,
                    has_example_mcp_tool(&request.tools),
                )),
            }
        })
    }

    fn list_models(
        &self,
        _request: ListModelsRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>> {
        Box::pin(async {
            Ok(ProviderModelCatalog {
                catalog: ModelCatalog {
                    models: fixed_models(),
                },
                identity: DUMMY_MODEL_CATALOG_IDENTITY.to_string(),
                etag: Some("dummy-fixed-v1".to_string()),
            })
        })
    }

    fn auth_state(&self) -> ruxex::provider::ProviderAuthState {
        let authenticated = self.is_authenticated();
        ruxex::provider::ProviderAuthState {
            status: if authenticated {
                ProviderAuthStatus::Authenticated
            } else {
                ProviderAuthStatus::Unauthenticated
            },
            account_id: authenticated.then(|| "dummy-account".to_string()),
            display_name: authenticated.then(|| "Dummy User".to_string()),
            managed_auth: false,
            revision: Some(*self.credential_manager.auth_change_receiver().borrow()),
        }
    }

    fn auth_environment(&self) -> ProviderAuthEnvironment {
        ProviderAuthEnvironment {
            host_api_key_env_present: std::env::var_os("RUXEX_API_KEY").is_some(),
            host_api_key_env_enabled: self.credential_manager.api_key_env_enabled(),
            ..ProviderAuthEnvironment::default()
        }
    }

    fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>> {
        Box::pin(async move {
            let revision = Some(*self.credential_manager.auth_change_receiver().borrow());
            Ok(ProviderRequestSnapshot {
                auth: ProviderRequestAuth {
                    authenticated: self.is_authenticated(),
                    managed_auth: false,
                    auth_header_attached: false,
                    auth_header_name: None,
                    revision,
                    owner_generation: None,
                },
                connection_key: ProviderConnectionKey {
                    provider_kind: DUMMY_PROVIDER_ID.to_string(),
                    auth_revision: revision,
                },
                credential_snapshot: Arc::new(DummyCredentialSnapshot),
            })
        })
    }

    fn account_state(&self) -> ProviderResult<ProviderAccountState> {
        let authenticated = self.is_authenticated();
        Ok(ProviderAccountState {
            provider_id: DUMMY_PROVIDER_ID.to_string(),
            account: authenticated.then_some(ProviderAccount::ApiKey),
            account_id: authenticated.then(|| "dummy-account".to_string()),
            display_name: authenticated.then(|| "Dummy User".to_string()),
            auth_status: if authenticated {
                ProviderAuthStatus::Authenticated
            } else {
                ProviderAuthStatus::Unauthenticated
            },
            usage: None,
        })
    }

    fn login(
        &self,
        mut credentials: BTreeMap<String, String>,
    ) -> ProviderFuture<'_, ProviderResult<()>> {
        let api_key = credentials.remove("api_key");
        Box::pin(async move {
            let api_key = validate_api_key(api_key.as_deref().unwrap_or_default())?;
            self.credential_manager
                .login(ProviderAuth::api_key(api_key))
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

impl DummyProvider {
    fn is_authenticated(&self) -> bool {
        self.credential_manager
            .auth_cached()
            .is_some_and(|auth| auth.api_key().is_some())
    }
}

fn validate_api_key(api_key: &str) -> ProviderResult<String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        Err(ProviderError::InvalidRequest(
            "api_key is required".to_string(),
        ))
    } else {
        Ok(api_key.to_string())
    }
}

fn fixed_models() -> Vec<ModelInfo> {
    ["A", "B", "C"]
        .into_iter()
        .map(|slug| {
            let mut model = ruxex_models_manager::model_info::model_info_template_from_slug(slug);
            model.visibility = ruxex::provider::protocol::ModelVisibility::List;
            model
        })
        .collect()
}

fn scenario_chat_stream(
    selection: ScenarioSelection,
    has_mcp_tool: bool,
) -> ruxex::provider::ProviderChatStream {
    let response_id = format!(
        "example-{}-response-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    let call_id = format!(
        "example-{}-call-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    let mut events = vec![
        Ok(ProviderChatEvent::Created {
            response_id: Some(response_id.clone()),
        }),
        Ok(ProviderChatEvent::ServerModel("A".to_string())),
    ];

    if matches!(
        selection.scenario,
        DummyScenario::ApprovalDeny
            | DummyScenario::ApprovalCancel
            | DummyScenario::Workflow
            | DummyScenario::ParallelTools
            | DummyScenario::UserInput
            | DummyScenario::Permissions
            | DummyScenario::Goal
    ) {
        return fixed_step_chat_stream(selection, events);
    }

    let final_text = match selection.scenario {
        DummyScenario::Basic => Some(FIXED_CHAT_ANSWER),
        DummyScenario::Tool if selection.continuation => Some(FIXED_TOOL_ANSWER),
        DummyScenario::Command if selection.continuation => Some(FIXED_COMMAND_ANSWER),
        DummyScenario::Approval if selection.continuation => Some(FIXED_APPROVAL_ANSWER),
        DummyScenario::Reasoning => Some(FIXED_REASONING_ANSWER),
        DummyScenario::Markdown => Some(FIXED_MARKDOWN_ANSWER),
        DummyScenario::LongStream => Some(FIXED_LONG_STREAM_ANSWER),
        DummyScenario::Mcp if selection.continuation => Some(FIXED_MCP_ANSWER),
        DummyScenario::Mcp if !has_mcp_tool => Some(FIXED_MCP_UNAVAILABLE),
        DummyScenario::Multi => Some(match selection.multi_turn {
            1 => "This is turn 1 of the multi-turn test.",
            2 => "This is turn 2 of the multi-turn test.",
            _ => "This is the completion turn of the multi-turn test.",
        }),
        DummyScenario::Disconnect => {
            events.push(Ok(ProviderChatEvent::OutputItemAdded(
                assistant_message_item(None),
            )));
            events.push(Ok(ProviderChatEvent::OutputTextDelta(
                "This response is intentionally interrupted.".to_string(),
            )));
            events.push(Err(ProviderError::Transport(
                "example provider disconnected after partial output".to_string(),
            )));
            return event_stream(events, selection.slow);
        }
        DummyScenario::Tool => {
            events.push(Ok(ProviderChatEvent::OutputItemDone(example_tool_call(
                call_id,
            ))));
            return complete_stream(events, response_id, false, selection.slow);
        }
        DummyScenario::Command => {
            events.push(Ok(ProviderChatEvent::OutputItemDone(example_command_call(
                call_id,
                selection.command_variant,
                selection.slow,
            ))));
            return complete_stream(events, response_id, false, selection.slow);
        }
        DummyScenario::Approval => {
            events.push(Ok(ProviderChatEvent::OutputItemDone(
                example_approval_command_call(call_id),
            )));
            return complete_stream(events, response_id, false, selection.slow);
        }
        DummyScenario::Mcp => {
            events.push(Ok(ProviderChatEvent::OutputItemDone(example_mcp_call(
                call_id,
            ))));
            return complete_stream(events, response_id, false, selection.slow);
        }
        DummyScenario::Plan => unreachable!("plan scenario uses a tool-aware stream"),
        DummyScenario::Error | DummyScenario::RateLimit => None,
        DummyScenario::ApprovalDeny
        | DummyScenario::ApprovalCancel
        | DummyScenario::Workflow
        | DummyScenario::ParallelTools
        | DummyScenario::Subagent
        | DummyScenario::ParallelSubagents
        | DummyScenario::UserInput
        | DummyScenario::Permissions
        | DummyScenario::Goal => {
            unreachable!("fixed-step scenarios return before the simple stream")
        }
    };

    if selection.scenario == DummyScenario::Reasoning {
        events.push(Ok(ProviderChatEvent::ReasoningSummaryDone {
            item_id: format!("example-reasoning-summary-{}", selection.multi_turn),
            text: FIXED_REASONING_SUMMARY.to_string(),
            summary_index: 0,
        }));
    }

    if let Some(text) = final_text {
        events.push(Ok(ProviderChatEvent::OutputItemAdded(
            assistant_message_item(None),
        )));
        let chunks = if selection.scenario == DummyScenario::LongStream {
            long_stream_text_chunks(text)
        } else if selection.stream {
            stream_text_chunks(text)
        } else {
            vec![text]
        };
        events.extend(
            chunks
                .into_iter()
                .map(|chunk| Ok(ProviderChatEvent::OutputTextDelta(chunk.to_string()))),
        );
        events.push(Ok(ProviderChatEvent::OutputItemDone(
            assistant_message_item(Some(text)),
        )));
    }

    complete_stream(events, response_id, true, selection.slow)
}

fn text_chat_stream(
    selection: ScenarioSelection,
    text: &str,
) -> ruxex::provider::ProviderChatStream {
    let response_id = format!(
        "example-{}-response-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    let mut events = vec![
        Ok(ProviderChatEvent::Created {
            response_id: Some(response_id.clone()),
        }),
        Ok(ProviderChatEvent::ServerModel("A".to_string())),
        Ok(ProviderChatEvent::OutputItemAdded(assistant_message_item(
            None,
        ))),
    ];
    let chunks = if selection.scenario == DummyScenario::LongStream {
        long_stream_text_chunks(text)
    } else if selection.stream {
        stream_text_chunks(text)
    } else {
        vec![text]
    };
    events.extend(
        chunks
            .into_iter()
            .map(|chunk| Ok(ProviderChatEvent::OutputTextDelta(chunk.to_string()))),
    );
    events.push(Ok(ProviderChatEvent::OutputItemDone(
        assistant_message_item(Some(text)),
    )));
    complete_stream(events, response_id, true, selection.slow)
}

fn plan_scenario_chat_stream(
    selection: ScenarioSelection,
    has_update_plan_tool: bool,
) -> ruxex::provider::ProviderChatStream {
    if selection.step == 0 && !has_update_plan_tool {
        return text_chat_stream(selection, FIXED_PLAN_UNAVAILABLE);
    }

    let response_id = format!(
        "example-{}-response-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    let mut events = vec![
        Ok(ProviderChatEvent::Created {
            response_id: Some(response_id.clone()),
        }),
        Ok(ProviderChatEvent::ServerModel("A".to_string())),
    ];

    if selection.step == 0 {
        events.push(Ok(ProviderChatEvent::OutputItemDone(
            example_update_plan_tool_call("example-plan-call-1"),
        )));
        complete_stream(events, response_id, false, selection.slow)
    } else {
        text_chat_stream_with_events(
            events,
            response_id,
            FIXED_PLAN_ANSWER,
            selection.stream,
            selection.slow,
        )
    }
}

fn example_update_plan_tool_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "update_plan".to_string(),
        namespace: None,
        arguments: serde_json::json!({
            "plan": [
                {"step": "Inspect the Example provider activity", "status": "in_progress"},
                {"step": "Review the rendered fixture", "status": "pending"}
            ]
        })
        .to_string(),
        encrypted_function_args: None,
        call_id: call_id.to_string(),
        internal_chat_message_metadata_passthrough: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpawnAgentToolForm {
    V1,
    V2,
}

fn subagent_scenario_chat_stream(
    selection: ScenarioSelection,
    tool_form: Option<SpawnAgentToolForm>,
    input: &[ResponseItem],
) -> ruxex::provider::ProviderChatStream {
    let response_id = format!(
        "example-{}-response-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    let calls = match (selection.scenario, selection.step) {
        (DummyScenario::Subagent, 0) => vec![(
            "example-subagent-call-1",
            "example_worker",
            "Reply in one short sentence confirming that the Example fixture can spawn an agent. Do not call tools.",
        )],
        (DummyScenario::Subagent, _) => {
            return text_chat_stream(selection, FIXED_SUBAGENT_ANSWER);
        }
        (DummyScenario::ParallelSubagents, 0) => vec![
            (
                "example-parallel-subagent-call-1",
                "example_worker",
                "Reply in one short sentence confirming that the Example fixture can launch a worker. Do not call tools.",
            ),
            (
                "example-parallel-subagent-call-2",
                "example_reviewer",
                "Reply in one short sentence confirming that the Example fixture can launch a reviewer. Do not call tools.",
            ),
        ],
        (DummyScenario::ParallelSubagents, 1) => {
            let mut missing = Vec::new();
            if !has_function_call_output(input, "example-parallel-subagent-call-1") {
                missing.push((
                    "example-parallel-subagent-call-1",
                    "example_worker",
                    "Reply in one short sentence confirming that the Example fixture can launch a worker. Do not call tools.",
                ));
            }
            if !has_function_call_output(input, "example-parallel-subagent-call-2") {
                missing.push((
                    "example-parallel-subagent-call-2",
                    "example_reviewer",
                    "Reply in one short sentence confirming that the Example fixture can launch a reviewer. Do not call tools.",
                ));
            }
            missing
        }
        (DummyScenario::ParallelSubagents, _) => {
            return text_chat_stream(selection, FIXED_PARALLEL_SUBAGENT_ANSWER);
        }
        _ => unreachable!("subagent stream requires a subagent scenario"),
    };

    let Some(tool_form) = tool_form else {
        return text_chat_stream(selection, FIXED_SUBAGENT_UNAVAILABLE);
    };

    let mut events = vec![
        Ok(ProviderChatEvent::Created {
            response_id: Some(response_id.clone()),
        }),
        Ok(ProviderChatEvent::ServerModel("A".to_string())),
    ];
    events.extend(calls.into_iter().map(|(call_id, task_name, message)| {
        Ok(ProviderChatEvent::OutputItemDone(example_spawn_agent_call(
            call_id, task_name, message, tool_form,
        )))
    }));
    complete_stream(events, response_id, false, selection.slow)
}

fn example_spawn_agent_call(
    call_id: &str,
    task_name: &str,
    message: &str,
    tool_form: SpawnAgentToolForm,
) -> ResponseItem {
    let (namespace, arguments) = match tool_form {
        SpawnAgentToolForm::V1 => (
            Some("multi_agent_v1".to_string()),
            serde_json::json!({"message": message}).to_string(),
        ),
        SpawnAgentToolForm::V2 => (
            None,
            serde_json::json!({"task_name": task_name, "message": message}).to_string(),
        ),
    };
    ResponseItem::FunctionCall {
        id: None,
        name: "spawn_agent".to_string(),
        namespace,
        arguments,
        encrypted_function_args: None,
        call_id: call_id.to_string(),
        internal_chat_message_metadata_passthrough: None,
    }
}

fn fixed_step_chat_stream(
    selection: ScenarioSelection,
    mut events: Vec<ProviderResult<ProviderChatEvent>>,
) -> ruxex::provider::ProviderChatStream {
    let response_id = format!(
        "example-{}-response-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    match selection.scenario {
        DummyScenario::ApprovalDeny => {
            let answer = if selection.step == 1 {
                FIXED_APPROVAL_DENIED_ANSWER
            } else {
                FIXED_APPROVAL_ANSWER
            };
            approval_outcome_chat_stream(selection, events, answer)
        }
        DummyScenario::ApprovalCancel => {
            approval_outcome_chat_stream(selection, events, FIXED_APPROVAL_CANCELLED_ANSWER)
        }
        DummyScenario::Workflow => match selection.step {
            0 => {
                push_assistant_text(&mut events, FIXED_WORKFLOW_START);
                events.push(Ok(ProviderChatEvent::OutputItemDone(
                    example_tool_call_with_id("example-workflow-tool-call-1"),
                )));
                complete_stream(events, response_id, false, selection.slow)
            }
            1 => {
                push_assistant_text(&mut events, FIXED_WORKFLOW_AFTER_TOOL);
                events.push(Ok(ProviderChatEvent::OutputItemDone(
                    example_command_call_with_id("example-workflow-command-call-1"),
                )));
                complete_stream(events, response_id, false, selection.slow)
            }
            _ => text_chat_stream_with_events(
                events,
                response_id,
                FIXED_WORKFLOW_ANSWER,
                selection.stream,
                selection.slow,
            ),
        },
        DummyScenario::ParallelTools => {
            if selection.step == 0 {
                events.push(Ok(ProviderChatEvent::OutputItemDone(
                    example_tool_call_with_id("example-parallel-tool-call-1"),
                )));
                events.push(Ok(ProviderChatEvent::OutputItemDone(
                    example_command_call_with_id("example-parallel-command-call-1"),
                )));
                complete_stream(events, response_id, false, selection.slow)
            } else {
                text_chat_stream_with_events(
                    events,
                    response_id,
                    FIXED_PARALLEL_ANSWER,
                    selection.stream,
                    selection.slow,
                )
            }
        }
        DummyScenario::UserInput => interaction_chat_stream(
            selection,
            events,
            example_user_input_call,
            FIXED_USER_INPUT_ANSWER,
        ),
        DummyScenario::Permissions => interaction_chat_stream(
            selection,
            events,
            example_permissions_call,
            FIXED_PERMISSIONS_ANSWER,
        ),
        DummyScenario::Goal => {
            if matches!(selection.step, 0 | 2) {
                let call = if selection.step == 0 {
                    example_create_goal_call("example-goal-create-call-1")
                } else {
                    example_update_goal_call("example-goal-update-call-1")
                };
                events.push(Ok(ProviderChatEvent::OutputItemDone(call)));
                complete_stream(events, response_id, false, selection.slow)
            } else {
                let answer = if selection.step == 1 {
                    FIXED_GOAL_SET_ANSWER
                } else {
                    FIXED_GOAL_COMPLETED_ANSWER
                };
                text_chat_stream_with_events(
                    events,
                    response_id,
                    answer,
                    selection.stream,
                    selection.slow,
                )
            }
        }
        _ => unreachable!("fixed step stream called for a simple scenario"),
    }
}

fn approval_outcome_chat_stream(
    selection: ScenarioSelection,
    mut events: Vec<ProviderResult<ProviderChatEvent>>,
    answer: &str,
) -> ruxex::provider::ProviderChatStream {
    let response_id = format!(
        "example-{}-response-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    if selection.step == 0 {
        let call_id = format!("example-{}-call-1", selection.scenario.as_str());
        events.push(Ok(ProviderChatEvent::OutputItemDone(
            example_approval_command_call(call_id),
        )));
        complete_stream(events, response_id, false, selection.slow)
    } else {
        text_chat_stream_with_events(
            events,
            response_id,
            answer,
            selection.stream,
            selection.slow,
        )
    }
}

fn interaction_chat_stream(
    selection: ScenarioSelection,
    mut events: Vec<ProviderResult<ProviderChatEvent>>,
    call_builder: fn(String) -> ResponseItem,
    answer: &str,
) -> ruxex::provider::ProviderChatStream {
    let response_id = format!(
        "example-{}-response-{}",
        selection.scenario.as_str(),
        selection.multi_turn
    );
    if selection.step == 0 {
        let call_id = format!("example-{}-call-1", selection.scenario.as_str());
        events.push(Ok(ProviderChatEvent::OutputItemDone(call_builder(call_id))));
        complete_stream(events, response_id, false, selection.slow)
    } else {
        text_chat_stream_with_events(
            events,
            response_id,
            answer,
            selection.stream,
            selection.slow,
        )
    }
}

fn push_assistant_text(events: &mut Vec<ProviderResult<ProviderChatEvent>>, text: &str) {
    events.push(Ok(ProviderChatEvent::OutputItemAdded(
        assistant_message_item(None),
    )));
    events.push(Ok(ProviderChatEvent::OutputTextDelta(text.to_string())));
    events.push(Ok(ProviderChatEvent::OutputItemDone(
        assistant_message_item(Some(text)),
    )));
}

fn text_chat_stream_with_events(
    mut events: Vec<ProviderResult<ProviderChatEvent>>,
    response_id: String,
    text: &str,
    stream: bool,
    slow: bool,
) -> ruxex::provider::ProviderChatStream {
    events.push(Ok(ProviderChatEvent::OutputItemAdded(
        assistant_message_item(None),
    )));
    let chunks = if stream {
        stream_text_chunks(text)
    } else {
        vec![text]
    };
    events.extend(
        chunks
            .into_iter()
            .map(|chunk| Ok(ProviderChatEvent::OutputTextDelta(chunk.to_string()))),
    );
    events.push(Ok(ProviderChatEvent::OutputItemDone(
        assistant_message_item(Some(text)),
    )));
    complete_stream(events, response_id, true, slow)
}

fn assistant_message_item(text: Option<&str>) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: text
            .map(|text| {
                vec![ContentItem::OutputText {
                    text: text.to_string(),
                }]
            })
            .unwrap_or_default(),
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn complete_stream(
    mut events: Vec<ProviderResult<ProviderChatEvent>>,
    response_id: String,
    end_turn: bool,
    slow: bool,
) -> ruxex::provider::ProviderChatStream {
    events.push(Ok(ProviderChatEvent::Completed {
        response_id,
        token_usage: Some(TokenUsage {
            input_tokens: 100,
            output_tokens: 20,
            total_tokens: 120,
            ..TokenUsage::default()
        }),
        usage_metadata: Some(ResponseUsageMetadata {
            amount: Some(0.001),
            metadata: None,
        }),
        end_turn: Some(end_turn),
    }));
    event_stream(events, slow)
}

fn event_stream(
    events: Vec<ProviderResult<ProviderChatEvent>>,
    slow: bool,
) -> ruxex::provider::ProviderChatStream {
    if !slow {
        return Box::pin(stream::iter(events));
    }

    Box::pin(stream::unfold(
        (events.into_iter(), true),
        |(mut events, first)| async move {
            let event = events.next()?;
            tokio::time::sleep(if first {
                Duration::from_secs(2)
            } else {
                Duration::from_millis(500)
            })
            .await;
            Some((event, (events, false)))
        },
    ))
}

fn example_tool_call(call_id: String) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "example_tool".to_string(),
        namespace: None,
        arguments: serde_json::json!({"input": "fixed-example-input"}).to_string(),
        encrypted_function_args: None,
        call_id,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn example_tool_call_with_id(call_id: &str) -> ResponseItem {
    example_tool_call(call_id.to_string())
}

fn example_command_call(
    call_id: String,
    command_variant: DummyCommandVariant,
    slow: bool,
) -> ResponseItem {
    let (command, timeout_ms) = match command_variant {
        DummyCommandVariant::Fixed => {
            if slow {
                (FIXED_SLEEP_COMMAND, None)
            } else {
                (FIXED_COMMAND, None)
            }
        }
        DummyCommandVariant::List => (FIXED_LS_COMMAND, None),
        DummyCommandVariant::Sleep => (FIXED_SLEEP_COMMAND, None),
        DummyCommandVariant::Large => (FIXED_LARGE_COMMAND, None),
        DummyCommandVariant::Fail => (FIXED_FAIL_COMMAND, None),
        DummyCommandVariant::Timeout => (FIXED_TIMEOUT_COMMAND, Some(1000)),
    };
    let mut arguments = serde_json::json!({"cmd": command});
    if let Some(timeout_ms) = timeout_ms {
        arguments["timeout_ms"] = serde_json::json!(timeout_ms);
    }
    ResponseItem::FunctionCall {
        id: None,
        name: "exec_command".to_string(),
        namespace: None,
        arguments: arguments.to_string(),
        encrypted_function_args: None,
        call_id,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn example_command_call_with_id(call_id: &str) -> ResponseItem {
    example_command_call(call_id.to_string(), DummyCommandVariant::Fixed, false)
}

fn example_approval_command_call(call_id: String) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "exec_command".to_string(),
        namespace: None,
        arguments: serde_json::json!({
            "cmd": FIXED_APPROVAL_COMMAND,
            "sandbox_permissions": "require_escalated",
            "justification": FIXED_APPROVAL_JUSTIFICATION,
        })
        .to_string(),
        encrypted_function_args: None,
        call_id,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn example_user_input_call(call_id: String) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "request_user_input".to_string(),
        namespace: None,
        arguments: serde_json::json!({
            "questions": [{
                "id": "confirm_fixture",
                "header": "Confirm",
                "question": "Proceed with the fixed UI test?",
                "options": [
                    {
                        "label": "Yes",
                        "description": "Continue the fixed fixture."
                    },
                    {
                        "label": "No",
                        "description": "Stop the fixed fixture."
                    }
                ]
            }]
        })
        .to_string(),
        encrypted_function_args: None,
        call_id,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn example_permissions_call(call_id: String) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "request_permissions".to_string(),
        namespace: None,
        arguments: serde_json::json!({
            "reason": "Select a workspace root",
            "permissions": {
                "file_system": {
                    "write": [".", "../shared"]
                }
            }
        })
        .to_string(),
        encrypted_function_args: None,
        call_id,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn example_create_goal_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "create_goal".to_string(),
        namespace: None,
        arguments: serde_json::json!({
            "objective": "Prepare the fixture review checklist."
        })
        .to_string(),
        encrypted_function_args: None,
        call_id: call_id.to_string(),
        internal_chat_message_metadata_passthrough: None,
    }
}

fn example_update_goal_call(call_id: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "update_goal".to_string(),
        namespace: None,
        arguments: serde_json::json!({"status": "complete"}).to_string(),
        encrypted_function_args: None,
        call_id: call_id.to_string(),
        internal_chat_message_metadata_passthrough: None,
    }
}

fn example_mcp_call(call_id: String) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: "echo".to_string(),
        namespace: Some("example".to_string()),
        arguments: serde_json::json!({"text": "fixed-mcp-input"}).to_string(),
        encrypted_function_args: None,
        call_id,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn stream_text_chunks(text: &str) -> Vec<&str> {
    match text {
        FIXED_CHAT_ANSWER => vec!["This is a ", "fixed response from the ", "dummy provider."],
        _ => vec![text],
    }
}

fn long_stream_text_chunks(text: &str) -> Vec<&str> {
    text.as_bytes()
        .chunks(32)
        .map(|chunk| std::str::from_utf8(chunk).expect("long stream fixture is ASCII"))
        .collect()
}

fn select_scenario(input: &[ResponseItem]) -> ProviderResult<ScenarioSelection> {
    if let Some(scenario) = continuation_scenario(input) {
        return Ok(ScenarioSelection {
            scenario,
            command_variant: DummyCommandVariant::Fixed,
            slow: false,
            stream: false,
            multi_turn: 1,
            continuation: true,
            step: scenario_step(input, scenario),
        });
    }

    let latest_user_text = latest_user_text(input).unwrap_or_default();
    if let Some(marker) = explicit_scenario(&latest_user_text) {
        let mut selection = marker?;
        if selection.scenario == DummyScenario::Multi {
            selection.multi_turn = multi_turn_count(input);
        }
        return Ok(selection);
    }

    if goal_follow_up(input) {
        return Ok(ScenarioSelection {
            scenario: DummyScenario::Goal,
            command_variant: DummyCommandVariant::Fixed,
            slow: false,
            stream: false,
            multi_turn: 1,
            continuation: false,
            step: 2,
        });
    }

    let lower = latest_user_text.to_ascii_lowercase();
    let scenario = if lower.contains("parallel-tools") {
        DummyScenario::ParallelTools
    } else if lower.contains("workflow") {
        DummyScenario::Workflow
    } else if lower.contains("long stream") || lower.contains("long-stream") {
        DummyScenario::LongStream
    } else if lower.contains("user input") || lower.contains("user-input") {
        DummyScenario::UserInput
    } else if lower.contains("permissions") {
        DummyScenario::Permissions
    } else if lower.contains("set it as a goal")
        || lower.contains("set a goal")
        || lower.contains("create a goal")
    {
        DummyScenario::Goal
    } else if lower.contains("approval") {
        DummyScenario::Approval
    } else if lower.contains("mcp") {
        DummyScenario::Mcp
    } else if lower.contains("large") || lower.contains("fail") || lower.contains("timeout") {
        DummyScenario::Command
    } else if lower.contains("command") || lower.contains("exec") {
        DummyScenario::Command
    } else if lower.contains("tool") {
        DummyScenario::Tool
    } else if lower.contains("multi") {
        DummyScenario::Multi
    } else if lower.contains("rate-limit") {
        DummyScenario::RateLimit
    } else if lower.contains("disconnect") {
        DummyScenario::Disconnect
    } else if lower.contains("error") {
        DummyScenario::Error
    } else if input.iter().any(|item| {
        user_message_text(item).is_some_and(|text| text.to_ascii_lowercase().contains("multi"))
    }) && multi_turn_count(input) <= 3
    {
        DummyScenario::Multi
    } else {
        DummyScenario::Basic
    };

    let multi_turn = (scenario == DummyScenario::Multi)
        .then(|| multi_turn_count(input))
        .unwrap_or(1);
    let slow = lower.contains("slow") || lower.contains("delay");
    let command_variant = if scenario == DummyScenario::Command {
        if lower.split_whitespace().any(|word| word == "ls") {
            DummyCommandVariant::List
        } else if lower.contains("large") {
            DummyCommandVariant::Large
        } else if lower.contains("fail") {
            DummyCommandVariant::Fail
        } else if lower.contains("timeout") {
            DummyCommandVariant::Timeout
        } else if lower.contains("sleep") || slow {
            DummyCommandVariant::Sleep
        } else {
            DummyCommandVariant::Fixed
        }
    } else {
        DummyCommandVariant::Fixed
    };

    Ok(ScenarioSelection {
        scenario,
        command_variant,
        slow,
        stream: lower.contains("stream")
            || matches!(scenario, DummyScenario::Basic | DummyScenario::LongStream) && slow
            || scenario == DummyScenario::LongStream,
        multi_turn,
        continuation: false,
        step: 0,
    })
}

fn explicit_scenario(text: &str) -> Option<ProviderResult<ScenarioSelection>> {
    let start = text.find("example:")?;
    let marker = text[start..]
        .split_whitespace()
        .next()?
        .trim_end_matches(['.', ',', '!', '?', ';', ')']);
    let parts: Vec<_> = marker["example:".len()..].split(':').collect();
    let scenario = match (parts.first().copied(), parts.get(1).copied()) {
        (Some("basic"), _) => DummyScenario::Basic,
        (Some("tool"), _) => DummyScenario::Tool,
        (Some("command"), _) => DummyScenario::Command,
        (Some("approval"), Some("deny")) => DummyScenario::ApprovalDeny,
        (Some("approval"), Some("cancel")) => DummyScenario::ApprovalCancel,
        (Some("approval"), _) => DummyScenario::Approval,
        (Some("workflow"), _) => DummyScenario::Workflow,
        (Some("parallel-tools"), _) => DummyScenario::ParallelTools,
        (Some("long-stream"), _) => DummyScenario::LongStream,
        (Some("user-input"), _) => DummyScenario::UserInput,
        (Some("permissions"), _) => DummyScenario::Permissions,
        (Some("goal"), _) => DummyScenario::Goal,
        (Some("mcp"), _) => DummyScenario::Mcp,
        (Some("plan"), _) => DummyScenario::Plan,
        (Some("reasoning"), _) => DummyScenario::Reasoning,
        (Some("markdown"), _) => DummyScenario::Markdown,
        (Some("subagent"), _) => DummyScenario::Subagent,
        (Some("parallel-subagents"), _) => DummyScenario::ParallelSubagents,
        (Some("multi"), _) => DummyScenario::Multi,
        (Some("error"), _) => DummyScenario::Error,
        (Some("rate-limit"), _) => DummyScenario::RateLimit,
        (Some("disconnect"), _) => DummyScenario::Disconnect,
        (Some(name), _) => {
            return Some(Err(ProviderError::InvalidRequest(format!(
                "unknown example scenario: {name}"
            ))));
        }
        (None, _) => {
            return Some(Err(ProviderError::InvalidRequest(
                "empty example scenario".to_string(),
            )));
        }
    };
    let approval_outcome = matches!(
        scenario,
        DummyScenario::ApprovalDeny | DummyScenario::ApprovalCancel
    );
    let invalid_modifier = if approval_outcome {
        parts.len() > 2
    } else {
        parts.iter().skip(1).any(|modifier| {
            let valid = match scenario {
                DummyScenario::Command => matches!(
                    *modifier,
                    "slow" | "ls" | "sleep" | "large" | "fail" | "timeout"
                ),
                DummyScenario::Approval => false,
                _ => matches!(*modifier, "slow" | "stream"),
            };
            !valid
        })
    };
    if invalid_modifier || (!approval_outcome && parts.len() > 2) {
        return Some(Err(ProviderError::InvalidRequest(
            "unsupported example scenario modifier".to_string(),
        )));
    }
    let modifier = (!approval_outcome).then(|| parts.get(1).copied()).flatten();
    let slow = modifier == Some("slow");
    let command_variant = match (scenario, modifier) {
        (DummyScenario::Command, Some("ls")) => DummyCommandVariant::List,
        (DummyScenario::Command, Some("sleep" | "slow")) => DummyCommandVariant::Sleep,
        (DummyScenario::Command, Some("large")) => DummyCommandVariant::Large,
        (DummyScenario::Command, Some("fail")) => DummyCommandVariant::Fail,
        (DummyScenario::Command, Some("timeout")) => DummyCommandVariant::Timeout,
        _ => DummyCommandVariant::Fixed,
    };
    Some(Ok(ScenarioSelection {
        scenario,
        command_variant,
        slow,
        stream: parts.contains(&"stream")
            || (scenario == DummyScenario::Basic && slow)
            || scenario == DummyScenario::LongStream,
        multi_turn: 1,
        continuation: false,
        step: 0,
    }))
}

fn goal_follow_up(input: &[ResponseItem]) -> bool {
    matches!(input.last(), Some(ResponseItem::Message { .. }))
        && has_function_call_output(input, "example-goal-create-call-1")
        && !has_function_call_output(input, "example-goal-update-call-1")
        && input
            .iter()
            .filter_map(user_message_text)
            .any(|text| is_goal_prompt(&text))
}

fn is_goal_prompt(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("example:goal")
        || lower.contains("set it as a goal")
        || lower.contains("set a goal")
        || lower.contains("create a goal")
}

fn scenario_step(input: &[ResponseItem], scenario: DummyScenario) -> usize {
    match scenario {
        DummyScenario::Plan => usize::from(has_function_call_output(input, "example-plan-call-1")),
        DummyScenario::Subagent => {
            usize::from(has_function_call_output(input, "example-subagent-call-1"))
        }
        DummyScenario::ParallelSubagents => {
            let worker_done = has_function_call_output(input, "example-parallel-subagent-call-1");
            let reviewer_done = has_function_call_output(input, "example-parallel-subagent-call-2");
            if worker_done && reviewer_done {
                2
            } else if worker_done || reviewer_done {
                1
            } else {
                0
            }
        }
        DummyScenario::Goal => {
            if has_function_call_output(input, "example-goal-update-call-1") {
                3
            } else if has_function_call_output(input, "example-goal-create-call-1") {
                1
            } else {
                0
            }
        }
        DummyScenario::Workflow => {
            if has_function_call_output(input, "example-workflow-command-call-1") {
                2
            } else if has_function_call_output(input, "example-workflow-tool-call-1") {
                1
            } else {
                0
            }
        }
        DummyScenario::ParallelTools => {
            if has_function_call_output(input, "example-parallel-tool-call-1")
                && has_function_call_output(input, "example-parallel-command-call-1")
            {
                1
            } else {
                0
            }
        }
        DummyScenario::ApprovalDeny => {
            if !has_function_call_output(input, "example-approval-deny-call-1") {
                0
            } else if function_call_output_contains(
                input,
                "example-approval-deny-call-1",
                "rejected by user",
            ) {
                1
            } else {
                2
            }
        }
        DummyScenario::ApprovalCancel => usize::from(function_call_output_contains(
            input,
            "example-approval-cancel-call-1",
            "example approval cancelled",
        )),
        DummyScenario::UserInput => {
            usize::from(has_function_call_output(input, "example-user-input-call-1"))
        }
        DummyScenario::Permissions => usize::from(has_function_call_output(
            input,
            "example-permissions-call-1",
        )),
        _ => 0,
    }
}

fn has_function_call_output(input: &[ResponseItem], expected_call_id: &str) -> bool {
    input.iter().any(|item| {
        matches!(
            item,
            ResponseItem::FunctionCallOutput { call_id, .. }
                if call_id.as_deref() == Some(expected_call_id)
        )
    })
}

fn function_call_output_contains(
    input: &[ResponseItem],
    expected_call_id: &str,
    expected_text: &str,
) -> bool {
    input.iter().any(|item| {
        matches!(
            item,
            ResponseItem::FunctionCallOutput {
                call_id, output, ..
            } if call_id.as_deref() == Some(expected_call_id)
                && output.to_string().contains(expected_text)
        )
    })
}

fn latest_user_text(input: &[ResponseItem]) -> Option<String> {
    input.iter().rev().find_map(user_message_text)
}

fn multi_turn_count(input: &[ResponseItem]) -> usize {
    input
        .iter()
        .filter(|item| user_message_text(item).is_some())
        .count()
        .max(1)
}

fn user_message_text(item: &ResponseItem) -> Option<String> {
    let ResponseItem::Message { role, content, .. } = item else {
        return None;
    };
    if role != "user" {
        return None;
    }
    let value = serde_json::to_value(content).ok()?;
    let text = value
        .as_array()?
        .iter()
        .filter_map(|item| item.get("text").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    Some(text)
}

fn continuation_scenario(input: &[ResponseItem]) -> Option<DummyScenario> {
    for item in input.iter().rev() {
        match item {
            ResponseItem::FunctionCallOutput { call_id, .. } => {
                return call_id.as_deref().and_then(scenario_from_call_id);
            }
            ResponseItem::CustomToolCallOutput { call_id, .. } => {
                return scenario_from_call_id(call_id);
            }
            ResponseItem::Message { .. } => return None,
            _ => {}
        }
    }
    None
}

fn scenario_from_call_id(call_id: &str) -> Option<DummyScenario> {
    if call_id.starts_with("example-goal-") {
        Some(DummyScenario::Goal)
    } else if call_id.starts_with("example-plan-call-") {
        Some(DummyScenario::Plan)
    } else if call_id.starts_with("example-subagent-call-") {
        Some(DummyScenario::Subagent)
    } else if call_id.starts_with("example-parallel-subagent-call-") {
        Some(DummyScenario::ParallelSubagents)
    } else if call_id.starts_with("example-tool-call-") {
        Some(DummyScenario::Tool)
    } else if call_id.starts_with("example-command-call-") {
        Some(DummyScenario::Command)
    } else if call_id.starts_with("example-approval-call-") {
        Some(DummyScenario::Approval)
    } else if call_id.starts_with("example-approval-deny-call-") {
        Some(DummyScenario::ApprovalDeny)
    } else if call_id.starts_with("example-approval-cancel-call-") {
        Some(DummyScenario::ApprovalCancel)
    } else if call_id.starts_with("example-workflow-tool-call-")
        || call_id.starts_with("example-workflow-command-call-")
    {
        Some(DummyScenario::Workflow)
    } else if call_id.starts_with("example-parallel-tool-call-")
        || call_id.starts_with("example-parallel-command-call-")
    {
        Some(DummyScenario::ParallelTools)
    } else if call_id.starts_with("example-user-input-call-") {
        Some(DummyScenario::UserInput)
    } else if call_id.starts_with("example-permissions-call-") {
        Some(DummyScenario::Permissions)
    } else if call_id.starts_with("example-mcp-call-") {
        Some(DummyScenario::Mcp)
    } else {
        None
    }
}

fn has_example_mcp_tool(tools: &[serde_json::Value]) -> bool {
    tools.iter().any(|tool| {
        let text = tool.to_string().to_ascii_lowercase();
        text.contains("example") && text.contains("echo")
    })
}

fn has_request_tool(tools: &[serde_json::Value], expected_name: &str) -> bool {
    fn contains_tool_name(value: &serde_json::Value, expected_name: &str) -> bool {
        match value {
            serde_json::Value::Object(object) => {
                object.get("name").and_then(serde_json::Value::as_str) == Some(expected_name)
                    || object
                        .values()
                        .any(|value| contains_tool_name(value, expected_name))
            }
            serde_json::Value::Array(values) => values
                .iter()
                .any(|value| contains_tool_name(value, expected_name)),
            _ => false,
        }
    }

    tools
        .iter()
        .any(|tool| contains_tool_name(tool, expected_name))
}

fn spawn_agent_tool_form(tools: &[serde_json::Value]) -> Option<SpawnAgentToolForm> {
    let has_top_level_v2_tool = tools
        .iter()
        .any(|tool| tool.get("name").and_then(serde_json::Value::as_str) == Some("spawn_agent"));
    if has_top_level_v2_tool {
        return Some(SpawnAgentToolForm::V2);
    }

    let has_v1_namespace_tool = tools.iter().any(|tool| {
        tool.get("name").and_then(serde_json::Value::as_str) == Some("multi_agent_v1")
            && tool
                .get("tools")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|nested| {
                    nested.iter().any(|nested_tool| {
                        nested_tool.get("name").and_then(serde_json::Value::as_str)
                            == Some("spawn_agent")
                    })
                })
    });
    has_v1_namespace_tool.then_some(SpawnAgentToolForm::V1)
}

impl DummyScenario {
    fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Tool => "tool",
            Self::Command => "command",
            Self::Approval => "approval",
            Self::ApprovalDeny => "approval-deny",
            Self::ApprovalCancel => "approval-cancel",
            Self::Workflow => "workflow",
            Self::ParallelTools => "parallel-tools",
            Self::LongStream => "long-stream",
            Self::UserInput => "user-input",
            Self::Permissions => "permissions",
            Self::Goal => "goal",
            Self::Mcp => "mcp",
            Self::Plan => "plan",
            Self::Reasoning => "reasoning",
            Self::Markdown => "markdown",
            Self::Subagent => "subagent",
            Self::ParallelSubagents => "parallel-subagents",
            Self::Multi => "multi",
            Self::Error => "error",
            Self::RateLimit => "rate-limit",
            Self::Disconnect => "disconnect",
        }
    }
}

pub fn provider_factory() -> ruxex_cli::ProviderRuntimeFactory {
    fn runtime(init: ProviderInit) -> AppServerRuntime {
        AppServerRuntimeBuilder::build_from_provider_factory::<DummyProvider>(init)
    }

    (runtime, DummyProvider::provider_descriptor())
}

pub fn prepare_home() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os(DUMMY_HOME_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ruxex-dummy-provider"));
    ruxex_model_provider::mount_provider_home(
        &home,
        ruxex::ProductIdentity::ruxex().home_marker_file(),
        &DummyProvider::provider_descriptor(),
    )
    .map_err(|error| anyhow::anyhow!(error))?;
    let config_path = home.join("config.toml");
    if !config_path.exists() {
        fs::write(&config_path, DEFAULT_DUMMY_CONFIG)?;
    } else if fs::read_to_string(&config_path)? == LEGACY_DEFAULT_DUMMY_CONFIG {
        // Migrate only the exact config generated by older dummy-provider versions; preserve
        // explicit user/provider settings in any other existing config.
        fs::write(&config_path, DEFAULT_DUMMY_CONFIG)?;
    }
    // SAFETY: this runs before the AppServer runtime is started and before the process creates
    // worker threads, so the process-wide home selection cannot race with another environment
    // mutation.
    unsafe { std::env::set_var("RUXEX_HOME", &home) };
    Ok(home)
}

pub async fn run(arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    if std::env::args_os().any(|arg| arg == APP_SERVER_DAEMON_MODE_ARG) {
        let exit = ruxex_cli::run_host_app_server_daemon_with_provider_factory(
            arg0_paths,
            Default::default(),
            Default::default(),
            false,
            provider_factory(),
        )
        .await?;
        if exit == ruxex_app_server::AppServerExit::Forced {
            std::process::exit(0);
        }
    } else {
        ruxex_cli::run_with_provider_factory(arg0_paths, provider_factory(), None).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use ruxex::provider::protocol::ResponseItem;
    use ruxex::provider::{ChatTransportOptions, ProviderRequestCompression};
    use ruxex_model_provider_contract_tests::{
        ChatContractConfig, assert_provider_chat_contract, validate_chat_events,
    };

    fn user_message(text: &str) -> ResponseItem {
        serde_json::from_value(serde_json::json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": text}],
        }))
        .expect("valid user message")
    }

    fn dummy_provider_for_test() -> DummyProvider {
        DummyProvider {
            credential_manager: CredentialManager::from_auth_for_testing(
                ruxex_login::RuxexAuth::from_api_key("dummy-key"),
            ),
        }
    }

    fn scenario_request(text: &str, tools: Vec<serde_json::Value>) -> ChatRequest {
        scenario_request_with_input(vec![user_message(text)], tools)
    }

    fn scenario_request_with_input(
        input: Vec<ResponseItem>,
        tools: Vec<serde_json::Value>,
    ) -> ChatRequest {
        ChatRequest {
            model: "A".to_string(),
            instructions: "Answer briefly.".to_string(),
            input,
            tools,
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

    fn run_scenario_request(
        provider: &DummyProvider,
        request: ChatRequest,
    ) -> Vec<ProviderResult<ProviderChatEvent>> {
        let events = futures::executor::block_on(async {
            provider
                .chat(request)
                .await
                .expect("scenario request should produce a stream")
                .collect::<Vec<_>>()
                .await
        });
        assert_fixed_response_usage(&events);
        events
    }

    fn assert_fixed_response_usage(events: &[ProviderResult<ProviderChatEvent>]) {
        let (token_usage, usage_metadata) = events
            .iter()
            .find_map(|event| match event {
                Ok(ProviderChatEvent::Completed {
                    token_usage: Some(token_usage),
                    usage_metadata: Some(usage_metadata),
                    ..
                }) => Some((token_usage, usage_metadata)),
                _ => None,
            })
            .expect("completed scenario response should include usage");
        assert_eq!(token_usage.input_tokens, 100);
        assert_eq!(token_usage.output_tokens, 20);
        assert_eq!(token_usage.total_tokens, 120);
        assert_eq!(usage_metadata.amount, Some(0.001));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn selects_explicit_command_slow_scenario() {
        let selection = select_scenario(&[user_message("example:command:slow")])
            .expect("scenario should be valid");

        assert_eq!(selection.scenario, DummyScenario::Command);
        assert!(selection.slow);

        let events = scenario_chat_stream(selection, true)
            .collect::<Vec<_>>()
            .await;
        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                ..
            })) if name == "exec_command"
                && arguments.contains("echo 'example-provider before sleep'")
                && arguments.contains("sleep 3")
                && arguments.contains("echo 'example-provider after sleep'")
        )));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn slow_basic_scenario_streams_the_fixed_response() {
        let selection = select_scenario(&[user_message("example:basic:slow")])
            .expect("scenario should be valid");

        assert!(selection.slow);
        assert!(selection.stream);

        let events = scenario_chat_stream(selection, true)
            .collect::<Vec<_>>()
            .await;
        let deltas = events
            .iter()
            .filter_map(|event| match event {
                Ok(ProviderChatEvent::OutputTextDelta(text)) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            deltas,
            vec!["This is a ", "fixed response from the ", "dummy provider."]
        );
    }

    #[test]
    fn ls_command_scenario_emits_ls() {
        let selection = select_scenario(&[user_message("example:command:ls")])
            .expect("scenario should be valid");
        let events =
            futures::executor::block_on(scenario_chat_stream(selection, true).collect::<Vec<_>>());

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                ..
            })) if name == "exec_command" && arguments.contains("\"cmd\":\"ls\"")
        )));
    }

    #[test]
    fn sleep_command_scenario_emits_echo_sleep_echo() {
        let selection = select_scenario(&[user_message("example:command:sleep")])
            .expect("scenario should be valid");
        let events =
            futures::executor::block_on(scenario_chat_stream(selection, true).collect::<Vec<_>>());

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                ..
            })) if name == "exec_command"
                && arguments.contains("echo 'example-provider before sleep'")
                && arguments.contains("sleep 3")
                && arguments.contains("echo 'example-provider after sleep'")
        )));
    }

    #[test]
    fn selects_keyword_scenario_and_modifier() {
        let selection = select_scenario(&[user_message("please run a command slowly")])
            .expect("scenario should be valid");

        assert_eq!(selection.scenario, DummyScenario::Command);
        assert!(selection.slow);
    }

    #[test]
    fn selects_explicit_approval_scenario() {
        let selection = select_scenario(&[user_message("example:approval")])
            .expect("approval scenario should be valid");

        assert_eq!(selection.scenario.as_str(), "approval");
    }

    #[test]
    fn selects_approval_shorthand() {
        let selection = select_scenario(&[user_message("approval")])
            .expect("approval shorthand should be valid");

        assert_eq!(selection.scenario.as_str(), "approval");
    }

    fn function_call_output(call_id: &str, name: &str) -> ResponseItem {
        function_call_output_with_text(call_id, name, "fixed tool output")
    }

    fn function_call_output_with_text(call_id: &str, name: &str, text: &str) -> ResponseItem {
        ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some(call_id.to_string()),
            name: Some(name.to_string()),
            namespace: None,
            output: serde_json::from_value(serde_json::json!(text)).expect("valid function output"),
            internal_chat_message_metadata_passthrough: None,
        }
    }

    #[test]
    fn selects_explicit_workflow_and_shorthand() {
        let explicit = select_scenario(&[user_message("example:workflow")])
            .expect("workflow scenario should be valid");
        let shorthand = select_scenario(&[user_message("workflow")])
            .expect("workflow shorthand should be valid");

        assert_eq!(explicit.scenario.as_str(), "workflow");
        assert_eq!(shorthand.scenario.as_str(), "workflow");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn workflow_scenario_progresses_from_tool_to_command_to_final() {
        let initial = select_scenario(&[user_message("example:workflow")])
            .expect("workflow scenario should be valid");
        let initial_events = scenario_chat_stream(initial, true)
            .collect::<Vec<_>>()
            .await;
        assert!(initial_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                call_id,
                ..
            })) if call_id == "example-workflow-tool-call-1"
        )));
        assert!(matches!(
            initial_events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(false),
                ..
            }))
        ));

        let after_tool = select_scenario(&[
            user_message("example:workflow"),
            function_call_output("example-workflow-tool-call-1", "example_tool"),
        ])
        .expect("workflow command phase should be valid");
        assert_eq!(after_tool.step, 1);
        let command_events = scenario_chat_stream(after_tool, true)
            .collect::<Vec<_>>()
            .await;
        assert!(command_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                call_id,
                ..
            })) if name == "exec_command" && call_id == "example-workflow-command-call-1"
        )));
        assert!(matches!(
            command_events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(false),
                ..
            }))
        ));

        let after_command = select_scenario(&[
            user_message("example:workflow"),
            function_call_output("example-workflow-tool-call-1", "example_tool"),
            function_call_output("example-workflow-command-call-1", "exec_command"),
        ])
        .expect("workflow final phase should be valid");
        assert_eq!(after_command.step, 2);
        let final_events = scenario_chat_stream(after_command, true)
            .collect::<Vec<_>>()
            .await;
        assert!(final_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The sequential workflow completed."
        )));
        assert!(matches!(
            final_events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(true),
                ..
            }))
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn parallel_tools_share_a_response_and_finish_after_both_outputs() {
        let initial = select_scenario(&[user_message("example:parallel-tools")])
            .expect("parallel tools scenario should be valid");
        let initial_events = scenario_chat_stream(initial, true)
            .collect::<Vec<_>>()
            .await;
        let response_ids = initial_events
            .iter()
            .filter_map(|event| match event {
                Ok(ProviderChatEvent::Created { response_id }) => response_id.clone(),
                _ => None,
            })
            .collect::<Vec<_>>();
        let call_ids = initial_events
            .iter()
            .filter_map(|event| match event {
                Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                    call_id,
                    ..
                })) => Some(call_id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(response_ids, vec!["example-parallel-tools-response-1"]);
        assert_eq!(
            call_ids,
            vec![
                "example-parallel-tool-call-1".to_string(),
                "example-parallel-command-call-1".to_string()
            ]
        );

        let after_outputs = select_scenario(&[
            user_message("example:parallel-tools"),
            function_call_output("example-parallel-tool-call-1", "example_tool"),
            function_call_output("example-parallel-command-call-1", "exec_command"),
        ])
        .expect("parallel completion should be valid");
        let final_events = scenario_chat_stream(after_outputs, true)
            .collect::<Vec<_>>()
            .await;
        assert!(final_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The parallel tools completed."
        )));
        assert!(matches!(
            final_events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(true),
                ..
            }))
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn long_stream_scenario_emits_bounded_markdown_chunks() {
        let selection = select_scenario(&[user_message("example:long-stream")])
            .expect("long stream scenario should be valid");
        let events = scenario_chat_stream(selection, true)
            .collect::<Vec<_>>()
            .await;
        let deltas = events
            .iter()
            .filter_map(|event| match event {
                Ok(ProviderChatEvent::OutputTextDelta(text)) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let combined = deltas.concat();

        assert!(deltas.len() >= 8);
        assert!(deltas.len() <= 64);
        assert!(combined.contains("# Long stream fixture"));
        assert!(combined.contains("```rust"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn slow_long_stream_keeps_content_and_adds_delay_flag() {
        let selection = select_scenario(&[user_message("example:long-stream:slow")])
            .expect("slow long stream scenario should be valid");

        assert!(selection.slow);
        assert!(selection.stream);
    }

    #[test]
    fn command_variants_emit_fixed_large_failure_and_timeout_commands() {
        for (marker, expected) in [
            ("example:command:large", "example-provider output line"),
            ("example:command:fail", "example-provider stderr"),
            ("example:command:timeout", "sleep 5"),
        ] {
            let selection =
                select_scenario(&[user_message(marker)]).expect("command variant should be valid");
            let events = futures::executor::block_on(
                scenario_chat_stream(selection, true).collect::<Vec<_>>(),
            );

            assert!(events.iter().any(|event| matches!(
                event,
                Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                    name,
                    arguments,
                    ..
                })) if name == "exec_command" && arguments.contains(expected)
            )));
        }

        let selection = select_scenario(&[user_message("example:command:timeout")])
            .expect("timeout command should be valid");
        let events =
            futures::executor::block_on(scenario_chat_stream(selection, true).collect::<Vec<_>>());
        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                arguments, ..
            })) if arguments.contains("\"timeout_ms\":1000")
        )));
    }

    #[test]
    fn command_variant_keywords_select_the_fixed_commands() {
        for (text, variant) in [
            ("large", DummyCommandVariant::Large),
            ("fail", DummyCommandVariant::Fail),
            ("timeout", DummyCommandVariant::Timeout),
        ] {
            let selection = select_scenario(&[user_message(text)])
                .expect("command keyword should select a scenario");
            assert_eq!(selection.scenario, DummyScenario::Command);
            assert_eq!(selection.command_variant, variant);
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn approval_scenario_emits_an_escalated_command() {
        let selection = select_scenario(&[user_message("example:approval")])
            .expect("approval scenario should be valid");
        let events = scenario_chat_stream(selection, true)
            .collect::<Vec<_>>()
            .await;

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            })) if name == "exec_command"
                && arguments.contains("approval-required command output")
                && arguments.contains("require_escalated")
                && arguments.contains("Run the example provider approval command.")
                && call_id == "example-approval-call-1"
        )));
        assert!(matches!(
            events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(false),
                ..
            }))
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn approval_scenario_continues_after_command_result() {
        let approval_call = ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("example-approval-call-1".to_string()),
            name: Some("exec_command".to_string()),
            namespace: None,
            output: serde_json::from_value(serde_json::json!("approved command output"))
                .expect("valid function output"),
            internal_chat_message_metadata_passthrough: None,
        };

        assert_eq!(
            continuation_scenario(&[approval_call]),
            Some(DummyScenario::Approval)
        );

        let mut selection = select_scenario(&[user_message("example:approval")])
            .expect("approval scenario should be valid");
        selection.continuation = true;
        let events = scenario_chat_stream(selection, true)
            .collect::<Vec<_>>()
            .await;

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The approved command result was received."
        )));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn approval_outcomes_keep_decisions_on_the_client_side() {
        for (marker, call_id, result, answer) in [
            (
                "example:approval:deny",
                "example-approval-deny-call-1",
                "exec command rejected by user",
                "The approval was denied by the client.",
            ),
            (
                "example:approval:cancel",
                "example-approval-cancel-call-1",
                "example approval cancelled",
                "The approval was cancelled by the client.",
            ),
        ] {
            let initial = select_scenario(&[user_message(marker)])
                .expect("approval outcome scenario should be valid");
            let initial_events = scenario_chat_stream(initial, true)
                .collect::<Vec<_>>()
                .await;
            assert!(initial_events.iter().any(|event| matches!(
                event,
                Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                    call_id: emitted_call_id,
                    arguments,
                    ..
                })) if emitted_call_id == call_id
                    && arguments.contains("require_escalated")
            )));
            assert!(matches!(
                initial_events.last(),
                Some(Ok(ProviderChatEvent::Completed {
                    end_turn: Some(false),
                    ..
                }))
            ));

            let continuation = select_scenario(&[
                user_message(marker),
                function_call_output_with_text(call_id, "exec_command", result),
            ])
            .expect("approval outcome continuation should be valid");
            let events = scenario_chat_stream(continuation, true)
                .collect::<Vec<_>>()
                .await;
            assert!(events.iter().any(|event| matches!(
                event,
                Ok(ProviderChatEvent::OutputTextDelta(text)) if text == answer
            )));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn approval_deny_scenario_finishes_when_the_client_approves_instead() {
        let continuation = select_scenario(&[
            user_message("example:approval:deny"),
            function_call_output_with_text(
                "example-approval-deny-call-1",
                "exec_command",
                "approval-required command output",
            ),
        ])
        .expect("approval outcome continuation should be valid");
        let events = scenario_chat_stream(continuation, true)
            .collect::<Vec<_>>()
            .await;

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == FIXED_APPROVAL_ANSWER
        )));
        assert!(matches!(
            events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(true),
                ..
            }))
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn user_input_and_permissions_scenarios_request_fixed_client_interactions() {
        let user_input = select_scenario(&[user_message("example:user-input")])
            .expect("user input scenario should be valid");
        let user_input_events = scenario_chat_stream(user_input, true)
            .collect::<Vec<_>>()
            .await;
        assert!(user_input_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            })) if name == "request_user_input"
                && call_id == "example-user-input-call-1"
                && arguments.contains("Proceed with the fixed UI test?")
        )));

        let user_input_continuation = select_scenario(&[
            user_message("example:user-input"),
            function_call_output("example-user-input-call-1", "request_user_input"),
        ])
        .expect("user input continuation should be valid");
        let user_input_final = scenario_chat_stream(user_input_continuation, true)
            .collect::<Vec<_>>()
            .await;
        assert!(user_input_final.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The fixed user input was received."
        )));

        let permissions = select_scenario(&[user_message("example:permissions")])
            .expect("permissions scenario should be valid");
        let permissions_events = scenario_chat_stream(permissions, true)
            .collect::<Vec<_>>()
            .await;
        assert!(permissions_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            })) if name == "request_permissions"
                && call_id == "example-permissions-call-1"
                && arguments.contains("Select a workspace root")
                && arguments.contains("../shared")
        )));

        let permissions_continuation = select_scenario(&[
            user_message("example:permissions"),
            function_call_output("example-permissions-call-1", "request_permissions"),
        ])
        .expect("permissions continuation should be valid");
        let permissions_final = scenario_chat_stream(permissions_continuation, true)
            .collect::<Vec<_>>()
            .await;
        assert!(permissions_final.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The fixed permissions response was received."
        )));
    }

    #[test]
    fn tool_output_continues_the_matching_scenario() {
        let input = vec![
            user_message("tool"),
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: Some("example-tool-call-1".to_string()),
                name: Some("example_tool".to_string()),
                namespace: None,
                output: serde_json::from_value(serde_json::json!("fixed tool output"))
                    .expect("valid function output"),
                internal_chat_message_metadata_passthrough: None,
            },
        ];

        assert_eq!(continuation_scenario(&input), Some(DummyScenario::Tool));
    }

    #[test]
    fn a_new_user_turn_does_not_reuse_an_old_tool_continuation() {
        let input = vec![
            user_message("tool"),
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: Some("example-tool-call-1".to_string()),
                name: Some("example_tool".to_string()),
                namespace: None,
                output: serde_json::from_value(serde_json::json!("fixed tool output"))
                    .expect("valid function output"),
                internal_chat_message_metadata_passthrough: None,
            },
            user_message("hello again"),
        ];

        assert_eq!(continuation_scenario(&input), None);
        assert_eq!(
            select_scenario(&input)
                .expect("scenario should be valid")
                .scenario,
            DummyScenario::Basic
        );
    }

    #[test]
    fn multi_scenario_is_inherited_by_a_follow_up_user_turn() {
        let input = vec![user_message("multi"), user_message("continue")];
        let selection = select_scenario(&input).expect("scenario should be valid");

        assert_eq!(selection.scenario, DummyScenario::Multi);
        assert_eq!(selection.multi_turn, 2);
    }

    #[test]
    fn explicit_multi_scenario_counts_the_current_conversation_turn() {
        let input = vec![
            user_message("example:multi"),
            user_message("continue"),
            user_message("example:multi next"),
        ];
        let selection = select_scenario(&input).expect("scenario should be valid");

        assert_eq!(selection.scenario, DummyScenario::Multi);
        assert_eq!(selection.multi_turn, 3);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn goal_scenario_sets_then_completes_a_goal_on_the_next_user_turn() {
        let initial_prompt =
            "Please track this fixture review checklist and set it as a goal. example:goal";
        let natural_language = select_scenario(&[user_message(
            "Please set it as a goal for this conversation: prepare the fixture review checklist.",
        )])
        .expect("natural-language goal prompt should select the goal scenario");
        assert_eq!(natural_language.scenario, DummyScenario::Goal);

        let initial = select_scenario(&[user_message(initial_prompt)])
            .expect("goal scenario should be valid");
        assert_eq!(initial.scenario, DummyScenario::Goal);
        assert_eq!(initial.step, 0);

        let initial_events = scenario_chat_stream(initial, true)
            .collect::<Vec<_>>()
            .await;
        assert!(initial_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                call_id,
                arguments,
                ..
            })) if name == "create_goal"
                && call_id == "example-goal-create-call-1"
                && arguments.contains("Prepare the fixture review checklist.")
        )));
        assert!(matches!(
            initial_events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(false),
                ..
            }))
        ));

        let after_create = select_scenario(&[
            user_message(initial_prompt),
            function_call_output("example-goal-create-call-1", "create_goal"),
        ])
        .expect("goal creation continuation should be valid");
        assert_eq!(after_create.scenario, DummyScenario::Goal);
        assert!(after_create.continuation);
        assert_eq!(after_create.step, 1);
        let create_ack_events = scenario_chat_stream(after_create, true)
            .collect::<Vec<_>>()
            .await;
        assert!(create_ack_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The fixture review goal was set."
        )));

        let next_user_turn = select_scenario(&[
            user_message(initial_prompt),
            function_call_output("example-goal-create-call-1", "create_goal"),
            user_message("I finished the checklist. Please complete the goal."),
        ])
        .expect("goal follow-up should be valid");
        assert_eq!(next_user_turn.scenario, DummyScenario::Goal);
        assert!(!next_user_turn.continuation);
        assert_eq!(next_user_turn.step, 2);
        let update_events = scenario_chat_stream(next_user_turn, true)
            .collect::<Vec<_>>()
            .await;
        assert!(update_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                call_id,
                arguments,
                ..
            })) if name == "update_goal"
                && call_id == "example-goal-update-call-1"
                && arguments.contains("complete")
        )));
        assert!(matches!(
            update_events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(false),
                ..
            }))
        ));

        let after_update = select_scenario(&[
            user_message(initial_prompt),
            function_call_output("example-goal-create-call-1", "create_goal"),
            user_message("I finished the checklist. Please complete the goal."),
            function_call_output("example-goal-update-call-1", "update_goal"),
        ])
        .expect("goal completion continuation should be valid");
        assert!(after_update.continuation);
        assert_eq!(after_update.step, 3);
        let completion_events = scenario_chat_stream(after_update, true)
            .collect::<Vec<_>>()
            .await;
        assert!(completion_events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The fixture review goal was completed."
        )));
    }

    #[test]
    fn multi_scenario_returns_to_basic_after_three_turns() {
        let input = vec![
            user_message("multi"),
            user_message("continue"),
            user_message("continue"),
            user_message("after completion"),
        ];
        let selection = select_scenario(&input).expect("scenario should be valid");

        assert_eq!(selection.scenario, DummyScenario::Basic);
    }

    #[test]
    fn tool_scenario_emits_a_fixed_function_call_before_completion() {
        let events = futures::executor::block_on(
            scenario_chat_stream(
                ScenarioSelection {
                    scenario: DummyScenario::Tool,
                    command_variant: DummyCommandVariant::Fixed,
                    slow: false,
                    stream: false,
                    multi_turn: 1,
                    continuation: false,
                    step: 0,
                },
                true,
            )
            .collect::<Vec<_>>(),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            })) if name == "example_tool"
                && arguments.contains("fixed-example-input")
                && call_id == "example-tool-call-1"
        )));
        assert!(matches!(
            events.last(),
            Some(Ok(ProviderChatEvent::Completed {
                end_turn: Some(false),
                ..
            }))
        ));
    }

    #[test]
    fn command_scenario_emits_the_fixed_command() {
        let events = futures::executor::block_on(
            scenario_chat_stream(
                ScenarioSelection {
                    scenario: DummyScenario::Command,
                    command_variant: DummyCommandVariant::Fixed,
                    slow: false,
                    stream: false,
                    multi_turn: 1,
                    continuation: false,
                    step: 0,
                },
                true,
            )
            .collect::<Vec<_>>(),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                ..
            })) if name == "exec_command"
                && arguments.contains("example-provider command output")
        )));
    }

    #[test]
    fn mcp_scenario_without_the_fixture_tool_returns_a_fixed_message() {
        let events = futures::executor::block_on(
            text_chat_stream(
                ScenarioSelection {
                    scenario: DummyScenario::Mcp,
                    command_variant: DummyCommandVariant::Fixed,
                    slow: false,
                    stream: false,
                    multi_turn: 1,
                    continuation: false,
                    step: 0,
                },
                FIXED_MCP_UNAVAILABLE,
            )
            .collect::<Vec<_>>(),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text)) if text == FIXED_MCP_UNAVAILABLE
        )));
    }

    #[test]
    fn plan_scenario_uses_update_plan_when_available() {
        let provider = dummy_provider_for_test();
        let events = run_scenario_request(
            &provider,
            scenario_request(
                "example:plan",
                vec![serde_json::json!({"type":"function", "name":"update_plan"})],
            ),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                name,
                arguments,
                ..
            })) if name == "update_plan"
                && arguments.contains("in_progress")
                && arguments.contains("pending")
        )));
    }

    #[test]
    fn plan_scenario_falls_back_when_update_plan_is_unavailable() {
        let provider = dummy_provider_for_test();
        let events = run_scenario_request(&provider, scenario_request("example:plan", Vec::new()));

        assert!(!events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(
                ResponseItem::FunctionCall { .. }
            ))
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text)) if text.contains("update_plan")
        )));
    }

    #[test]
    fn plan_scenario_completes_after_update_plan_output() {
        let provider = dummy_provider_for_test();
        let events = run_scenario_request(
            &provider,
            scenario_request_with_input(
                vec![
                    user_message("example:plan"),
                    function_call_output("example-plan-call-1", "update_plan"),
                ],
                vec![serde_json::json!({"type":"function", "name":"update_plan"})],
            ),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The fixed plan fixture is complete."
        )));
        assert!(!events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(
                ResponseItem::FunctionCall { .. }
            ))
        )));
    }

    #[test]
    fn reasoning_scenario_emits_a_fixed_summary() {
        let provider = dummy_provider_for_test();
        let events =
            run_scenario_request(&provider, scenario_request("example:reasoning", Vec::new()));

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::ReasoningSummaryDone { text, .. })
                if text == "I will keep this fixture response concise and show the summary separately."
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "This response follows the fixed reasoning summary."
        )));
    }

    #[test]
    fn markdown_scenario_contains_renderer_fixture_syntax() {
        let provider = dummy_provider_for_test();
        let events =
            run_scenario_request(&provider, scenario_request("example:markdown", Vec::new()));
        let answer = events
            .iter()
            .filter_map(|event| match event {
                Ok(ProviderChatEvent::OutputTextDelta(text)) => Some(text.as_str()),
                _ => None,
            })
            .collect::<String>();

        for syntax in [
            "# Markdown rendering fixture",
            "**bold**",
            "~~strikethrough~~",
            "`inline code`",
            "[Ruxex](https://example.com)",
            "1. Ordered item",
            "- [x] Completed task",
            "> A blockquote",
            "| Feature | Rendering |",
            "---",
            "```rust",
            "```diff",
            "```mermaid",
        ] {
            assert!(answer.contains(syntax), "missing Markdown syntax: {syntax}");
        }
    }

    #[test]
    fn single_subagent_scenario_emits_supported_spawn_agent_form() {
        let provider = dummy_provider_for_test();
        let tool_forms = [
            (
                serde_json::json!({
                    "type": "namespace",
                    "name": "multi_agent_v1",
                    "tools": [{"type": "function", "name": "spawn_agent"}]
                }),
                Some("multi_agent_v1"),
            ),
            (
                serde_json::json!({"type": "function", "name": "spawn_agent"}),
                None,
            ),
        ];

        for (tool, expected_namespace) in tool_forms {
            let events =
                run_scenario_request(&provider, scenario_request("example:subagent", vec![tool]));
            let calls = events
                .iter()
                .filter_map(|event| match event {
                    Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                        namespace,
                        name,
                        arguments,
                        call_id,
                        ..
                    })) => Some((
                        namespace.as_deref(),
                        name.as_str(),
                        arguments.as_str(),
                        call_id.as_str(),
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>();

            assert_eq!(calls.len(), 1);
            let (namespace, name, arguments, call_id) = calls[0];
            assert_eq!(namespace, expected_namespace);
            assert_eq!(name, "spawn_agent");
            assert_eq!(call_id, "example-subagent-call-1");
            let arguments: serde_json::Value =
                serde_json::from_str(arguments).expect("spawn_agent arguments should be JSON");
            assert!(
                arguments["message"]
                    .as_str()
                    .unwrap()
                    .contains("Do not call tools.")
            );
            if expected_namespace.is_none() {
                assert_eq!(arguments["task_name"], "example_worker");
            }
        }
    }

    #[test]
    fn parallel_subagents_scenario_emits_two_calls_with_distinct_names() {
        let provider = dummy_provider_for_test();
        let events = run_scenario_request(
            &provider,
            scenario_request(
                "example:parallel-subagents",
                vec![serde_json::json!({"type":"function", "name":"spawn_agent"})],
            ),
        );
        let calls = events
            .iter()
            .filter_map(|event| match event {
                Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                    namespace,
                    name,
                    arguments,
                    call_id,
                    ..
                })) => Some((namespace, name, arguments, call_id)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|(namespace, name, ..)| {
            namespace.is_none() && name.as_str() == "spawn_agent"
        }));
        assert_eq!(calls[0].3, "example-parallel-subagent-call-1");
        assert_eq!(calls[1].3, "example-parallel-subagent-call-2");
        assert!(calls[0].2.contains("example_worker"));
        assert!(calls[1].2.contains("example_reviewer"));
    }

    #[test]
    fn subagent_scenario_falls_back_without_spawn_tool() {
        let provider = dummy_provider_for_test();
        let events =
            run_scenario_request(&provider, scenario_request("example:subagent", Vec::new()));

        assert!(!events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(
                ResponseItem::FunctionCall { .. }
            ))
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text)) if text.contains("spawn_agent")
        )));
    }

    #[test]
    fn subagent_scenarios_complete_after_matching_tool_outputs() {
        let provider = dummy_provider_for_test();
        let v2_spawn_tool = vec![serde_json::json!({"type":"function", "name":"spawn_agent"})];

        let single = run_scenario_request(
            &provider,
            scenario_request_with_input(
                vec![
                    user_message("example:subagent"),
                    function_call_output("example-subagent-call-1", "spawn_agent"),
                ],
                v2_spawn_tool.clone(),
            ),
        );
        assert!(single.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The Example subagent fixture is complete."
        )));
        assert!(!single.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(
                ResponseItem::FunctionCall { .. }
            ))
        )));

        let after_first_parallel_output = run_scenario_request(
            &provider,
            scenario_request_with_input(
                vec![
                    user_message("example:parallel-subagents"),
                    function_call_output("example-parallel-subagent-call-1", "spawn_agent"),
                ],
                v2_spawn_tool.clone(),
            ),
        );
        let remaining_calls = after_first_parallel_output
            .iter()
            .filter_map(|event| match event {
                Ok(ProviderChatEvent::OutputItemDone(ResponseItem::FunctionCall {
                    call_id,
                    ..
                })) => Some(call_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(remaining_calls, vec!["example-parallel-subagent-call-2"]);
        assert!(!after_first_parallel_output.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The Example parallel subagent fixture is complete."
        )));

        let parallel = run_scenario_request(
            &provider,
            scenario_request_with_input(
                vec![
                    user_message("example:parallel-subagents"),
                    function_call_output("example-parallel-subagent-call-1", "spawn_agent"),
                    function_call_output("example-parallel-subagent-call-2", "spawn_agent"),
                ],
                v2_spawn_tool,
            ),
        );
        assert!(parallel.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "The Example parallel subagent fixture is complete."
        )));
        assert!(!parallel.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputItemDone(
                ResponseItem::FunctionCall { .. }
            ))
        )));
    }

    #[test]
    fn completed_text_and_tool_responses_report_fixed_usage() {
        for marker in ["example:basic", "example:tool"] {
            let selection = select_scenario(&[user_message(marker)])
                .expect("usage fixture scenario should be valid");
            let events = futures::executor::block_on(
                scenario_chat_stream(selection, true).collect::<Vec<_>>(),
            );
            let (token_usage, usage_metadata) = events
                .iter()
                .find_map(|event| match event {
                    Ok(ProviderChatEvent::Completed {
                        token_usage: Some(token_usage),
                        usage_metadata: Some(usage_metadata),
                        ..
                    }) => Some((token_usage, usage_metadata)),
                    _ => None,
                })
                .expect("completed response should include token and amount usage");

            assert_eq!(token_usage.input_tokens, 100, "{marker}");
            assert_eq!(token_usage.cached_input_tokens, 0, "{marker}");
            assert_eq!(token_usage.cache_write_input_tokens, 0, "{marker}");
            assert_eq!(token_usage.output_tokens, 20, "{marker}");
            assert_eq!(token_usage.reasoning_output_tokens, 0, "{marker}");
            assert_eq!(token_usage.total_tokens, 120, "{marker}");
            assert_eq!(usage_metadata.amount, Some(0.001), "{marker}");
        }
    }

    #[test]
    fn disconnect_scenario_emits_partial_output_without_completion() {
        let selection = select_scenario(&[user_message("example:disconnect")])
            .expect("disconnect scenario should be valid");
        let events =
            futures::executor::block_on(scenario_chat_stream(selection, true).collect::<Vec<_>>());

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "This response is intentionally interrupted."
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Err(ProviderError::Transport(message))
                if message == "example provider disconnected after partial output"
        )));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Ok(ProviderChatEvent::Completed { .. })))
        );
    }

    #[test]
    fn exposes_three_fixed_models() {
        let models = fixed_models();

        assert_eq!(
            models
                .iter()
                .map(|model| model.slug.as_str())
                .collect::<Vec<_>>(),
            ["A", "B", "C"]
        );
    }

    #[test]
    fn fixed_models_are_visible_in_the_model_picker() {
        let models = fixed_models();

        assert!(
            models
                .iter()
                .all(|model| model.visibility == ruxex::provider::protocol::ModelVisibility::List)
        );
    }

    #[test]
    fn model_catalog_identity_matches_list_models() {
        let provider = DummyProvider {
            credential_manager: ruxex_login::CredentialManager::from_auth_for_testing(
                ruxex_login::RuxexAuth::from_api_key("dummy-key"),
            ),
        };
        let catalog =
            futures::executor::block_on(provider.list_models(ListModelsRequest::new("test")))
                .expect("dummy model catalog should be available");

        assert_eq!(
            provider
                .model_catalog_identity()
                .expect("dummy model identity should be available"),
            catalog.identity
        );
    }

    #[test]
    fn returns_one_fixed_chat_answer() {
        let events = futures::executor::block_on(
            scenario_chat_stream(
                ScenarioSelection {
                    scenario: DummyScenario::Basic,
                    command_variant: DummyCommandVariant::Fixed,
                    slow: false,
                    stream: false,
                    multi_turn: 1,
                    continuation: false,
                    step: 0,
                },
                true,
            )
            .collect::<Vec<_>>(),
        );

        let output_item_added = events.iter().position(|event| {
            matches!(
                event,
                Ok(ProviderChatEvent::OutputItemAdded(ResponseItem::Message {
                    role,
                    ..
                })) if role == "assistant"
            )
        });
        let output_text_delta = events.iter().position(|event| {
            matches!(event, Ok(ProviderChatEvent::OutputTextDelta(text)) if text == FIXED_CHAT_ANSWER)
        });
        let output_item_done = events.iter().position(|event| {
            matches!(
                event,
                Ok(ProviderChatEvent::OutputItemDone(ResponseItem::Message {
                    role,
                    ..
                })) if role == "assistant"
            )
        });

        assert!(
            output_item_added.expect("basic chat should start an assistant item")
                < output_text_delta.expect("basic chat should emit text")
        );
        assert!(
            output_text_delta.expect("basic chat should emit text")
                < output_item_done.expect("basic chat should complete an assistant item")
        );
        assert!(events.iter().any(|event| {
            matches!(
                event,
                Ok(ruxex::provider::ProviderChatEvent::OutputTextDelta(text))
                    if text == FIXED_CHAT_ANSWER
            )
        }));
        assert!(matches!(
            events.last(),
            Some(Ok(ruxex::provider::ProviderChatEvent::Completed { .. }))
        ));
    }

    #[test]
    fn basic_scenario_satisfies_the_shared_chat_contract() {
        let events = futures::executor::block_on(
            scenario_chat_stream(
                ScenarioSelection {
                    scenario: DummyScenario::Basic,
                    command_variant: DummyCommandVariant::Fixed,
                    slow: false,
                    stream: true,
                    multi_turn: 1,
                    continuation: false,
                    step: 0,
                },
                false,
            )
            .collect::<Vec<_>>(),
        );

        let report = validate_chat_events(events, &ChatContractConfig::default())
            .expect("basic scenario should satisfy the shared chat contract");
        assert_eq!(report.assistant_text, FIXED_CHAT_ANSWER);
    }

    #[tokio::test]
    async fn provider_chat_satisfies_the_shared_live_contract() {
        let provider = DummyProvider {
            credential_manager: CredentialManager::from_auth_for_testing(
                ruxex_login::RuxexAuth::from_api_key("dummy-key"),
            ),
        };
        let report = assert_provider_chat_contract(
            &provider,
            ChatRequest {
                model: "A".to_string(),
                instructions: "Answer briefly.".to_string(),
                input: vec![user_message("hello")],
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
                transport: ChatTransportOptions {
                    session_id: None,
                    thread_id: None,
                    extra_headers: BTreeMap::new(),
                    compression: ProviderRequestCompression::None,
                    turn_state: None,
                    connection_key: None,
                    credential_snapshot: None,
                },
            },
            &ChatContractConfig::default(),
        )
        .await
        .expect("dummy provider chat should satisfy the shared live contract");

        assert_eq!(report.assistant_text, FIXED_CHAT_ANSWER);
    }

    #[test]
    fn standalone_tool_item_satisfies_the_shared_chat_contract() {
        let events = futures::executor::block_on(
            scenario_chat_stream(
                ScenarioSelection {
                    scenario: DummyScenario::Tool,
                    command_variant: DummyCommandVariant::Fixed,
                    slow: false,
                    stream: false,
                    multi_turn: 1,
                    continuation: false,
                    step: 0,
                },
                false,
            )
            .collect::<Vec<_>>(),
        );

        validate_chat_events(
            events,
            &ChatContractConfig::default().without_required_text(),
        )
        .expect("standalone tool item should satisfy the shared chat contract");
    }

    #[test]
    fn fixed_chat_answer_is_in_english() {
        let events = futures::executor::block_on(
            text_chat_stream(
                ScenarioSelection {
                    scenario: DummyScenario::Basic,
                    command_variant: DummyCommandVariant::Fixed,
                    slow: false,
                    stream: false,
                    multi_turn: 1,
                    continuation: false,
                    step: 0,
                },
                FIXED_CHAT_ANSWER,
            )
            .collect::<Vec<_>>(),
        );

        assert!(events.iter().any(|event| matches!(
            event,
            Ok(ProviderChatEvent::OutputTextDelta(text))
                if text == "This is a fixed response from the dummy provider."
        )));
    }

    #[test]
    fn rejects_blank_api_keys() {
        assert!(validate_api_key("   ").is_err());
        assert_eq!(validate_api_key("dummy-key").unwrap(), "dummy-key");
    }

    #[test]
    fn login_marks_the_provider_authenticated() {
        let home = tempfile::tempdir().expect("test home");
        let manager = Arc::new(futures::executor::block_on(CredentialManager::new(
            home.path().to_path_buf(),
            false,
            ruxex_login::AuthCredentialsStoreMode::Ephemeral,
            None,
            None,
            ruxex_login::AuthKeyringBackendKind::default(),
            ruxex_login::AuthRouteConfig::from_http_client_factory(
                ruxex::provider::transport::HttpClientFactory::new(
                    ruxex::provider::transport::OutboundProxyPolicy::ReqwestDefault,
                ),
            ),
        )));
        let provider = DummyProvider {
            credential_manager: manager,
        };
        let mut credentials = BTreeMap::new();
        credentials.insert("api_key".to_string(), "dummy-key".to_string());

        futures::executor::block_on(provider.login(credentials)).expect("login succeeds");

        assert!(provider.is_authenticated());
        assert_eq!(
            provider.auth_state().account_id.as_deref(),
            Some("dummy-account")
        );
    }
}
