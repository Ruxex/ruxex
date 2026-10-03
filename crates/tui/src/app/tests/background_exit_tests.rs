use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn external_writer_view_quits_with_escape_ctrl_c_or_q() -> Result<()> {
    let (mut app, mut events, _operations) = make_test_app_with_channels().await;
    app.chat_widget.show_external_writer_thread();
    let mut app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    while events.try_recv().is_ok() {}

    app.keymap.app.open_external_editor = vec![crate::key_hint::ctrl(KeyCode::Char('g'))];
    app.handle_key_event(
        &mut tui,
        &mut app_server,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
    )
    .await;
    assert_eq!(
        app.chat_widget.external_editor_state(),
        ExternalEditorState::Closed
    );
    assert!(
        !events
            .try_recv()
            .is_ok_and(|event| matches!(event, AppEvent::LaunchExternalEditor))
    );

    for key in [
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
    ] {
        app.handle_key_event(&mut tui, &mut app_server, key).await;
        assert!(matches!(
            events.try_recv(),
            Ok(AppEvent::Exit(ExitMode::Immediate))
        ));
    }
    Ok(())
}

#[tokio::test]
async fn external_writer_view_preserves_draft_from_keys_and_paste() -> Result<()> {
    let (mut app, _events, _operations) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    app.enqueue_primary_thread_session(
        test_thread_session(thread_id, test_path_buf("/tmp/project")),
        Vec::new(),
    )
    .await?;
    app.app_server_target = AppServerTarget::Remote {
        endpoint: crate::RemoteAppServerEndpoint::WebSocket {
            websocket_url: "wss://example.com/".to_string(),
            auth_token: None,
        },
    };
    app.ensure_thread_channel(thread_id).mark_external_writer();
    app.chat_widget.insert_str("Retained draft");
    app.chat_widget.show_external_writer_thread();
    let mut app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;

    for offline in [false, true] {
        app.reconnect.offline = offline;
        app.handle_tui_event(
            &mut tui,
            &mut app_server,
            TuiEvent::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        )
        .await?;
        app.handle_tui_event(&mut tui, &mut app_server, TuiEvent::Paste("pasted".into()))
            .await?;
        assert_eq!(
            app.chat_widget.composer_text_with_pending(),
            "Retained draft"
        );
    }

    app.reconnect.offline = false;
    app.handle_tui_event(
        &mut tui,
        &mut app_server,
        TuiEvent::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL)),
    )
    .await?;
    assert!(app.overlay.is_some());
    Ok(())
}




fn prepare_local_daemon_thread(app: &mut App) -> Result<ThreadId> {
    app.app_server_target = AppServerTarget::LocalDaemon {
        allow_embedded_fallback: true,
        endpoint: crate::RemoteAppServerEndpoint::UnixSocket {
            socket_path: AbsolutePathBuf::relative_to_current_dir("genex.sock")?,
        },
    };
    let thread_id = ThreadId::new();
    app.active_thread_id = Some(thread_id);
    app.chat_widget.handle_thread_session(test_thread_session(
        thread_id,
        test_path_buf("/tmp/project"),
    ));
    Ok(thread_id)
}

fn prepare_running_local_daemon(app: &mut App) -> Result<ThreadId> {
    let thread_id = prepare_local_daemon_thread(app)?;
    app.chat_widget.handle_server_notification(
        turn_started_notification(thread_id, "turn-1"),
        /*replay_kind*/ None,
    );
    Ok(thread_id)
}

async fn open_running_task_exit_menu(
    app: &mut App,
    tui: &mut tui::Tui,
    app_server: &mut AppServerSession,
) {
    app.handle_key_event(
        tui,
        app_server,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    )
    .await;
}

async fn prepare_background_exit_test(
    app: &App,
    app_event_rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    op_rx: &mut mpsc::UnboundedReceiver<Op>,
) -> Result<(AppServerSession, tui::Tui)> {
    while app_event_rx.try_recv().is_ok() {}
    while op_rx.try_recv().is_ok() {}
    let app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    Ok((app_server, crate::tui::test_support::make_test_tui()?))
}


#[tokio::test]
async fn cancel_task_interrupts_without_exiting() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    prepare_running_local_daemon(&mut app)?;
    let (mut app_server, mut tui) =
        prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;
    open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let event = app_event_rx.try_recv()?;
    assert_matches!(
        event,
        AppEvent::RunningTaskExit {
            action: RunningTaskExitAction::CancelTask,
            ..
        }
    );
    let control = app.handle_event(&mut tui, &mut app_server, event).await?;

    assert_matches!(control, AppRunControl::Continue);
    assert_matches!(op_rx.try_recv(), Ok(Op::Interrupt));
    assert!(app_event_rx.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn run_in_background_detaches_without_interrupting_main_or_side_threads() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let parent_thread_id = prepare_running_local_daemon(&mut app)?;
    let side_thread_id = ThreadId::new();
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));
    let (mut app_server, mut tui) =
        prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;
    open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let event = app_event_rx.try_recv()?;
    assert_matches!(
        event,
        AppEvent::RunningTaskExit {
            action: RunningTaskExitAction::RunInBackground,
            thread_id,
        } if thread_id == parent_thread_id
    );
    let control = app.handle_event(&mut tui, &mut app_server, event).await?;

    assert_matches!(control, AppRunControl::Exit(ExitReason::UserRequested));
    assert!(app.side_threads.contains_key(&side_thread_id));
    assert!(op_rx.try_recv().is_err());
    assert!(app_event_rx.try_recv().is_err());
    Ok(())
}


#[tokio::test]
async fn daemon_ctrl_c_closes_running_side_thread_and_returns_to_parent() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let side_thread_id = prepare_running_local_daemon(&mut app)?;
    let (mut app_server, mut tui) =
        prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;
    let started = app_server.start_thread(&app.config).await?;
    let parent_thread_id = started.session.thread_id;
    app.primary_thread_id = Some(parent_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(parent_thread_id));
    app.thread_event_channels.insert(
        parent_thread_id,
        ThreadEventChannel::new_with_session(
            THREAD_EVENT_CHANNEL_CAPACITY,
            started.session,
            started.turns,
        ),
    );

    open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

    assert!(app.chat_widget.no_modal_or_popup_active());
    assert_eq!(app.active_thread_id, Some(parent_thread_id));
    assert!(!app.side_threads.contains_key(&side_thread_id));
    assert!(op_rx.try_recv().is_err());
    Ok(())
}


#[tokio::test]
async fn daemon_ctrl_c_hides_background_exit_with_queued_follow_up() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    prepare_running_local_daemon(&mut app)?;
    app.chat_widget
        .apply_external_edit("queued follow-up".to_string());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(
        app.chat_widget.queued_user_message_texts(),
        vec!["queued follow-up".to_string()]
    );
    let (mut app_server, mut tui) =
        prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;

    open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

    assert!(!render_bottom_popup(&app.chat_widget, /*width*/ 90).contains("Run in background"));
    assert!(op_rx.try_recv().is_err());
    assert!(app_event_rx.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn non_daemon_ctrl_c_keeps_interrupt_behavior() -> Result<()> {
    for target in [
        AppServerTarget::Embedded,
        AppServerTarget::Remote {
            endpoint: crate::RemoteAppServerEndpoint::WebSocket {
                websocket_url: "ws://127.0.0.1:4500".to_string(),
                auth_token: None,
            },
        },
    ] {
        let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
        prepare_running_local_daemon(&mut app)?;
        app.app_server_target = target;
        let (mut app_server, mut tui) =
            prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;

        open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

        assert!(app.chat_widget.no_modal_or_popup_active());
        assert_matches!(op_rx.try_recv(), Ok(Op::Interrupt));
        assert!(app_event_rx.try_recv().is_err());
    }
    Ok(())
}

#[tokio::test]
async fn daemon_ctrl_c_with_draft_preserves_composer_cancellation() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    prepare_running_local_daemon(&mut app)?;
    app.chat_widget.apply_external_edit("draft".to_string());
    let (mut app_server, mut tui) =
        prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;

    open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

    assert!(app.chat_widget.no_modal_or_popup_active());
    assert!(app.chat_widget.composer_is_empty());
    assert!(op_rx.try_recv().is_err());
    assert_matches!(
        app_event_rx.try_recv(),
        Ok(AppEvent::AppendMessageHistoryEntry { text, .. }) if text == "draft"
    );
    Ok(())
}

#[tokio::test]
async fn daemon_ctrl_c_during_paste_burst_does_not_show_exit_menu() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    prepare_running_local_daemon(&mut app)?;
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    assert!(app.chat_widget.composer_text_with_pending().is_empty());
    assert!(!app.chat_widget.composer_is_empty());
    let (mut app_server, mut tui) =
        prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;

    open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

    assert!(app.chat_widget.no_modal_or_popup_active());
    assert!(!app.chat_widget.composer_is_empty());
    assert_matches!(op_rx.try_recv(), Ok(Op::Interrupt));
    assert!(app_event_rx.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn daemon_ctrl_c_during_mcp_startup_does_not_show_background_exit_menu() -> Result<()> {
    let (mut app, mut app_event_rx, mut op_rx) = make_test_app_with_channels().await;
    let thread_id = prepare_local_daemon_thread(&mut app)?;
    app.chat_widget
        .set_mcp_startup_expected_servers(["slow".to_string()]);
    app.chat_widget.handle_server_notification(
        ServerNotification::McpServerStatusUpdated(McpServerStatusUpdatedNotification {
            thread_id: Some(thread_id.to_string()),
            name: "slow".to_string(),
            status: McpServerStartupState::Starting,
            error: None,
            failure_reason: None,
        }),
        /*replay_kind*/ None,
    );
    assert!(app.chat_widget.is_task_running_for_test());
    let (mut app_server, mut tui) =
        prepare_background_exit_test(&app, &mut app_event_rx, &mut op_rx).await?;

    open_running_task_exit_menu(&mut app, &mut tui, &mut app_server).await;

    assert!(app.chat_widget.no_modal_or_popup_active());
    assert_matches!(op_rx.try_recv(), Ok(Op::Interrupt));
    assert!(app_event_rx.try_recv().is_err());
    Ok(())
}
