use super::*;
use crate::app_event::AgentsOverviewAction;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ThreadLoadedListParams;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_model_provider_info::ModelProviderInfo;
use core_test_support::responses;
use core_test_support::streaming_sse::StreamingSseChunk;
use core_test_support::streaming_sse::start_streaming_sse_server;
use pretty_assertions::assert_eq;


#[tokio::test]
async fn lifecycle_shortcuts_target_filtered_task_in_any_state() {
    let mut app = make_test_app().await;
    let mut keymap = TuiKeymap::default();
    keymap.agents.archive = Some(KeybindingsSpec::One(KeybindingSpec("f5".into())));
    keymap.agents.delete = Some(KeybindingsSpec::One(KeybindingSpec("f6".into())));
    keymap.agents.hide = Some(KeybindingsSpec::One(KeybindingSpec("f7".into())));
    app.keymap = RuntimeKeymap::from_config(&keymap).unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    app.app_event_tx = AppEventSender::new(tx);
    for status in [
        ThreadStatus::Idle,
        ThreadStatus::NotLoaded,
        ThreadStatus::Active {
            active_flags: Vec::new(),
        },
    ] {
        let target = ThreadId::new();
        let mut view = app.agents_overview_view(
            vec![
                overview_thread(
                    ThreadId::new(),
                    /*parent_thread_id*/ None,
                    "Other",
                    ThreadStatus::Idle,
                ),
                overview_thread(target, /*parent_thread_id*/ None, "Target", status),
            ],
            Some(target),
        );
        view.handle_key_event(KeyCode::Esc.into());
        view.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        for character in "Target".chars() {
            view.handle_key_event(KeyCode::Char(character).into());
        }
        for (key, expected) in [
            (5, AgentsOverviewAction::Archive),
            (6, AgentsOverviewAction::Delete),
        ] {
            view.handle_key_event(KeyCode::F(key).into());
            let event = rx.try_recv();
            assert!(
                matches!(event, Ok(AppEvent::ConfirmAgentsOverviewAction { thread_id, action }) if thread_id == target && action == expected),
                "unexpected event: {event:?}"
            );
        }
        view.handle_key_event(KeyCode::F(7).into());
        assert!(
            matches!(rx.try_recv(), Ok(AppEvent::HideAgentsOverviewThread { thread_id }) if thread_id == target)
        );
        assert!(rx.try_recv().is_err());
        view.handle_key_event(KeyCode::Esc.into());
    }
}


#[tokio::test]
async fn hiding_rename_target_does_not_transfer_draft_to_neighbor() -> Result<()> {
    let (mut app, mut rx, _op_rx) = crate::app::tests::make_test_app_with_channels().await;
    let mut app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let mut keymap = TuiKeymap::default();
    keymap.agents.hide = Some(KeybindingsSpec::One(KeybindingSpec("f7".into())));
    app.keymap = RuntimeKeymap::from_config(&keymap).unwrap();
    let threads = ["Rename target", "Neighbor"]
        .into_iter()
        .map(|name| {
            overview_thread(
                ThreadId::new(),
                /*parent_thread_id*/ None,
                name,
                ThreadStatus::Idle,
            )
        })
        .collect::<Vec<_>>();
    let target = ThreadId::from_string(&threads[0].id)?;
    app.agents_overview.threads = threads
        .iter()
        .map(|thread| {
            (
                ThreadId::from_string(&thread.id).unwrap(),
                Some(thread.clone()),
            )
        })
        .collect();
    let mut view = app.agents_overview_view(threads, Some(target));
    view.handle_key_event(KeyCode::Char('r').into());
    view.handle_paste("Unsubmitted title".into());
    app.agents_overview.visible_thread_ids = view.thread_ids();
    app.chat_widget.show_bottom_pane_view(Box::new(view));
    app.chat_widget.handle_key_event(KeyCode::F(7).into());
    let hide = std::iter::from_fn(|| rx.try_recv().ok())
        .find(|event| matches!(event, AppEvent::HideAgentsOverviewThread { .. }))
        .expect("hide shortcut emits an event");
    Box::pin(app.handle_event(&mut tui, &mut app_server, hide)).await?;
    {
        let state = app.agents_overview.view_state.lock().unwrap();
        assert_eq!((state.renaming, state.input.as_str()), (false, ""));
    }
    app.chat_widget.handle_key_event(KeyCode::Enter.into());
    assert!(
        !std::iter::from_fn(|| rx.try_recv().ok())
            .any(|event| matches!(event, AppEvent::RenameAgentsOverviewThread { .. }))
    );
    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn hidden_task_stays_hidden_through_activity_and_seed_until_explicit_resume() -> Result<()> {
    let (mut app, mut rx, _op_rx) = crate::app::tests::make_test_app_with_channels().await;
    let mut app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let started = app_server.start_thread(&app.config).await?;
    let id = started.session.thread_id;
    app.enqueue_primary_thread_session(started.session, started.turns)
        .await?;
    let thread = overview_thread(
        id,
        /*parent_thread_id*/ None,
        "Hidden task",
        ThreadStatus::Idle,
    );
    app.agents_overview.threads.insert(id, Some(thread.clone()));
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let view = app.agents_overview_view(vec![thread.clone()], Some(id));
    app.chat_widget.show_bottom_pane_view(Box::new(view));
    app.chat_widget.handle_key_event(KeyCode::Esc.into());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    let hide = std::iter::from_fn(|| rx.try_recv().ok())
        .find(|event| matches!(event, AppEvent::HideAgentsOverviewThread { .. }))
        .expect("shortcut requests hiding the task");
    Box::pin(app.handle_event(&mut tui, &mut app_server, hide)).await?;
    app.track_agents_overview_notification(&ServerNotification::ThreadStarted(
        ThreadStartedNotification {
            thread: thread.clone(),
        },
    ));
    app.track_agents_overview_notification(&ServerNotification::ThreadStatusChanged(
        codex_app_server_protocol::ThreadStatusChangedNotification {
            thread_id: id.to_string(),
            status: ThreadStatus::Active {
                active_flags: Vec::new(),
            },
        },
    ));
    app.track_agents_overview_notification(&ServerNotification::ThreadClosed(
        ThreadClosedNotification {
            thread_id: id.to_string(),
        },
    ));
    let request_id = Uuid::new_v4();
    app.agents_overview.initialized = false;
    app.agents_overview.request_id = Some(request_id);
    app.apply_agents_overview_thread_refresh(
        &app_server,
        request_id,
        Ok(AgentsOverviewThreadRefresh {
            threads: HashMap::from([(id, Some(thread.clone()))]),
            last_messages: HashMap::new(),
            recent_seed_complete: true,
        }),
    );
    assert_eq!(
        app.agents_overview_view(vec![thread.clone()], /*selected_thread_id*/ None)
            .thread_ids(),
        Vec::<ThreadId>::new()
    );
    assert_eq!(app.primary_thread_id, Some(id));
    app.resume_target_session(
        &mut tui,
        &mut app_server,
        SessionTarget {
            thread_id: id,
            path: None,
            cwd: None,
            history_mode: None,
        },
    )
    .await?;
    assert_eq!(
        app.agents_overview_view(vec![thread], /*selected_thread_id*/ None)
            .thread_ids(),
        vec![id]
    );
    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn rejected_delete_preserves_a_live_attachment_and_draft() -> Result<()> {
    let (mut app, _rx, _op_rx) = crate::app::tests::make_test_app_with_channels().await;
    app.config.ephemeral = true;
    let mut app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let started = app_server.start_thread(&app.config).await?;
    let id = started.session.thread_id;
    app.enqueue_primary_thread_session(started.session, started.turns)
        .await?;
    app.chat_widget
        .apply_external_edit("Keep this draft".into());
    let draft = app.chat_widget.capture_thread_input_state();
    let mut tui = crate::tui::test_support::make_test_tui()?;
    tui.pause_events();
    // Ephemeral tasks provide a deterministic rejection before server teardown.
    Box::pin(app.run_agents_overview_action(
        &mut tui,
        &mut app_server,
        id,
        AgentsOverviewAction::Delete,
    ))
    .await?;
    assert_eq!(
        (app.primary_thread_id, app.chat_widget.thread_id()),
        (Some(id), Some(id))
    );
    assert_eq!(app.chat_widget.capture_thread_input_state(), draft);
    assert!(render_bottom_popup(&app.chat_widget, /*width*/ 80).contains("Could not delete task"));
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn disabled_footer_shortcuts_stay_bold_when_wrapped() {
    let app = make_test_app().await;
    let mut view = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
    view.handle_key_event(KeyCode::Esc.into());
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 84, /*height*/ 24,
    );
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    view.render(area, &mut buffer);
    let delete_key = crate::key_hint::plain(KeyCode::Delete).display_label();
    for (key, label) in [
        ("x", "x stop"),
        ("h", "h hide"),
        ("a", "a archive"),
        (delete_key.as_str(), delete_key.as_str()),
    ] {
        let cells = buffer
            .content()
            .windows(label.len())
            .find(|cells| {
                cells
                    .iter()
                    .map(ratatui::buffer::Cell::symbol)
                    .collect::<String>()
                    == label
            })
            .expect("footer shortcut");
        assert_eq!(
            cells[..key.len()]
                .iter()
                .map(|cell| cell.modifier)
                .collect::<Vec<_>>(),
            vec![ratatui::style::Modifier::BOLD | ratatui::style::Modifier::DIM; key.len()]
        );
    }
}
