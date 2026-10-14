use super::*;
use crate::chatwidget::UserMessage;
use ruxex_app_server_protocol::RuxexErrorInfo;
use ruxex_app_server_protocol::ErrorNotification;
use ruxex_app_server_protocol::ThreadGoalStatus;
use ruxex_protocol::models::ManagedFileSystemPermissions;
use ruxex_protocol::permissions::FileSystemAccessMode;
use ruxex_protocol::permissions::FileSystemPath;
use ruxex_protocol::permissions::FileSystemSandboxEntry;
use ruxex_protocol::permissions::FileSystemSpecialPath;
use ruxex_protocol::permissions::NetworkSandboxPolicy;
use pretty_assertions::assert_eq;





#[tokio::test]
async fn misalignment_policy_blocks_queued_turns_and_goal_resumption() -> Result<()> {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let checkout = tempfile::tempdir()?;
    app.config.cwd = AbsolutePathBuf::from_absolute_path(checkout.path())?;
    for args in [
        vec!["init", "--quiet"],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    ] {
        assert!(
            std::process::Command::new("git")
                .args(args)
                .current_dir(checkout.path())
                .status()?
                .success()
        );
    }
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let mut app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let started = app_server.start_thread(&app.config).await?;
    let thread_id = started.session.thread_id;
    let session = started.session.clone();
    app.enqueue_primary_thread_session(started.session, started.turns)
        .await?;
    app_server
        .thread_goal_set(
            thread_id,
            Some("Do not restart this stopped goal".to_string()),
            Some(ThreadGoalStatus::Blocked),
            /*token_budget*/ None,
        )
        .await?;
    app.chat_widget
        .restore_user_message_to_composer("queued before terminal failure".into());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let queued_turn = next_user_turn_op(&mut op_rx);
    let failed_turn = Turn {
        error: Some(AppServerTurnError {
            misalignment: None,
            message: "misalignment policy violation".to_string(),
            ruxex_error_info: Some(RuxexErrorInfo::MisalignmentPolicyViolation),
            additional_details: None,
        }),
        ..test_turn("turn-1", TurnStatus::Failed, Vec::new())
    };
    app.enqueue_primary_thread_session(session, vec![failed_turn])
        .await?;
    let precaution = render_bottom_popup(&app.chat_widget, /*width*/ 80);
    let AppServerRequestId::Integer(next_request_id) = app_server.next_request_id() else {
        unreachable!("embedded app-server request IDs are integers");
    };

    app.maybe_prompt_resume_paused_goal_after_resume(&mut app_server, thread_id)
        .await;
    for event in [
        AppEvent::RuxexOp(queued_turn),
        AppEvent::RuxexOp(crate::app_command::AppCommand::ExecApproval {
            id: "queued-approval".to_string(),
            turn_id: None,
            decision: ruxex_app_server_protocol::CommandExecutionApprovalDecision::Accept,
        }),
        AppEvent::SetThreadGoalStatus {
            thread_id,
            status: ThreadGoalStatus::Active,
        },
        AppEvent::SetThreadGoalDraft {
            thread_id,
            draft: crate::goal_files::GoalDraft {
                objective: "Do not replace this stopped goal".to_string(),
                ..Default::default()
            },
            mode: crate::app_event::ThreadGoalSetMode::ReplaceExisting,
        },
        AppEvent::StartSide {
            parent_thread_id: thread_id,
            user_message: Some("Do not fork this stopped thread".into()),
        },
        AppEvent::ForkCurrentSession { name: None },
        AppEvent::StartManagedWorktree {
            mode: crate::app_event::ManagedWorktreeMode::Fork,
            name: None,
        },
        AppEvent::OpenAgentPicker,
        AppEvent::SelectAgentThread(thread_id),
    ] {
        app.handle_event(&mut tui, &mut app_server, event).await?;
    }

    assert!(!app.config.ruxex_home.join("worktrees").exists());
    assert_eq!(
        app_server.next_request_id(),
        AppServerRequestId::Integer(next_request_id + 1)
    );
    assert_eq!(
        render_bottom_popup(&app.chat_widget, /*width*/ 80),
        precaution
    );
    assert_eq!(
        app_server
            .thread_goal_get(thread_id)
            .await?
            .goal
            .unwrap()
            .status,
        ThreadGoalStatus::Blocked
    );

    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn misalignment_policy_in_parent_stops_active_side_conversation() -> Result<()> {
    let (mut app, _app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let parent_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.primary_thread_id = Some(parent_thread_id);
    app.active_thread_id = Some(side_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));
    app.chat_widget.handle_thread_session(test_thread_session(
        side_thread_id,
        app.config.cwd.to_path_buf(),
    ));
    app.chat_widget
        .restore_user_message_to_composer("pending side turn".into());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    while op_rx.try_recv().is_ok() {}

    app.enqueue_thread_notification(
        parent_thread_id,
        ServerNotification::Error(ErrorNotification {
            error: AppServerTurnError {
                misalignment: None,
                message: "misalignment policy violation".to_string(),
                ruxex_error_info: Some(RuxexErrorInfo::MisalignmentPolicyViolation),
                additional_details: None,
            },
            will_retry: false,
            thread_id: parent_thread_id.to_string(),
            turn_id: "parent-turn".to_string(),
        }),
    )
    .await?;

    assert!(app.chat_widget.has_misalignment_policy_violation());
    assert!(
        render_bottom_popup(&app.chat_widget, /*width*/ 80)
            .contains("Chat stopped as a precaution")
    );
    assert_matches!(op_rx.try_recv(), Ok(Op::Interrupt));

    Ok(())
}
