use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use crate::responses_metadata::CompactionTrigger;
use genex_core_plugins::executor_plugin_hook_sources;
use genex_hooks::InterruptRequest;
use genex_hooks::PermissionRequestDecision;
use genex_hooks::PermissionRequestOutcome;
use genex_hooks::PermissionRequestRequest;
use genex_hooks::PostToolUseOutcome;
use genex_hooks::PostToolUseRequest;
use genex_hooks::PreToolUseOutcome;
use genex_hooks::PreToolUseRequest;
use genex_hooks::SessionStartOutcome;
use genex_hooks::StartHookTarget;
use genex_hooks::StopHookTarget;
use genex_hooks::StopOutcome;
use genex_hooks::SubagentHookContext;
use genex_hooks::UserPromptSubmitOutcome;
use genex_hooks::UserPromptSubmitRequest;
use genex_hooks::hook_execution_mode_label;
use genex_hooks::hook_handler_type_label;
use genex_otel::HOOK_RUN_DURATION_METRIC;
use genex_otel::HOOK_RUN_METRIC;
use genex_plugin::ExecutorPluginHookSource;
use genex_protocol::items::FunctionCallOutputItem;
use genex_protocol::items::TurnItem;
use genex_protocol::items::UserMessageItem;
use genex_protocol::model_types::ModelInfo;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::AskForApproval;
use genex_protocol::protocol::EventMsg;
use genex_protocol::protocol::GenexErrorInfo;
use genex_protocol::protocol::HookCompletedEvent;
use genex_protocol::protocol::HookEventName;
use genex_protocol::protocol::HookExecutionMode;
use genex_protocol::protocol::HookOutputEntryKind;
use genex_protocol::protocol::HookRunStatus;
use genex_protocol::protocol::HookRunSummary;
use genex_protocol::protocol::HookSource;
use genex_protocol::protocol::HookStartedEvent;
use genex_protocol::protocol::SessionSource;
use genex_protocol::protocol::SubAgentSource;
use genex_protocol::protocol::WarningEvent;
use genex_thread_store::PersistContext;
use genex_thread_store::ReadThreadParams;
use serde_json::Map;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::instrument;

use crate::context::ContextualUserFragment;
use crate::context::HookAdditionalContext;
use crate::event_mapping::parse_turn_item;
use crate::session::TurnInput;
use crate::session::session::Session;
use crate::session::step_context::StepContext;
use crate::session::turn_context::TurnContext;
use crate::state::TurnState;
use crate::tools::hook_names::HookToolName;
use crate::tools::sandboxing::PermissionRequestPayload;
use crate::turn_metadata::ExecutionMetadata;

pub(crate) struct HookRuntimeOutcome {
    pub should_stop: bool,
    pub additional_contexts: Vec<String>,
}

pub(crate) enum PreToolUseHookResult {
    Continue { updated_input: Option<Value> },
    Blocked(String),
}

struct ContextInjectingHookOutcome {
    hook_events: Vec<HookCompletedEvent>,
    outcome: HookRuntimeOutcome,
}

impl From<SessionStartOutcome> for ContextInjectingHookOutcome {
    fn from(value: SessionStartOutcome) -> Self {
        let SessionStartOutcome {
            hook_events,
            should_stop,
            stop_reason: _,
            additional_contexts,
        } = value;
        Self {
            hook_events,
            outcome: HookRuntimeOutcome {
                should_stop,
                additional_contexts,
            },
        }
    }
}

impl From<UserPromptSubmitOutcome> for ContextInjectingHookOutcome {
    fn from(value: UserPromptSubmitOutcome) -> Self {
        let UserPromptSubmitOutcome {
            hook_events,
            should_stop,
            stop_reason: _,
            additional_contexts,
        } = value;
        Self {
            hook_events,
            outcome: HookRuntimeOutcome {
                should_stop,
                additional_contexts,
            },
        }
    }
}

#[instrument(level = "trace", skip_all)]
pub(crate) async fn run_pending_session_start_hooks(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
) -> bool {
    while let Some(session_start_source) = sess.take_pending_session_start_source().await {
        // Spawned subagents can start fresh or fork their parent's history, so both
        // sources dispatch SubagentStart. Internal/system subagents skip start hooks.
        let target = match &turn_context.session_source {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_role, .. })
                if matches!(
                    session_start_source,
                    genex_hooks::SessionStartSource::Startup
                        | genex_hooks::SessionStartSource::Fork
                ) =>
            {
                let context = subagent_hook_context(sess, agent_role);
                StartHookTarget::SubagentStart {
                    turn_id: turn_context.sub_id.clone(),
                    agent_id: context.agent_id,
                    agent_type: context.agent_type,
                }
            }
            SessionSource::SubAgent(_) => return false,
            _ => StartHookTarget::SessionStart {
                source: session_start_source,
            },
        };
        let request = genex_hooks::SessionStartRequest {
            session_id: sess.session_id().into(),
            #[allow(deprecated)]
            cwd: turn_context.cwd.clone(),
            transcript_path: sess.hook_transcript_path().await,
            model: turn_context.model_info().slug.clone(),
            permission_mode: hook_permission_mode(turn_context.approval_policy()),
            target,
        };
        let hooks = sess.hooks();
        let preview_runs = hooks.preview_session_start(&request);
        if run_context_injecting_hook(
            sess,
            turn_context,
            preview_runs,
            hooks.run_session_start(request, Some(turn_context.sub_id.clone())),
        )
        .await
        .record_additional_contexts(sess, turn_context)
        .await
        {
            return true;
        }
    }

    false
}

/// Runs matching `PreToolUse` hooks before a tool executes.
///
/// `tool_name` is the canonical name serialized to hook stdin. Matcher aliases
/// are internal compatibility names used only for selecting configured hook
/// handlers.
pub(crate) async fn run_pre_tool_use_hooks(
    sess: &Arc<Session>,
    step_context: &StepContext,
    tool_use_id: String,
    tool_name: &HookToolName,
    tool_input: &Value,
) -> PreToolUseHookResult {
    let turn_context = &step_context.turn;
    let request = PreToolUseRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        subagent: thread_spawn_subagent_hook_context(sess, turn_context),
        #[allow(deprecated)]
        cwd: turn_context.cwd.clone(),
        transcript_path: sess.hook_transcript_path().await,
        model: step_context.settings.model_info.slug.clone(),
        permission_mode: hook_permission_mode(step_context.settings.approval_policy()),
        tool_name: tool_name.name().to_string(),
        matcher_aliases: tool_name.matcher_aliases().to_vec(),
        tool_use_id,
        tool_input: tool_input.clone(),
    };
    let hooks = sess.hooks();
    let preview_runs = hooks.preview_pre_tool_use(&request);
    emit_hook_started_events(sess, turn_context, preview_runs).await;

    let PreToolUseOutcome {
        hook_events,
        should_block,
        block_reason,
        additional_contexts,
        updated_input,
    } = hooks.run_pre_tool_use(request).await;
    emit_hook_completed_events(sess, turn_context, hook_events).await;
    record_additional_contexts(sess, turn_context, additional_contexts).await;

    if !should_block {
        return PreToolUseHookResult::Continue { updated_input };
    }

    let Some(reason) = block_reason else {
        return PreToolUseHookResult::Continue {
            updated_input: None,
        };
    };

    if (tool_name.name() == "Bash" || tool_name.name() == "apply_patch")
        && let Some(command) = tool_input.get("command").and_then(Value::as_str)
    {
        PreToolUseHookResult::Blocked(format!(
            "Command blocked by PreToolUse hook: {reason}. Command: {command}"
        ))
    } else {
        PreToolUseHookResult::Blocked(format!(
            "Tool call blocked by PreToolUse hook: {reason}. Tool: {}",
            tool_name.name()
        ))
    }
}

// PermissionRequest hooks share the same preview/start/completed event flow as
// other hook types, but they return an optional decision instead of mutating
// tool input or post-run state.
pub(crate) async fn run_permission_request_hooks(
    sess: &Arc<Session>,
    review_context: &crate::tools::approvals::ApprovalReviewContext,
    run_id_suffix: &str,
    payload: PermissionRequestPayload,
) -> Option<PermissionRequestDecision> {
    let turn_context = review_context.turn();
    let request = PermissionRequestRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        subagent: thread_spawn_subagent_hook_context(sess, turn_context),
        #[allow(deprecated)]
        cwd: turn_context.cwd.to_path_buf(),
        transcript_path: sess.hook_transcript_path().await,
        model: review_context.model_info.slug.clone(),
        permission_mode: hook_permission_mode(review_context.approval_policy),
        tool_name: payload.tool_name.name().to_string(),
        matcher_aliases: payload.tool_name.matcher_aliases().to_vec(),
        run_id_suffix: run_id_suffix.to_string(),
        tool_input: payload.tool_input,
    };
    let hooks = sess.hooks();
    let preview_runs = hooks.preview_permission_request(&request);
    emit_hook_started_events(sess, turn_context, preview_runs).await;

    let PermissionRequestOutcome {
        hook_events,
        decision,
    } = hooks.run_permission_request(request).await;
    emit_hook_completed_events(sess, turn_context, hook_events).await;

    decision
}

/// Runs matching `PostToolUse` hooks after a tool has produced a successful output.
///
/// The `tool_name`, matcher aliases, `tool_input`, and `tool_response` values are
/// already adapted by the tool handler into the stable hook contract. Passing
/// raw internal tool data here would leak implementation details into user hook
/// matchers and hook logs.
pub(crate) async fn run_post_tool_use_hooks(
    sess: &Arc<Session>,
    step_context: &StepContext,
    tool_use_id: String,
    tool_name: String,
    matcher_aliases: Vec<String>,
    tool_input: Value,
    tool_response: Value,
) -> PostToolUseOutcome {
    let turn_context = &step_context.turn;
    let request = PostToolUseRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        subagent: thread_spawn_subagent_hook_context(sess, turn_context),
        #[allow(deprecated)]
        cwd: turn_context.cwd.clone(),
        transcript_path: sess.hook_transcript_path().await,
        model: step_context.settings.model_info.slug.clone(),
        permission_mode: hook_permission_mode(step_context.settings.approval_policy()),
        tool_name,
        matcher_aliases,
        tool_use_id,
        tool_input,
        tool_response,
    };
    let hooks = sess.hooks();
    let preview_runs = hooks.preview_post_tool_use(&request);
    emit_hook_started_events(sess, turn_context, preview_runs).await;

    let outcome = hooks.run_post_tool_use(request).await;
    emit_hook_completed_events(sess, turn_context, outcome.hook_events.clone()).await;
    outcome
}

fn executor_hook_sources_for_step(step_context: &StepContext) -> Vec<ExecutorPluginHookSource> {
    step_context
        .executor_capability_discovery
        .as_deref()
        .map(|snapshot| {
            executor_plugin_hook_sources(snapshot, |server, tool| {
                step_context.mcp.tool_info(server, tool)
            })
            .into_iter()
            .filter(|source| {
                !step_context
                    .turn
                    .disabled_plugin_ids
                    .contains(&source.plugin_id.as_key())
            })
            .collect()
        })
        .unwrap_or_default()
}

fn build_request_metadata(
    step_context: Option<&StepContext>,
    turn_context: &TurnContext,
) -> Map<String, Value> {
    let settings = step_context
        .map(|step_context| step_context.settings.as_ref())
        .unwrap_or(turn_context.initial_settings.as_ref());
    turn_context
        .turn_metadata_state
        .current_meta_value_for_mcp_request(ExecutionMetadata::from_settings(settings))
        .map(|turn_metadata| {
            Map::from_iter([(
                crate::X_GENEX_TURN_METADATA_HEADER.to_string(),
                turn_metadata,
            )])
        })
        .unwrap_or_default()
}

#[instrument(level = "trace", skip_all)]
pub(crate) async fn run_turn_stop_hooks(
    sess: &Arc<Session>,
    step_context: &Arc<StepContext>,
    stop_hook_active: bool,
    last_assistant_message: Option<String>,
) -> StopOutcome {
    let turn_context = &step_context.turn;
    // Resolve the stop hook kind from the session source before building the
    // request. Root turns run Stop; thread-spawned child turns run SubagentStop.
    let (target, transcript_path) = match &turn_context.session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            agent_role,
            parent_thread_id,
            ..
        }) => {
            let context = subagent_hook_context(sess, agent_role);
            let agent_transcript_path = sess.hook_transcript_path().await;
            let parent_transcript_path = match sess
                .services
                .thread_store
                .read_thread(ReadThreadParams {
                    thread_id: *parent_thread_id,
                    include_archived: true,
                    include_history: false,
                })
                .await
            {
                Ok(thread) => thread.rollout_path,
                Err(error) => {
                    tracing::warn!(
                        parent_thread_id = %parent_thread_id,
                        error = %error,
                        "failed to resolve parent transcript path for subagent hook"
                    );
                    None
                }
            };
            (
                StopHookTarget::SubagentStop {
                    agent_id: context.agent_id,
                    agent_type: context.agent_type,
                    agent_transcript_path,
                },
                parent_transcript_path,
            )
        }
        // Internal/synthetic subagents do not expose user-configured lifecycle
        // hooks, so there is no Stop or SubagentStop request to dispatch.
        SessionSource::SubAgent(_) => return StopOutcome::default(),
        _ => (StopHookTarget::Stop, sess.hook_transcript_path().await),
    };
    let request_metadata = build_request_metadata(Some(step_context), turn_context);
    let request = genex_hooks::StopRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        #[allow(deprecated)]
        cwd: turn_context.cwd.clone(),
        transcript_path,
        model: turn_context.model_info().slug.clone(),
        permission_mode: hook_permission_mode(turn_context.approval_policy()),
        request_metadata: (!request_metadata.is_empty()).then_some(request_metadata),
        stop_hook_active,
        last_assistant_message,
        target,
    };
    let executor_hook_sources = executor_hook_sources_for_step(step_context);
    let hooks = sess.hooks().with_executor_hooks(executor_hook_sources);
    emit_hook_started_events(sess, turn_context, hooks.preview_stop(&request)).await;

    let mut outcome = hooks.run_stop(request).await;
    emit_hook_completed_events(sess, turn_context, std::mem::take(&mut outcome.hook_events)).await;
    outcome
}

#[instrument(level = "trace", skip_all)]
pub(crate) async fn run_session_end_hooks(sess: &Arc<Session>) {
    let hooks = sess.hooks();
    let preview_runs = hooks.preview_session_end();
    if preview_runs.is_empty() {
        return;
    }

    let turn_context = sess.new_default_turn().await;

    // SessionEnd is root-only; ThreadSpawn uses SubagentStart/SubagentStop and other subagents
    // are internal implementation details.
    if matches!(&turn_context.session_source, SessionSource::SubAgent(_)) {
        return;
    }

    let request = genex_hooks::SessionEndRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        #[allow(deprecated)]
        cwd: turn_context.cwd.clone(),
        transcript_path: sess.hook_transcript_path().await,
    };
    if let Err(err) = sess.flush_rollout().await {
        tracing::warn!("failed to flush transcript before SessionEnd hook: {err}");
    }
    emit_hook_started_events(sess, &turn_context, preview_runs).await;

    let outcome = hooks.run_session_end(request).await;
    emit_hook_completed_events(sess, &turn_context, outcome.hook_events).await;
}

pub(crate) async fn run_turn_interrupt_hooks(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    turn_state: &Mutex<TurnState>,
) {
    if matches!(&turn_context.session_source, SessionSource::SubAgent(_)) {
        return;
    }

    // The active turn has already been detached. Reuse only its last executing step's discovery.
    let last_known_step_context = turn_state.lock().await.last_known_step_context.clone();
    let executor_hook_sources = last_known_step_context
        .as_deref()
        .map(executor_hook_sources_for_step)
        .unwrap_or_default();
    let has_executor_hooks = !executor_hook_sources.is_empty();
    let hooks = sess.hooks().with_executor_hooks(executor_hook_sources);
    let preview_runs = hooks.preview_interrupt();
    if preview_runs.is_empty() && !has_executor_hooks {
        return;
    }

    let request_metadata = build_request_metadata(last_known_step_context.as_deref(), turn_context);
    let request = InterruptRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        #[allow(deprecated)]
        cwd: turn_context.cwd.clone(),
        transcript_path: sess.hook_transcript_path().await,
        model: turn_context.model_info().slug.clone(),
        permission_mode: hook_permission_mode(turn_context.approval_policy()),
        request_metadata: (!request_metadata.is_empty()).then_some(request_metadata),
    };
    if let Err(err) = sess.flush_rollout().await {
        tracing::warn!("failed to flush transcript before Interrupt hook: {err}");
    }
    emit_hook_started_events(sess, turn_context, preview_runs).await;

    let outcome = hooks.run_interrupt(request).await;
    emit_hook_completed_events(sess, turn_context, outcome.hook_events).await;
}

pub(crate) async fn run_pre_compact_hooks(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    trigger: CompactionTrigger,
) -> PreCompactHookOutcome {
    let request = genex_hooks::PreCompactRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        subagent: thread_spawn_subagent_hook_context(sess, turn_context),
        #[allow(deprecated)]
        cwd: turn_context.cwd.clone(),
        transcript_path: sess.hook_transcript_path().await,
        model: turn_context.model_info().slug.clone(),
        trigger: compaction_trigger_label(trigger).to_string(),
    };
    let preview_runs = sess.hooks().preview_pre_compact(&request);
    emit_hook_started_events(sess, turn_context, preview_runs).await;

    let outcome = sess.hooks().run_pre_compact(request).await;
    emit_hook_completed_events(sess, turn_context, outcome.hook_events).await;
    if outcome.should_stop {
        PreCompactHookOutcome::Stopped
    } else {
        PreCompactHookOutcome::Continue
    }
}

pub(crate) enum PreCompactHookOutcome {
    Continue,
    Stopped,
}

pub(crate) enum PostCompactHookOutcome {
    Continue,
    Stopped,
}

pub(crate) async fn run_post_compact_hooks(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    trigger: CompactionTrigger,
) -> PostCompactHookOutcome {
    let request = genex_hooks::PostCompactRequest {
        session_id: sess.session_id().into(),
        turn_id: turn_context.sub_id.clone(),
        subagent: thread_spawn_subagent_hook_context(sess, turn_context),
        #[allow(deprecated)]
        cwd: turn_context.cwd.clone(),
        transcript_path: sess.hook_transcript_path().await,
        model: turn_context.model_info().slug.clone(),
        trigger: compaction_trigger_label(trigger).to_string(),
    };
    let preview_runs = sess.hooks().preview_post_compact(&request);
    emit_hook_started_events(sess, turn_context, preview_runs).await;

    let outcome = sess.hooks().run_post_compact(request).await;
    emit_hook_completed_events(sess, turn_context, outcome.hook_events).await;
    if outcome.should_stop {
        PostCompactHookOutcome::Stopped
    } else {
        PostCompactHookOutcome::Continue
    }
}

#[instrument(level = "trace", skip_all)]
pub(crate) async fn run_legacy_after_agent_hook(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    input: &[ResponseItem],
    last_assistant_message: Option<String>,
) -> bool {
    let mut abort_message = None;
    let input_messages = input
        .iter()
        .filter_map(|item| match parse_turn_item(item) {
            Some(TurnItem::UserMessage(user_message)) => Some(user_message.message()),
            _ => None,
        })
        .collect();
    let hooks = sess.hooks();
    for hook_outcome in hooks
        .dispatch(genex_hooks::HookPayload {
            session_id: sess.session_id().into(),
            #[allow(deprecated)]
            cwd: turn_context.cwd.clone(),
            client: turn_context.app_server_client_name.clone(),
            triggered_at: chrono::Utc::now(),
            hook_event: genex_hooks::HookEvent::AfterAgent {
                event: genex_hooks::HookEventAfterAgent {
                    thread_id: sess.thread_id,
                    turn_id: turn_context.sub_id.clone(),
                    input_messages,
                    last_assistant_message,
                },
            },
        })
        .await
    {
        let hook_name = hook_outcome.hook_name;
        let (error, should_abort) = match hook_outcome.result {
            genex_hooks::HookResult::Success => continue,
            genex_hooks::HookResult::FailedContinue(error) => (error, false),
            genex_hooks::HookResult::FailedAbort(error) => (error, true),
        };
        let action = if should_abort {
            "aborting operation"
        } else {
            "continuing"
        };
        tracing::warn!(
            turn_id = %turn_context.sub_id,
            hook_name = %hook_name,
            error = %error,
            "after_agent hook failed; {action}"
        );
        if should_abort && abort_message.is_none() {
            abort_message = Some(format!(
                "after_agent hook '{hook_name}' failed and aborted turn completion: {error}"
            ));
        }
    }
    let Some(message) = abort_message else {
        return false;
    };
    let event = EventMsg::Error(genex_protocol::protocol::ErrorEvent {
        misalignment: None,
        message,
        genex_error_info: Some(GenexErrorInfo::Other),
    });
    sess.send_event(turn_context, event).await;
    true
}

pub(crate) async fn inspect_pending_input(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    pending_input_item: &TurnInput,
) -> HookRuntimeOutcome {
    match pending_input_item {
        TurnInput::UserInput { content, .. } => {
            let request = UserPromptSubmitRequest {
                session_id: sess.session_id().into(),
                turn_id: turn_context.sub_id.clone(),
                subagent: thread_spawn_subagent_hook_context(sess, turn_context),
                #[allow(deprecated)]
                cwd: turn_context.cwd.clone(),
                transcript_path: sess.hook_transcript_path().await,
                model: turn_context.model_info().slug.clone(),
                permission_mode: hook_permission_mode(turn_context.approval_policy()),
                prompt: UserMessageItem::new(content).message(),
            };
            let hooks = sess.hooks();
            let preview_runs = hooks.preview_user_prompt_submit(&request);
            run_context_injecting_hook(
                sess,
                turn_context,
                preview_runs,
                hooks.run_user_prompt_submit(request),
            )
            .await
        }
        TurnInput::ResponseItem(_) | TurnInput::FunctionCallOutput(_) => HookRuntimeOutcome {
            should_stop: false,
            additional_contexts: Vec::new(),
        },
        TurnInput::InterAgentCommunication(_) => HookRuntimeOutcome {
            should_stop: false,
            additional_contexts: Vec::new(),
        },
    }
}

pub(crate) async fn record_pending_input(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    model_info: &ModelInfo,
    pending_input: TurnInput,
    additional_contexts: Vec<String>,
    persist_context: PersistContext,
) {
    match pending_input {
        TurnInput::UserInput {
            content,
            client_id,
            acceptance_order,
        } => {
            sess.record_user_prompt_and_emit_turn_item(
                turn_context.as_ref(),
                model_info,
                content.as_slice(),
                client_id,
                acceptance_order,
                persist_context,
            )
            .await;
        }
        TurnInput::ResponseItem(item) => {
            sess.record_annotated_conversation_items(turn_context, model_info, vec![item])
                .await;
        }
        TurnInput::FunctionCallOutput(item) => {
            sess.record_annotated_conversation_items(turn_context, model_info, vec![item.clone()])
                .await;
            if let ResponseItem::FunctionCallOutput {
                id: Some(id),
                name: Some(name),
                namespace,
                output,
                ..
            } = item.item
            {
                let item = TurnItem::FunctionCallOutput(FunctionCallOutputItem {
                    id: id.to_string(),
                    name,
                    namespace,
                    output: output.body,
                });
                sess.emit_turn_item_started(turn_context, &item).await;
                sess.emit_turn_item_completed(turn_context, item).await;
            }
            sess.ensure_rollout_materialized(persist_context).await;
        }
        TurnInput::InterAgentCommunication(communication) => {
            sess.record_inter_agent_communication(turn_context, model_info, communication)
                .await;
        }
    }
    record_additional_contexts(sess, turn_context, additional_contexts).await;
}

/// Processes finished async hook results at a safe turn boundary.
///
/// Before the user prompt, records additional context directly into conversation
/// history so results from a previous turn appear before the new prompt. After
/// sampling, injects context into the active turn's pending-input queue so it
/// reaches the next sampling request. Warnings and telemetry are handled in both
/// cases.
pub(crate) async fn drain_async_hook_results(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    before_user_prompt: bool,
) {
    while let Ok(result) = sess.async_hook_results.try_recv() {
        let additional_contexts = result
            .run
            .entries
            .iter()
            .filter(|entry| entry.kind == HookOutputEntryKind::Context)
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>();

        if before_user_prompt {
            record_additional_contexts(sess, turn_context, additional_contexts).await;
        } else if !additional_contexts.is_empty() {
            let _ = sess
                .inject_hook_context_if_running(additional_context_messages(additional_contexts))
                .await;
        }

        for entry in &result.run.entries {
            if entry.kind == HookOutputEntryKind::Warning {
                sess.send_event(
                    turn_context,
                    EventMsg::Warning(WarningEvent {
                        message: entry.text.clone(),
                    }),
                )
                .await;
            }
        }

        emit_hook_completed_events(sess, turn_context, vec![result]).await;
    }
}

async fn run_context_injecting_hook<Fut, Outcome>(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    preview_runs: Vec<HookRunSummary>,
    outcome_future: Fut,
) -> HookRuntimeOutcome
where
    Fut: Future<Output = Outcome>,
    Outcome: Into<ContextInjectingHookOutcome>,
{
    emit_hook_started_events(sess, turn_context, preview_runs).await;

    let outcome = outcome_future.await.into();
    emit_hook_completed_events(sess, turn_context, outcome.hook_events).await;
    outcome.outcome
}

impl HookRuntimeOutcome {
    async fn record_additional_contexts(
        self,
        sess: &Arc<Session>,
        turn_context: &Arc<TurnContext>,
    ) -> bool {
        record_additional_contexts(sess, turn_context, self.additional_contexts).await;

        self.should_stop
    }
}

pub(crate) async fn record_additional_contexts(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    additional_contexts: Vec<String>,
) {
    let developer_messages = additional_context_messages(additional_contexts);
    if developer_messages.is_empty() {
        return;
    }

    sess.record_conversation_items(
        turn_context,
        turn_context.model_info(),
        developer_messages.as_slice(),
    )
    .await;
}

fn additional_context_messages(additional_contexts: Vec<String>) -> Vec<ResponseItem> {
    additional_contexts
        .into_iter()
        .map(HookAdditionalContext::new)
        .map(ContextualUserFragment::into)
        .collect()
}

fn should_emit_hook_notification(run: &HookRunSummary) -> bool {
    !run.builtin && run.execution_mode == HookExecutionMode::Sync
}

async fn emit_hook_started_events(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    preview_runs: Vec<HookRunSummary>,
) {
    for run in preview_runs
        .into_iter()
        .filter(should_emit_hook_notification)
    {
        sess.send_event(
            turn_context,
            EventMsg::HookStarted(HookStartedEvent {
                turn_id: Some(turn_context.sub_id.clone()),
                run,
            }),
        )
        .await;
    }
}

pub(crate) async fn emit_hook_completed_events(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
    completed_events: Vec<HookCompletedEvent>,
) {
    for completed in completed_events {
        emit_hook_completed_metrics(turn_context, &completed);
        if should_emit_hook_notification(&completed.run) {
            sess.send_event(turn_context, EventMsg::HookCompleted(completed))
                .await;
        }
    }
}

fn emit_hook_completed_metrics(turn_context: &TurnContext, completed: &HookCompletedEvent) {
    let tags = hook_run_metric_tags(&completed.run);
    turn_context
        .session_telemetry
        .counter(HOOK_RUN_METRIC, /*inc*/ 1, &tags);
    if let Some(duration_ms) = completed.run.duration_ms
        && let Ok(duration_ms) = u64::try_from(duration_ms)
    {
        turn_context.session_telemetry.record_duration(
            HOOK_RUN_DURATION_METRIC,
            Duration::from_millis(duration_ms),
            &tags,
        );
    }
}

fn hook_run_metric_tags(run: &HookRunSummary) -> [(&'static str, &'static str); 5] {
    let hook_name = match run.event_name {
        HookEventName::PreToolUse => "PreToolUse",
        HookEventName::PermissionRequest => "PermissionRequest",
        HookEventName::PostToolUse => "PostToolUse",
        HookEventName::PreCompact => "PreCompact",
        HookEventName::PostCompact => "PostCompact",
        HookEventName::SessionStart => "SessionStart",
        HookEventName::SessionEnd => "SessionEnd",
        HookEventName::UserPromptSubmit => "UserPromptSubmit",
        HookEventName::SubagentStart => "SubagentStart",
        HookEventName::SubagentStop => "SubagentStop",
        HookEventName::Stop => "Stop",
        HookEventName::Interrupt => "Interrupt",
    };
    let hook_source = match run.source {
        HookSource::System => "system",
        HookSource::User => "user",
        HookSource::Project => "project",
        HookSource::Mdm => "mdm",
        HookSource::SessionFlags => "session_flags",
        HookSource::Plugin => "plugin",
        HookSource::CloudRequirements => "cloud_requirements",
        HookSource::CloudManagedConfig => "cloud_managed_config",
        HookSource::Unknown => "unknown",
    };
    let status = match run.status {
        HookRunStatus::Running => "running",
        HookRunStatus::Completed => "completed",
        HookRunStatus::Failed => "failed",
        HookRunStatus::Blocked => "blocked",
        HookRunStatus::Stopped => "stopped",
    };
    [
        ("hook_name", hook_name),
        ("source", hook_source),
        ("status", status),
        ("handler_type", hook_handler_type_label(run.handler_type)),
        (
            "execution_mode",
            hook_execution_mode_label(run.execution_mode),
        ),
    ]
}

fn hook_permission_mode(approval_policy: AskForApproval) -> String {
    match approval_policy {
        AskForApproval::Never => "bypassPermissions",
        AskForApproval::UnlessTrusted | AskForApproval::OnRequest | AskForApproval::Granular(_) => {
            "default"
        }
    }
    .to_string()
}

fn thread_spawn_subagent_hook_context(
    sess: &Arc<Session>,
    turn_context: &TurnContext,
) -> Option<SubagentHookContext> {
    match &turn_context.session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn { agent_role, .. }) => {
            Some(subagent_hook_context(sess, agent_role))
        }
        _ => None,
    }
}

fn subagent_hook_context(sess: &Arc<Session>, agent_role: &Option<String>) -> SubagentHookContext {
    SubagentHookContext {
        agent_id: sess.thread_id().to_string(),
        agent_type: agent_role
            .clone()
            .unwrap_or_else(|| crate::agent::role::DEFAULT_ROLE_NAME.to_string()),
    }
}

fn compaction_trigger_label(value: CompactionTrigger) -> &'static str {
    match value {
        CompactionTrigger::Manual => "manual",
        CompactionTrigger::Auto => "auto",
    }
}
