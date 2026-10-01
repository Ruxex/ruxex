use super::*;
use crate::app::safety_buffering::SafetyBufferedRetry;
use crate::app::session_lifecycle::ThreadAttachPresentation;
use crate::chatwidget::UserMessage;
use crate::chatwidget::tests::helpers::normalize_completion_timestamps;
use genex_app_server_client::AppServerEvent;
use genex_app_server_protocol::ModelSafetyBufferingUpdatedNotification;
use genex_model_provider_info::ModelProviderInfo;
use genex_protocol::models::ManagedFileSystemPermissions;
use genex_protocol::model_types::ToolMode;
use genex_protocol::permissions::FileSystemAccessMode;
use genex_protocol::permissions::FileSystemPath;
use genex_protocol::permissions::FileSystemSandboxEntry;
use genex_protocol::permissions::FileSystemSpecialPath;
use genex_protocol::permissions::NetworkSandboxPolicy;
use genex_utils_absolute_path::test_support::PathExt;
use core_test_support::responses;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_completed_with_tokens;
use core_test_support::responses::ev_response_created;
use core_test_support::streaming_sse::StreamingSseChunk;
use core_test_support::streaming_sse::StreamingSseServer;
use core_test_support::streaming_sse::start_streaming_sse_server;
use pretty_assertions::assert_eq;
use serde_json::Value;
use tokio::sync::oneshot;

const CURRENT_MODEL: &str = "gpt-5.6-terra";
const FASTER_MODEL: &str = "gpt-5.6-luna";
const MODEL_PROVIDER_ID: &str = "safety-retry-test";
const PREVIOUS_PROMPT: &str = "Establish context";
const RETRY_PROMPT: &str = "Handle the safety-buffered request";
const COMMITTED_STEER: &str = "Keep the accepted steer";
const UNSENT_DRAFT: &str = "Keep this unsent draft";
const RETRY_GOAL: &str = "Preserve this goal across the retry";
const SAFETY_RETRY_THREAD_NAME: &str = "Safety retry source";

#[derive(Clone, Copy, PartialEq, Eq)]
enum SafetyRetryScenario {
    Once,
    RetryTwice,
    InterruptedPrevious,
    UnsupportedPermissions,
}

fn response_chunks(response_id: &str) -> Vec<StreamingSseChunk> {
    [
        ev_response_created(response_id),
        ev_assistant_message(&format!("message-{response_id}"), "done"),
        ev_completed(response_id),
    ]
    .into_iter()
    .map(|event| StreamingSseChunk {
        gate: None,
        body: responses::sse(vec![event]),
    })
    .collect()
}

fn gated_response_chunks(
    response_id: &str,
    completed: Value,
) -> (Vec<StreamingSseChunk>, oneshot::Sender<()>) {
    let (release_tx, release_rx) = oneshot::channel();
    (
        vec![
            StreamingSseChunk {
                gate: None,
                body: responses::sse(vec![ev_response_created(response_id)]),
            },
            StreamingSseChunk {
                gate: Some(release_rx),
                body: responses::sse(vec![completed]),
            },
        ],
        release_tx,
    )
}

fn next_user_turn_event(
    app_event_rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
) -> AppCommand {
    while let Ok(event) = app_event_rx.try_recv() {
        if let AppEvent::GenexOp(turn @ AppCommand::UserTurn { .. }) = event {
            return turn;
        }
    }
    panic!("expected UserTurn app event");
}

fn submit_prompt(app: &mut App, prompt: &str) {
    app.chat_widget.apply_external_edit(prompt.to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

fn drain_active_thread_events(app: &mut App) {
    while let Some(event) = app
        .active_thread_rx
        .as_mut()
        .and_then(|receiver| receiver.try_recv().ok())
    {
        app.handle_thread_event_now(event);
    }
}

async fn next_turn_started(
    app: &mut App,
    app_server: &mut AppServerSession,
    thread_id: ThreadId,
) -> String {
    loop {
        let event = tokio::time::timeout(
            std::time::Duration::from_secs(/*secs*/ 5),
            app_server.next_event(),
        )
        .await
        .expect("app-server should emit a turn/start event")
        .expect("app-server event stream should remain open");
        let started_turn_id = if let AppServerEvent::ServerNotification(notification) = &event
            && let ServerNotification::TurnStarted(notification) = notification.as_ref()
            && notification.thread_id == thread_id.to_string()
        {
            Some(notification.turn.id.clone())
        } else {
            None
        };
        app.handle_app_server_event(app_server, event).await;
        drain_active_thread_events(app);
        if let Some(turn_id) = started_turn_id {
            return turn_id;
        }
    }
}

async fn wait_for_turn_completed(
    app: &mut App,
    app_server: &mut AppServerSession,
    thread_id: ThreadId,
) {
    loop {
        let event = tokio::time::timeout(
            std::time::Duration::from_secs(/*secs*/ 5),
            app_server.next_event(),
        )
        .await
        .expect("app-server should emit a turn/completed event")
        .expect("app-server event stream should remain open");
        let completed = matches!(
            &event,
            AppServerEvent::ServerNotification(notification)
                if matches!(
                    notification.as_ref(),
                    ServerNotification::TurnCompleted(notification)
                        if notification.thread_id == thread_id.to_string()
                )
        );
        app.handle_app_server_event(app_server, event).await;
        drain_active_thread_events(app);
        if completed {
            return;
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_turn_interrupt_is_nonblocking_and_coalesces_repeated_requests() -> Result<()> {
    Box::pin(interrupt_after_inactive_steer(SteerSwitch::WithinTask)).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agents_overview_acknowledges_inactive_steer_before_interrupt() -> Result<()> {
    Box::pin(interrupt_after_inactive_steer(SteerSwitch::BetweenTasks)).await
}

enum SteerSwitch {
    WithinTask,
    BetweenTasks,
}


async fn drive_until_request_count(
    app: &mut App,
    app_server: &mut AppServerSession,
    server: &StreamingSseServer,
    expected_request_count: usize,
) {
    let timeout = tokio::time::sleep(std::time::Duration::from_secs(/*secs*/ 5));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            () = server.wait_for_request_count(expected_request_count) => return,
            event = app_server.next_event() => {
                let event = event.expect("app-server event stream should remain open");
                app.handle_app_server_event(app_server, event).await;
                drain_active_thread_events(app);
            }
            () = &mut timeout => {
                panic!("expected {expected_request_count} Responses API requests");
            }
        }
    }
}

fn user_input_texts(body: &Value) -> Vec<String> {
    body.get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .filter(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .filter_map(|item| item.get("content").and_then(Value::as_array))
        .flatten()
        .filter(|span| span.get("type").and_then(Value::as_str) == Some("input_text"))
        .filter_map(|span| span.get("text").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fn user_message_count(thread: &Thread, prompt: &str) -> usize {
    thread
        .turns
        .iter()
        .flat_map(|turn| &turn.items)
        .filter_map(|item| match item {
            ThreadItem::UserMessage { content, .. } => Some(content),
            _ => None,
        })
        .filter(|content| {
            content
                .iter()
                .filter_map(|item| match item {
                    AppServerUserInput::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<String>()
                == prompt
        })
        .count()
}


#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn safety_retry_forks_after_the_previous_turn_and_uses_faster_settings() -> Result<()> {
    run_safety_retry(
        Some(PREVIOUS_PROMPT),
        /*failing_draft*/ None,
        /*committed_steer*/ None,
        SafetyRetryScenario::Once,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn safety_retry_preserves_a_committed_steer_from_the_interrupted_turn() -> Result<()> {
    run_safety_retry(
        Some(PREVIOUS_PROMPT),
        /*failing_draft*/ None,
        Some(COMMITTED_STEER),
        SafetyRetryScenario::Once,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn safety_retry_forks_first_turn_and_continues_without_duplicating_prompt() -> Result<()> {
    run_safety_retry(
        /*previous_prompt*/ None,
        /*failing_draft*/ None,
        /*committed_steer*/ None,
        SafetyRetryScenario::Once,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn safety_retry_can_retry_a_first_turn_a_second_time() -> Result<()> {
    run_safety_retry(
        /*previous_prompt*/ None,
        /*failing_draft*/ None,
        /*committed_steer*/ None,
        SafetyRetryScenario::RetryTwice,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn safety_retry_replays_older_interruption_notices() -> Result<()> {
    run_safety_retry(
        Some(PREVIOUS_PROMPT),
        /*failing_draft*/ None,
        /*committed_steer*/ None,
        SafetyRetryScenario::InterruptedPrevious,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn safety_retry_branch_failure_preserves_unsent_draft() -> Result<()> {
    run_safety_retry(
        Some(PREVIOUS_PROMPT),
        Some(UNSENT_DRAFT),
        /*committed_steer*/ None,
        SafetyRetryScenario::Once,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn safety_retry_rejects_unsupported_permissions_before_interrupting() -> Result<()> {
    run_safety_retry(
        /*previous_prompt*/ None,
        /*failing_draft*/ None,
        /*committed_steer*/ None,
        SafetyRetryScenario::UnsupportedPermissions,
    )
    .await
}
