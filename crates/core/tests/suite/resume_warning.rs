#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use core::time::Duration;
use core_test_support::load_default_config_for_test;
use core_test_support::thread_manager_with_model_provider_info;
use core_test_support::wait_for_event;
use ruxex_core::NewThread;
use ruxex_history::InitialHistory;
use ruxex_history::ResumedHistory;
use ruxex_history::RolloutItem;
use ruxex_login::RuxexAuth;
use ruxex_protocol::ThreadId;
use ruxex_protocol::config_types::ModeKind;
use ruxex_protocol::config_types::ReasoningSummary;
use ruxex_protocol::mcp::ClientMcpExtensions;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::TurnCompleteEvent;
use ruxex_protocol::protocol::TurnContextItem;
use ruxex_protocol::protocol::TurnStartedEvent;
use ruxex_protocol::protocol::UserMessageEvent;
use ruxex_protocol::protocol::WarningEvent;
use tempfile::TempDir;

fn resume_history(
    config: &ruxex_core::config::Config,
    previous_model: &str,
    rollout_path: &std::path::Path,
) -> InitialHistory {
    let turn_id = "resume-warning-seed-turn".to_string();
    let turn_ctx = TurnContextItem {
        turn_id: Some(turn_id.clone()),
        root_turn_id: None,
        disabled_plugin_ids: None,
        cwd: config.cwd.clone(),
        workspace_roots: None,
        current_date: None,
        timezone: None,
        approval_policy: config.permissions.approval_policy.value(),
        approvals_reviewer: None,
        sandbox_policy: config.legacy_sandbox_policy(),
        permission_profile: None,
        active_permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        model: previous_model.to_string(),
        comp_hash: None,
        collaboration_mode: None,
        multi_agent_version: None,
        multi_agent_mode: None,
        realtime_active: None,
        cyber_access_program: None,
        effort: config.model_reasoning_effort.clone(),
        summary: config
            .model_reasoning_summary
            .unwrap_or(ReasoningSummary::Auto),
    };

    InitialHistory::Resumed(ResumedHistory {
        conversation_id: ThreadId::default(),
        history: Arc::new(vec![
            RolloutItem::EventMsg(EventMsg::TurnStarted(TurnStartedEvent {
                turn_id: turn_id.clone(),
                root_turn_id: None,
                trace_id: None,
                started_at: None,
                model_context_window: None,
                collaboration_mode_kind: ModeKind::Default,
            })),
            RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "seed".to_string(),
                images: None,
                local_images: vec![],
                text_elements: vec![],
                ..Default::default()
            })),
            RolloutItem::TurnContext(turn_ctx),
            RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
                turn_id,
                last_agent_message: None,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            })),
        ]),
        rollout_path: Some(rollout_path.to_path_buf()),
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn emits_warning_when_resumed_model_differs() {
    // Arrange a config with a current model and a prior rollout recorded under a different model.
    let home = TempDir::new().expect("tempdir");
    let mut config = load_default_config_for_test(&home).await;
    config.model = Some("current-model".to_string());
    // Ensure cwd is absolute (the helper sets it to the temp dir already).
    assert!(config.cwd.is_absolute());

    let rollout_path = home.path().join("rollout.jsonl");
    std::fs::write(&rollout_path, "").expect("create rollout placeholder");

    let initial_history = resume_history(&config, "previous-model", &rollout_path);

    let thread_manager = thread_manager_with_model_provider_info(
        RuxexAuth::from_api_key("test"),
        config.model_provider.clone(),
    );
    // Act: resume the conversation.
    let NewThread {
        thread: conversation,
        ..
    } = thread_manager
        .resume_thread_with_history(
            config.clone(),
            initial_history,
            /*parent_trace*/ None,
            ClientMcpExtensions::default(),
        )
        .await
        .expect("resume conversation");

    // Assert: a Warning event is emitted describing the model mismatch.
    let warning = wait_for_event(&conversation, |ev| {
        matches!(
            ev,
            EventMsg::Warning(WarningEvent { message })
                if message.contains("previous-model") && message.contains("current-model")
        )
    })
    .await;
    let EventMsg::Warning(WarningEvent { message }) = warning else {
        panic!("expected warning event");
    };
    assert!(message.contains("previous-model"));
    assert!(message.contains("current-model"));

    // Drain the TurnComplete/Shutdown window to avoid leaking tasks between tests.
    // The warning is emitted during initialization, so a short sleep is sufficient.
    tokio::time::sleep(Duration::from_millis(50)).await;
}
