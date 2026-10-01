use super::*;
use pretty_assertions::assert_eq;


#[tokio::test]
async fn replayed_command_completion_preserves_tracking_without_duplicate_starts() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();
    let mut item =
        begin_unified_exec_startup(&mut chat, "call-replay", "process-replay", "cat replay");
    if let AppServerThreadItem::CommandExecution { status, .. } = &mut item {
        *status = AppServerCommandExecutionStatus::Completed;
    }

    chat.handle_server_notification(
        ServerNotification::ItemCompleted(ItemCompletedNotification {
            thread_id: String::new(),
            turn_id: "turn-1".to_string(),
            completed_at_ms: 0,
            item,
        }),
        Some(ReplayKind::ThreadSnapshot),
    );

    assert!(chat.running_commands.is_empty());
    assert!(chat.unified_exec_processes.is_empty());
    let history = drain_insert_history(&mut rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<Vec<_>>();
    assert_eq!(
        history,
        vec!["• Ran cat replay\n  └ (no output)\n".to_string()]
    );
}

#[tokio::test]
async fn replayed_completion_preserves_unrelated_running_command() {
    for active_mcp in [false, true] {
        let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        chat.on_task_started();
        let mut completed = begin_exec(&mut chat, "call-running", "sleep 5");
        if active_mcp {
            chat.transcript.active_cell = Some(Box::new(history_cell::new_active_mcp_tool_call(
                "mcp-running".to_string(),
                McpInvocation {
                    server: "server".to_string(),
                    tool: "tool".to_string(),
                    arguments: None,
                },
                /*animations_enabled*/ false,
            )));
        }
        if let AppServerThreadItem::CommandExecution {
            id,
            command,
            status,
            ..
        } = &mut completed
        {
            *id = "call-completed".to_string();
            *command = "printf completed".to_string();
            *status = AppServerCommandExecutionStatus::Completed;
        }

        chat.replay_thread_item(completed, "turn-1".to_string(), ReplayKind::ThreadSnapshot);

        assert_eq!(drain_insert_history(&mut rx).len(), 1);
        assert!(active_blob(&chat).contains(if active_mcp {
            "Calling"
        } else {
            "Running sleep 5"
        }));
    }
}




#[test]
fn app_server_exec_approval_request_splits_shell_wrapped_command() {
    let script = r#"python3 -c 'print("Hello, world!")'"#;
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            kind: Default::default(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: None,
            command: Some(
                shlex::try_join(["/bin/zsh", "-lc", script])
                    .expect("round-trippable shell wrapper"),
            ),
            cwd: Some(test_path_buf("/tmp").abs().into()),
            command_actions: None,
            additional_permissions: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: None,
        },
        &test_path_buf("/tmp").abs(),
    );

    assert_eq!(
        request.command,
        vec![
            "/bin/zsh".to_string(),
            "-lc".to_string(),
            script.to_string(),
        ]
    );
}

#[tokio::test]
async fn exec_approval_uses_approval_id_when_present() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_exec_approval_request(
        &mut chat,
        "sub-short",
        ExecApprovalRequestEvent {
            kind: Default::default(),
            call_id: "call-parent".into(),
            approval_id: Some("approval-subcommand".into()),
            turn_id: "turn-short".into(),
            environment_id: None,
            command: vec!["bash".into(), "-lc".into(), "echo hello world".into()],
            cwd: AbsolutePathBuf::current_dir().expect("current dir"),
            reason: Some(
                "this is a test reason such as one that would be produced by the model".into(),
            ),
            network_approval_context: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            additional_permissions: None,
            available_decisions: None,
        },
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let mut found = false;
    while let Ok(app_ev) = rx.try_recv() {
        if let AppEvent::SubmitThreadOp {
            op: Op::ExecApproval { id, decision, .. },
            ..
        } = app_ev
        {
            assert_eq!(id, "approval-subcommand");
            assert_matches!(
                decision,
                codex_app_server_protocol::CommandExecutionApprovalDecision::Accept
            );
            found = true;
            break;
        }
    }
    assert!(found, "expected ExecApproval op to be sent");
}



#[tokio::test]
async fn unified_exec_begin_restores_status_indicator_after_preamble() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.on_task_started();
    assert_eq!(chat.bottom_pane.status_indicator_visible(), true);

    // Simulate a hidden status row during an active turn.
    chat.bottom_pane.hide_status_indicator();
    assert_eq!(chat.bottom_pane.status_indicator_visible(), false);
    assert_eq!(chat.bottom_pane.is_task_running(), true);

    begin_unified_exec_startup(&mut chat, "call-1", "proc-1", "sleep 2");

    assert_eq!(chat.bottom_pane.status_indicator_visible(), true);
}


#[tokio::test]
async fn exec_history_cell_shows_working_then_completed() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // Begin command
    let begin = begin_exec(&mut chat, "call-1", "echo done");

    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 0, "no exec cell should have been flushed yet");

    // End command successfully
    end_exec(&mut chat, begin, "done", "", /*exit_code*/ 0);

    let cells = drain_insert_history(&mut rx);
    // Exec end now finalizes and flushes the exec cell immediately.
    assert_eq!(cells.len(), 1, "expected finalized exec cell to flush");
    // Inspect the flushed exec cell rendering.
    let lines = &cells[0];
    let blob = lines_to_single_string(lines);
    // New behavior: no glyph markers; ensure command is shown and no panic.
    assert!(
        blob.contains("• Ran"),
        "expected summary header present: {blob:?}"
    );
    assert!(
        blob.contains("echo done"),
        "expected command text to be present: {blob:?}"
    );
}

#[tokio::test]
async fn exec_history_cell_shows_working_then_failed() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // Begin command
    let begin = begin_exec(&mut chat, "call-2", "false");
    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 0, "no exec cell should have been flushed yet");

    // End command with failure
    end_exec(&mut chat, begin, "", "Bloop", /*exit_code*/ 2);

    let cells = drain_insert_history(&mut rx);
    // Exec end with failure should also flush immediately.
    assert_eq!(cells.len(), 1, "expected finalized exec cell to flush");
    let lines = &cells[0];
    let blob = lines_to_single_string(lines);
    assert!(
        blob.contains("• Ran false"),
        "expected command and header text present: {blob:?}"
    );
    assert!(blob.to_lowercase().contains("bloop"), "expected error text");
}

#[tokio::test]
async fn exec_end_without_begin_uses_event_command() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let command = vec![
        "bash".to_string(),
        "-lc".to_string(),
        "echo orphaned".to_string(),
    ];
    let command_actions = codex_shell_command::parse_command::parse_command(&command)
        .into_iter()
        .map(|parsed| AppServerCommandAction::from_core_with_cwd(parsed, &chat.config.cwd))
        .collect();
    let cwd = chat.config.cwd.clone();
    handle_exec_end(
        &mut chat,
        AppServerThreadItem::CommandExecution {
            model_context: None,
            id: "call-orphan".to_string(),
            command: codex_shell_command::parse_command::shlex_join(&command),
            cwd: cwd.into(),
            process_id: None,
            plugin_id: None,
            script_path: None,
            source: ExecCommandSource::Agent,
            status: AppServerCommandExecutionStatus::Completed,
            command_actions,
            aggregated_output: Some("done".to_string()),
            exit_code: Some(0),
            duration_ms: Some(5),
        },
    );

    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 1, "expected finalized exec cell to flush");
    let blob = lines_to_single_string(&cells[0]);
    assert!(
        blob.contains("• Ran echo orphaned"),
        "expected command text to come from event: {blob:?}"
    );
    assert!(
        !blob.contains("call-orphan"),
        "call id should not be rendered when event has the command: {blob:?}"
    );
}

#[tokio::test]
async fn exec_end_without_begin_does_not_flush_unrelated_running_exploring_cell() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();

    begin_exec(&mut chat, "call-exploring", "cat /dev/null");
    assert!(drain_insert_history(&mut rx).is_empty());
    assert!(active_blob(&chat).contains("Read null"));

    let orphan =
        begin_unified_exec_startup(&mut chat, "call-orphan", "proc-1", "echo repro-marker");
    assert!(drain_insert_history(&mut rx).is_empty());

    end_exec(
        &mut chat,
        orphan,
        "repro-marker\n",
        "",
        /*exit_code*/ 0,
    );

    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 1, "only the orphan end should be inserted");
    let orphan_blob = lines_to_single_string(&cells[0]);
    assert!(
        orphan_blob.contains("• Ran echo repro-marker"),
        "expected orphan end to render a standalone entry: {orphan_blob:?}"
    );
    let active = active_blob(&chat);
    assert!(
        active.contains("• Exploring"),
        "expected unrelated exploring call to remain active: {active:?}"
    );
    assert!(
        active.contains("Read null"),
        "expected active exploring command to remain visible: {active:?}"
    );
    assert!(
        !active.contains("echo repro-marker"),
        "orphaned end should not replace the active exploring cell: {active:?}"
    );
}

#[tokio::test]
async fn exec_end_without_begin_flushes_completed_unrelated_exploring_cell() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();

    let begin_ls = begin_exec(&mut chat, "call-ls", "ls -la");
    end_exec(&mut chat, begin_ls, "", "", /*exit_code*/ 0);
    assert!(drain_insert_history(&mut rx).is_empty());
    assert!(active_blob(&chat).contains("ls -la"));

    let orphan = begin_unified_exec_startup(&mut chat, "call-after", "proc-1", "echo after");
    end_exec(&mut chat, orphan, "after\n", "", /*exit_code*/ 0);

    let cells = drain_insert_history(&mut rx);
    assert_eq!(
        cells.len(),
        2,
        "completed exploring cell should flush before the orphan entry"
    );
    let first = lines_to_single_string(&cells[0]);
    let second = lines_to_single_string(&cells[1]);
    assert!(
        first.contains("• Explored"),
        "expected flushed exploring cell: {first:?}"
    );
    assert!(
        first.contains("List ls -la"),
        "expected flushed exploring cell: {first:?}"
    );
    assert!(
        second.contains("• Ran echo after"),
        "expected orphan end entry after flush: {second:?}"
    );
    assert!(
        chat.transcript.active_cell.is_none(),
        "both entries should be finalized"
    );
}

#[tokio::test]
async fn overlapping_exploring_exec_end_is_not_misclassified_as_orphan() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let begin_ls = begin_exec(&mut chat, "call-ls", "ls -la");
    let begin_cat = begin_exec(&mut chat, "call-cat", "cat foo.txt");
    assert!(drain_insert_history(&mut rx).is_empty());

    end_exec(&mut chat, begin_ls, "foo.txt\n", "", /*exit_code*/ 0);

    let cells = drain_insert_history(&mut rx);
    assert!(
        cells.is_empty(),
        "tracked end inside an exploring cell should not render as an orphan"
    );
    let active = active_blob(&chat);
    assert!(
        active.contains("List ls -la"),
        "expected first command still grouped: {active:?}"
    );
    assert!(
        active.contains("Read foo.txt"),
        "expected second running command to stay in the same active cell: {active:?}"
    );
    assert!(
        active.contains("• Exploring"),
        "expected grouped exploring header to remain active: {active:?}"
    );

    end_exec(&mut chat, begin_cat, "hello\n", "", /*exit_code*/ 0);
}

#[tokio::test]
async fn exec_history_shows_unified_exec_startup_commands() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();

    let begin = begin_exec_with_source(
        &mut chat,
        "call-startup",
        "echo unified exec startup",
        ExecCommandSource::UnifiedExecStartup,
    );
    assert!(
        drain_insert_history(&mut rx).is_empty(),
        "exec begin should not flush until completion"
    );

    end_exec(
        &mut chat,
        begin,
        "echo unified exec startup\n",
        "",
        /*exit_code*/ 0,
    );

    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 1, "expected finalized exec cell to flush");
    let blob = lines_to_single_string(&cells[0]);
    assert!(
        blob.contains("• Ran echo unified exec startup"),
        "expected startup command to render: {blob:?}"
    );
}

#[tokio::test]
async fn exec_history_shows_unified_exec_tool_calls() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();

    let begin = begin_exec_with_source(
        &mut chat,
        "call-startup",
        "ls",
        ExecCommandSource::UnifiedExecStartup,
    );
    end_exec(&mut chat, begin, "", "", /*exit_code*/ 0);

    let blob = active_blob(&chat);
    assert_eq!(blob, "• Explored\n  └ List ls\n");
}


#[tokio::test]
async fn unified_exec_end_after_task_complete_is_suppressed() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();

    let begin = begin_exec_with_source(
        &mut chat,
        "call-startup",
        "echo unified exec startup",
        ExecCommandSource::UnifiedExecStartup,
    );
    drain_insert_history(&mut rx);

    chat.on_task_complete(
        /*last_agent_message*/ None, /*completion*/ None, /*from_replay*/ false,
    );
    end_exec(&mut chat, begin, "", "", /*exit_code*/ 0);

    let cells = drain_insert_history(&mut rx);
    assert!(
        cells.is_empty(),
        "expected unified exec end after task complete to be suppressed"
    );
}

#[tokio::test]
async fn unified_exec_interaction_after_task_complete_is_suppressed() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();
    chat.on_task_complete(
        /*last_agent_message*/ None, /*completion*/ None, /*from_replay*/ false,
    );

    terminal_interaction(&mut chat, "call-1", "proc-1", "ls\n");

    let cells = drain_insert_history(&mut rx);
    assert!(
        cells.is_empty(),
        "expected unified exec interaction after task complete to be suppressed"
    );
}




#[tokio::test]
async fn unified_exec_wait_status_header_updates_on_late_command_display() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();
    chat.unified_exec_processes.push(UnifiedExecProcessSummary {
        key: "proc-1".to_string(),
        call_id: "call-1".to_string(),
        command_display: "sleep 5".to_string(),
        recent_chunks: Vec::new(),
    });

    terminal_interaction(&mut chat, "call-1", "proc-1", "");

    assert!(chat.transcript.active_cell.is_none());
    assert_eq!(
        chat.status_state.current_status.header,
        "Waiting for background terminal"
    );
    let status = chat
        .bottom_pane
        .status_widget()
        .expect("status indicator should be visible");
    assert_eq!(status.header(), "Waiting for background terminal");
    assert_eq!(status.details(), Some("sleep 5"));
}

#[tokio::test]
async fn unified_exec_empty_poll_for_finished_process_does_not_show_waiting_status() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();

    terminal_interaction(&mut chat, "call-finished", "proc-finished", "");

    assert_eq!(chat.status_state.current_status.header, "Working");
    let status = chat
        .bottom_pane
        .status_widget()
        .expect("task status indicator should remain visible");
    assert_eq!(status.header(), "Working");
    assert!(chat.unified_exec_wait_streak.is_none());
}











#[tokio::test]
async fn bang_shell_enter_while_task_running_submits_run_user_shell_command() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let thread_id = ThreadId::new();
    let rollout_file = NamedTempFile::new().unwrap();
    let configured = crate::session_state::ThreadSessionState {
        windows_sandbox_host: crate::app::WindowsSandboxHost::Local,
        thread_id,
        forked_from_id: None,
        fork_parent_title: None,
        thread_name: None,
        model: "test-model".to_string(),
        model_provider_id: "test-provider".to_string(),
        service_tier: None,
        approval_policy: AskForApproval::Never,
        approvals_reviewer: ApprovalsReviewer::User,
        permission_profile: PermissionProfile::read_only(),
        active_permission_profile: None,
        cwd: test_path_buf("/home/user/project").abs(),
        runtime_workspace_roots: Vec::new(),
        instruction_source_paths: Vec::new(),
        reasoning_effort: Some(ReasoningEffortConfig::default()),
        collaboration_mode: None,
        message_history: None,
        network_proxy: None,
        rollout_path: Some(rollout_file.path().to_path_buf()),
    };
    chat.handle_thread_session(configured);
    drain_insert_history(&mut rx);
    while op_rx.try_recv().is_ok() {}
    handle_turn_started(&mut chat, "turn-1");

    chat.bottom_pane
        .set_composer_text("!echo hi".to_string(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    match op_rx.try_recv() {
        Ok(Op::RunUserShellCommand { command }) => assert_eq!(command, "echo hi"),
        other => panic!("expected RunUserShellCommand op, got {other:?}"),
    }
    assert_matches!(
        rx.try_recv(),
        Ok(AppEvent::AppendMessageHistoryEntry { text, .. }) if text == "!echo hi"
    );
    assert_matches!(rx.try_recv(), Err(TryRecvError::Empty));
}

#[tokio::test]
async fn user_message_during_user_shell_command_is_queued_not_steered() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    handle_turn_started(&mut chat, "turn-1");
    let begin = begin_exec_with_source(
        &mut chat,
        "user-shell-sleep",
        "sleep 10",
        ExecCommandSource::UserShell,
    );

    assert!(chat.only_user_shell_commands_running());
    chat.bottom_pane
        .set_composer_text("hi".to_string(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_matches!(op_rx.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(chat.queued_user_message_texts(), vec!["hi".to_string()]);

    end_exec(&mut chat, begin, "", "", /*exit_code*/ 0);
    complete_assistant_message(
        &mut chat,
        "msg-done",
        "done",
        Some(MessagePhase::FinalAnswer),
    );
    handle_turn_completed(&mut chat, "turn-1", /*duration_ms*/ None);

    match next_submit_op(&mut op_rx) {
        Op::UserTurn { items, .. } => assert_eq!(
            items,
            vec![UserInput::Text {
                text: "hi".to_string(),
                text_elements: Vec::new(),
            }]
        ),
        other => panic!("expected queued user message after shell completion, got {other:?}"),
    }
    assert!(chat.input_queue.queued_user_messages.is_empty());
}






#[tokio::test]
async fn interrupt_preserves_unified_exec_processes() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    begin_unified_exec_startup(&mut chat, "call-1", "process-1", "sleep 5");
    begin_unified_exec_startup(&mut chat, "call-2", "process-2", "sleep 6");
    assert_eq!(chat.unified_exec_processes.len(), 2);

    handle_turn_interrupted(&mut chat, "turn-1");

    assert_eq!(chat.unified_exec_processes.len(), 2);

    chat.add_ps_output();
    let cells = drain_insert_history(&mut rx);
    let combined = cells
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        combined.contains("Background terminals"),
        "expected /ps to remain available after interrupt; got {combined:?}"
    );
    assert!(
        combined.contains("sleep 5") && combined.contains("sleep 6"),
        "expected /ps to list running unified exec processes; got {combined:?}"
    );

    let _ = drain_insert_history(&mut rx);
}


#[tokio::test]
async fn turn_complete_keeps_unified_exec_processes() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    begin_unified_exec_startup(&mut chat, "call-1", "process-1", "sleep 5");
    begin_unified_exec_startup(&mut chat, "call-2", "process-2", "sleep 6");
    assert_eq!(chat.unified_exec_processes.len(), 2);

    handle_turn_completed(&mut chat, "turn-1", /*duration_ms*/ None);

    assert_eq!(chat.unified_exec_processes.len(), 2);

    chat.add_ps_output();
    let cells = drain_insert_history(&mut rx);
    let combined = cells
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        combined.contains("Background terminals"),
        "expected /ps to remain available after turn complete; got {combined:?}"
    );
    assert!(
        combined.contains("sleep 5") && combined.contains("sleep 6"),
        "expected /ps to list running unified exec processes; got {combined:?}"
    );

    let _ = drain_insert_history(&mut rx);
}


#[tokio::test]
async fn apply_patch_manual_approval_adjusts_header() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    let mut proposed_changes = HashMap::new();
    proposed_changes.insert(
        PathBuf::from("foo.txt"),
        FileChange::Add {
            content: "hello\n".to_string(),
        },
    );
    handle_apply_patch_approval_request(
        &mut chat,
        "s1",
        ApplyPatchApprovalRequestEvent {
            call_id: "c1".into(),
            turn_id: "turn-c1".into(),
            changes: proposed_changes,
            reason: None,
            grant_root: None,
        },
    );
    drain_insert_history(&mut rx);

    let mut apply_changes = HashMap::new();
    apply_changes.insert(
        PathBuf::from("foo.txt"),
        FileChange::Add {
            content: "hello\n".to_string(),
        },
    );
    handle_patch_apply_begin(&mut chat, "c1", "turn-c1", apply_changes);

    let cells = drain_insert_history(&mut rx);
    assert!(!cells.is_empty(), "expected apply block cell to be sent");
    let blob = lines_to_single_string(cells.last().unwrap());
    assert!(
        blob.contains("Added foo.txt") || blob.contains("Edited foo.txt"),
        "expected apply summary header for foo.txt: {blob:?}"
    );
}


#[tokio::test]
async fn apply_patch_approval_sends_op_with_call_id() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    // Simulate receiving an approval request with a distinct event id and call id.
    let mut changes = HashMap::new();
    changes.insert(
        PathBuf::from("file.rs"),
        FileChange::Add {
            content: "fn main(){}\n".into(),
        },
    );
    let ev = ApplyPatchApprovalRequestEvent {
        call_id: "call-999".into(),
        turn_id: "turn-999".into(),
        changes,
        reason: None,
        grant_root: None,
    };
    handle_apply_patch_approval_request(&mut chat, "sub-123", ev);

    // Approve via key press 'y'
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    // Expect a thread-scoped PatchApproval op carrying the call id.
    let mut found = false;
    while let Ok(app_ev) = rx.try_recv() {
        if let AppEvent::SubmitThreadOp {
            op: Op::PatchApproval { id, decision },
            ..
        } = app_ev
        {
            assert_eq!(id, "call-999");
            assert_matches!(
                decision,
                codex_app_server_protocol::FileChangeApprovalDecision::Accept
            );
            found = true;
            break;
        }
    }
    assert!(found, "expected PatchApproval op to be sent");
}

#[tokio::test]
async fn apply_patch_full_flow_integration_like() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // 1) Backend requests approval
    let mut changes = HashMap::new();
    changes.insert(
        PathBuf::from("pkg.rs"),
        FileChange::Add { content: "".into() },
    );
    handle_apply_patch_approval_request(
        &mut chat,
        "sub-xyz",
        ApplyPatchApprovalRequestEvent {
            call_id: "call-1".into(),
            turn_id: "turn-call-1".into(),
            changes,
            reason: None,
            grant_root: None,
        },
    );

    // 2) User approves via 'y' and App receives a thread-scoped op
    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    let mut maybe_op: Option<Op> = None;
    while let Ok(app_ev) = rx.try_recv() {
        if let AppEvent::SubmitThreadOp { op, .. } = app_ev {
            maybe_op = Some(op);
            break;
        }
    }
    let op = maybe_op.expect("expected thread-scoped op after key press");

    // 3) App forwards to widget.submit_op, which pushes onto codex_op_tx
    chat.submit_op(op);
    let forwarded = op_rx
        .try_recv()
        .expect("expected op forwarded to codex channel");
    match forwarded {
        Op::PatchApproval { id, decision } => {
            assert_eq!(id, "call-1");
            assert_matches!(
                decision,
                codex_app_server_protocol::FileChangeApprovalDecision::Accept
            );
        }
        other => panic!("unexpected op forwarded: {other:?}"),
    }

    // 4) Simulate patch begin/end events from backend; ensure history cells are emitted
    let mut changes2 = HashMap::new();
    changes2.insert(
        PathBuf::from("pkg.rs"),
        FileChange::Add { content: "".into() },
    );
    handle_patch_apply_begin(&mut chat, "call-1", "turn-call-1", changes2);
    let mut end_changes = HashMap::new();
    end_changes.insert(
        PathBuf::from("pkg.rs"),
        FileChange::Add { content: "".into() },
    );
    handle_patch_apply_end(
        &mut chat,
        "call-1",
        "turn-call-1",
        end_changes,
        AppServerPatchApplyStatus::Completed,
    );
}

#[tokio::test]
async fn apply_patch_untrusted_shows_approval_modal() -> anyhow::Result<()> {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    // Ensure approval policy is untrusted (OnRequest)
    chat.config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())?;

    // Simulate a patch approval request from backend
    let mut changes = HashMap::new();
    changes.insert(
        PathBuf::from("a.rs"),
        FileChange::Add { content: "".into() },
    );
    handle_apply_patch_approval_request(
        &mut chat,
        "sub-1",
        ApplyPatchApprovalRequestEvent {
            call_id: "call-1".into(),
            turn_id: "turn-call-1".into(),
            changes,
            reason: None,
            grant_root: None,
        },
    );

    // Render and ensure the approval modal title is present
    let area = Rect::new(0, 0, 80, 12);
    let mut buf = Buffer::empty(area);
    chat.render(area, &mut buf);

    let mut contains_title = false;
    for y in 0..area.height {
        let mut row = String::new();
        for x in 0..area.width {
            row.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        if row.contains("Would you like to make the following edits?") {
            contains_title = true;
            break;
        }
    }
    assert!(
        contains_title,
        "expected approval modal to be visible with title 'Would you like to make the following edits?'"
    );

    Ok(())
}

#[tokio::test]
async fn apply_patch_request_omits_diff_summary_from_modal() -> anyhow::Result<()> {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    // Ensure we are in OnRequest so an approval is surfaced
    chat.config
        .permissions
        .approval_policy
        .set(AskForApproval::OnRequest.to_core())?;

    // Simulate backend asking to apply a patch adding two lines to README.md
    let mut changes = HashMap::new();
    changes.insert(
        PathBuf::from("README.md"),
        FileChange::Add {
            // Two lines (no trailing empty line counted)
            content: "line one\nline two\n".into(),
        },
    );
    handle_apply_patch_approval_request(
        &mut chat,
        "sub-apply",
        ApplyPatchApprovalRequestEvent {
            call_id: "call-apply".into(),
            turn_id: "turn-apply".into(),
            changes,
            reason: None,
            grant_root: None,
        },
    );

    assert!(
        drain_insert_history(&mut rx).is_empty(),
        "expected approval request to render via modal instead of history"
    );

    let area = Rect::new(0, 0, 80, chat.desired_height(/*width*/ 80));
    let mut buf = ratatui::buffer::Buffer::empty(area);
    chat.render(area, &mut buf);
    let mut contents = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            contents.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        contents.push('\n');
    }
    assert!(!contents.contains("README.md (+2 -0)"));
    assert!(!contents.contains("+line one"));
    assert!(!contents.contains("+line two"));

    Ok(())
}
