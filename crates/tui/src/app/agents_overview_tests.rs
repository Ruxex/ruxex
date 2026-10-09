use super::super::agents_overview_view::AgentsOverviewGrouping;
use super::*;






use crate::app::test_support::make_test_app;
use crate::app_event::AgentsOverviewThreadRefresh;
use crate::bottom_pane::BottomPaneView;
use crate::chatwidget::tests::helpers::render_bottom_popup;
use crate::render::renderable::Renderable;
use crate::test_support::PathBufExt;
use crate::test_support::test_path_buf;
use crate::test_support::test_path_display;
use genex_app_server_client::AppServerEvent;
use genex_app_server_protocol::CurrentTimeReadParams;
use genex_app_server_protocol::ReasoningSummaryTextDeltaNotification;
use genex_app_server_protocol::RequestId;
use genex_app_server_protocol::ServerRequest;
use genex_app_server_protocol::SessionSource;
use genex_app_server_protocol::ThreadActiveFlag;
use genex_app_server_protocol::ThreadArchivedNotification;
use genex_app_server_protocol::ThreadClosedNotification;
use genex_app_server_protocol::ThreadDeletedNotification;
use genex_app_server_protocol::ThreadNameUpdatedNotification;
use genex_app_server_protocol::ThreadSource;
use genex_app_server_protocol::ThreadStartedNotification;
use genex_app_server_protocol::ThreadStatus;
use genex_app_server_protocol::ThreadUnsubscribeParams;
use genex_app_server_protocol::ThreadUnsubscribeResponse;
use genex_app_server_protocol::ThreadUnsubscribeStatus;
use genex_app_server_protocol::ToolRequestUserInputParams;
use genex_app_server_protocol::ToolRequestUserInputQuestion;
use genex_config::types::KeybindingSpec;
use genex_config::types::KeybindingsSpec;
use genex_config::types::TuiKeymap;
use genex_protocol::models::PermissionProfile;
use genex_protocol::protocol::SubAgentSource;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

static OVERVIEW_TIMESTAMP: std::sync::LazyLock<i64> =
    std::sync::LazyLock::new(|| chrono::Utc::now().timestamp() - 120);


#[tokio::test]
async fn overview_right_preserves_editors_and_offline_state() {
    let mut app = make_test_app().await;
    app.config.disable_paste_burst = true;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    app.app_event_tx = AppEventSender::new(tx);
    let thread_id = ThreadId::new();
    let mut view = app.agents_overview_view(
        vec![overview_thread(
            thread_id,
            /*parent_thread_id*/ None,
            "Task",
            ThreadStatus::Idle,
        )],
        Some(thread_id),
    );
    view.handle_key_event(KeyCode::Esc.into());
    view.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
    view.handle_key_event(KeyCode::Right.into());
    assert!(app.agents_overview.view_state.lock().unwrap().renaming);
    view.handle_key_event(KeyCode::Esc.into());
    app.agents_overview
        .view_state
        .lock()
        .unwrap()
        .connection_notice = Some("Offline");
    view.handle_key_event(KeyCode::Right.into());
    view.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    view.on_ctrl_c();
    view.handle_key_event(KeyCode::Right.into());
    assert!(!view.is_complete());
    assert!(rx.try_recv().is_err());
}

fn overview_thread(
    thread_id: ThreadId,
    parent_thread_id: Option<ThreadId>,
    name: &str,
    status: ThreadStatus,
) -> Thread {
    Thread {
        originator: None,
        environments: None,
        id: thread_id.to_string(),
        extra: None,
        project_id: None,
        daybreak_enabled: None,
        session_id: parent_thread_id.unwrap_or(thread_id).to_string(),
        forked_from_id: None,
        parent_thread_id: None,
        preview: name.to_string(),
        ephemeral: false,
        section: None,
        section_entered_at: None,
        history_mode: Default::default(),
        model_provider: "provider".to_string(),
        model: None,
        reasoning_effort: None,
        created_at: *OVERVIEW_TIMESTAMP,
        updated_at: *OVERVIEW_TIMESTAMP,
        recency_at: Some(*OVERVIEW_TIMESTAMP),
        status,
        path: None,
        cwd: test_path_buf("/tmp/project").abs(),
        cli_version: "0.0.0".to_string(),
        source: parent_thread_id.map_or(SessionSource::Cli, |parent_thread_id| {
            SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth: 1,
                agent_path: None,
                agent_nickname: None,
                agent_role: None,
            })
        }),
        can_accept_direct_input: Some(parent_thread_id.is_none()),
        thread_source: None,
        agent_nickname: None,
        agent_role: None,
        git_info: None,
        name: Some(name.to_string()),
        turns: Vec::new(),
    }
}

#[tokio::test]
async fn shared_overview_keeps_rows_and_replays_changes_over_stale_reads() -> Result<()> {
    let mut app = make_test_app().await;
    let app_server = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let [retained, archived, deleted, created] = std::array::from_fn(|_| ThreadId::new());
    let [
        mut retained_thread,
        archived_thread,
        deleted_thread,
        created_thread,
    ] = [
        (retained, "Old name"),
        (archived, "Archived"),
        (deleted, "Deleted"),
        (created, "Created in another client"),
    ]
    .map(|(id, name)| {
        overview_thread(id, /*parent_thread_id*/ None, name, ThreadStatus::Idle)
    });
    let request_id = Uuid::new_v4();
    app.agents_overview.request_id = Some(request_id);
    // The seed is still in flight and the command center has never been opened.
    for notification in [
        ServerNotification::ThreadStarted(ThreadStartedNotification {
            thread: created_thread.clone(),
        }),
        ServerNotification::ThreadNameUpdated(ThreadNameUpdatedNotification {
            thread_id: retained.to_string(),
            thread_name: Some("New name".to_string()),
        }),
        ServerNotification::ThreadClosed(ThreadClosedNotification {
            thread_id: retained.to_string(),
        }),
        ServerNotification::ThreadArchived(ThreadArchivedNotification {
            thread_id: archived.to_string(),
        }),
        ServerNotification::ThreadDeleted(ThreadDeletedNotification {
            thread_id: deleted.to_string(),
        }),
    ] {
        app.handle_app_server_event(
            &app_server,
            AppServerEvent::ServerNotification(Box::new(notification)),
        )
        .await;
    }
    app.apply_agents_overview_thread_refresh(
        &app_server,
        request_id,
        Ok(AgentsOverviewThreadRefresh {
            last_messages: HashMap::new(),
            threads: HashMap::from([
                (retained, Some(retained_thread.clone())),
                (archived, Some(archived_thread)),
                (deleted, Some(deleted_thread)),
            ]),
            recent_seed_complete: true,
        }),
    );
    retained_thread.name = Some("New name".to_string());
    retained_thread.status = ThreadStatus::NotLoaded;
    let expected = HashMap::from([
        (retained, Some(retained_thread)),
        (created, Some(created_thread)),
    ]);
    assert_eq!(app.agents_overview.threads, expected);

    let request_id = Uuid::new_v4();
    app.agents_overview.request_id = Some(request_id);
    app.apply_agents_overview_thread_refresh(
        &app_server,
        request_id,
        Ok(AgentsOverviewThreadRefresh {
            last_messages: HashMap::new(),
            threads: HashMap::from([(retained, None)]),
            recent_seed_complete: true,
        }),
    );
    assert_eq!(app.agents_overview.threads, expected);
    let request_id = Uuid::new_v4();
    app.agents_overview.request_id = Some(request_id);
    app.apply_agents_overview_thread_refresh(
        &app_server,
        request_id,
        Err("temporarily unavailable".to_string()),
    );
    assert_eq!(app.agents_overview.threads, expected);

    let view = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
    app.chat_widget.show_bottom_pane_view(Box::new(view));
    let stale_messages = HashMap::from([(retained, "Removed answer".into())]);
    for event in [
        AppServerEvent::ServerNotification(Box::new(ServerNotification::ThreadReverted(
            genex_app_server_protocol::ThreadRevertedNotification {
                thread_id: retained.to_string(),
            },
        ))),
        AppServerEvent::Lagged { skipped: 1 },
    ] {
        let old_request = Uuid::new_v4();
        app.agents_overview.request_id = Some(old_request);
        app.agents_overview.activity.entry(retained).or_default();
        app.agents_overview.last_messages = stale_messages.clone();
        app.handle_app_server_event(&app_server, event).await;
        assert!(!app.agents_overview.activity.contains_key(&retained));
        assert!(app.agents_overview.last_messages.is_empty());
        // Fresh activity must survive an older read arriving after invalidation.
        app.track_agents_overview_notification(&reasoning_delta(
            retained,
            "new-reasoning",
            "**Checking the revised task**",
        ));
        let current_request = app.agents_overview.request_id.unwrap();
        for (request_id, last_messages) in [
            (old_request, stale_messages.clone()),
            (current_request, HashMap::new()),
        ] {
            app.apply_agents_overview_thread_refresh(
                &app_server,
                request_id,
                Ok(AgentsOverviewThreadRefresh {
                    threads: HashMap::new(),
                    last_messages,
                    recent_seed_complete: true,
                }),
            );
            assert!(app.agents_overview.last_messages.is_empty());
        }
        assert!(app.agents_overview.activity.contains_key(&retained));
        assert!(app.agents_overview.request_id.is_none());
    }
    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_overview_seeds_once_and_retains_locally_resumed_history() -> Result<()> {
    let mut app = make_test_app().await;
    trust_fixture_folders(&mut app);
    let mut ids = Vec::new();
    for day in 1..=22 {
        let source = match day {
            3 => genex_protocol::protocol::SessionSource::Custom("custom".to_string()),
            4 => genex_protocol::protocol::SessionSource::Custom("external".to_string()),
            5 => genex_protocol::protocol::SessionSource::Exec,
            6 => genex_protocol::protocol::SessionSource::Mcp,
            _ => genex_protocol::protocol::SessionSource::Cli,
        };
        ids.push(ThreadId::from_string(
            &app_test_support::create_fake_rollout_with_source(
                &app.config.genex_home,
                &format!("2025-01-{day:02}T12-00-00"),
                &format!("2025-01-{day:02}T12:00:00Z"),
                &format!("Task {day}"),
                /*git_info*/ None,
                source,
            )
            .expect("materialize historical session"),
        )?);
    }
    let message_path = app_test_support::rollout_path(
        &app.config.genex_home,
        "2025-01-21T12-00-00",
        &ids[20].to_string(),
    );
    let mut history = std::fs::read_to_string(&message_path)?;
    history.push_str(
        &serde_json::json!({
            "timestamp": "2025-01-21T12:00:01Z",
            "type": "event_msg",
            "payload": { "type": "agent_message", "message": "Found the regression in the parser." }
        })
        .to_string(),
    );
    history.push('\n');
    std::fs::write(message_path, history)?;
    let config = app.config.clone();
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(&config)).await?;
    for thread_id in [ids[0], ids[21]] {
        app_server
            .resume_thread(
                &app.local_settings,
                config.clone(),
                thread_id,
                crate::app_server_session::ResumeModelSettings::PreserveExistingThread,
            )
            .await?;
    }
    // A newer rollout missing from the index must not trigger a startup filesystem scan.
    app_test_support::create_fake_rollout_with_source(
        &app.config.genex_home,
        "2025-01-23T12-00-00",
        "2025-01-23T12:00:00Z",
        "Unindexed task",
        /*git_info*/ None,
        genex_protocol::protocol::SessionSource::Cli,
    )
    .expect("materialize unindexed session");
    app.app_server_target = AppServerTarget::LocalDaemon {
        allow_embedded_fallback: true,
        endpoint: crate::RemoteAppServerEndpoint::UnixSocket {
            socket_path: test_path_buf("/tmp/unused.sock").abs(),
        },
    };
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    app.app_event_tx = crate::app_event_sender::AppEventSender::new(event_tx);
    app.refresh_agents_overview_threads(&app_server);
    finish_overview_refresh(&mut app, &app_server, &mut event_rx).await;
    let recent_ids: HashSet<_> = ids[2..].iter().copied().collect();
    let mut expected = recent_ids.clone();
    expected.insert(ids[0]);
    let retained: HashSet<_> = app.agents_overview.threads.keys().copied().collect();
    assert_eq!(retained, expected);
    assert_eq!(
        app.agents_overview.last_messages,
        HashMap::from([(ids[20], "Found the regression in the parser.".to_string())])
    );
    let thread = app.agents_overview.threads[&ids[20]].as_ref().unwrap();
    assert_eq!(thread.status, ThreadStatus::NotLoaded);

    let created = app_server.start_thread(&config).await?.session.thread_id;
    // Closing the view must not cancel a metadata refresh or forget unloaded entries.
    app.primary_thread_id = Some(ids[0]);
    app.open_agents_overview(&app_server);
    let visible: HashSet<_> = app
        .agents_overview
        .visible_thread_ids
        .iter()
        .copied()
        .collect();
    assert_eq!(visible, expected);
    app.agents_overview.view_state.lock().unwrap().completion =
        Some(crate::bottom_pane::ViewCompletion::Accepted);
    app.chat_widget.pre_draw_tick();
    finish_overview_refresh(&mut app, &app_server, &mut event_rx).await;
    let retained: HashSet<_> = app.agents_overview.threads.keys().copied().collect();
    assert_eq!(retained, expected);

    // A creation notification adds a session without attaching a local view.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = app_server.next_event().await.expect("server notification");
            let is_created = matches!(&event, AppServerEvent::ServerNotification(notification)
                if matches!(notification.as_ref(), ServerNotification::ThreadStarted(started) if started.thread.id == created.to_string()));
            app.handle_app_server_event(&app_server, event).await;
            if is_created { break; }
        }
    }).await.expect("creation notification received");
    expected.insert(created);
    let retained: HashSet<_> = app.agents_overview.threads.keys().copied().collect();
    assert_eq!(retained, expected);

    // Opening older history is a local addition, even without starting a turn.
    let resumed = app_server
        .resume_thread(
            &app.local_settings,
            config.clone(),
            ids[1],
            crate::app_server_session::ResumeModelSettings::PreserveExistingThread,
        )
        .await?;
    app.enqueue_primary_thread_session(resumed.session, resumed.turns)
        .await?;
    app.open_agents_overview(&app_server);
    finish_overview_refresh(&mut app, &app_server, &mut event_rx).await;
    expected.insert(ids[1]);
    let visible: HashSet<_> = app
        .agents_overview
        .visible_thread_ids
        .iter()
        .copied()
        .collect();
    assert_eq!(visible, expected);

    // An unloaded row can be selected through the same flow as a loaded row.
    std::fs::write(
        app.config.genex_home.join("config.toml"),
        "[tui]\nresume_cwd = \"session\"\n",
    )?;
    crate::legacy_core::config::set_project_trust_level(
        app.config.genex_home.as_path(),
        &test_path_buf("/"),
        genex_protocol::config_types::TrustLevel::Trusted,
    )
    .map_err(std::io::Error::other)?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    Box::pin(app.select_agents_overview_thread(&mut tui, &mut app_server, ids[2])).await?;
    assert_eq!(app.primary_thread_id, Some(ids[2]));
    app_server.shutdown().await?;

    // A fresh TUI/server has no in-memory additions; read-only resumes did not promote history.
    let mut restarted = make_test_app().await;
    restarted.app_server_target = app.app_server_target.clone();
    let app_server = Box::pin(crate::start_embedded_app_server_for_picker(&config)).await?;
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    restarted.app_event_tx = crate::app_event_sender::AppEventSender::new(event_tx);
    restarted.refresh_agents_overview_threads(&app_server);
    finish_overview_refresh(&mut restarted, &app_server, &mut event_rx).await;
    let retained: HashSet<_> = restarted.agents_overview.threads.keys().copied().collect();
    assert_eq!(retained, recent_ids);
    app_server.shutdown().await?;
    Ok(())
}

async fn finish_overview_refresh(
    app: &mut App,
    app_server: &AppServerSession,
    events: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let AppEvent::AgentsOverviewThreadsLoaded { request_id, result } =
                events.recv().await.expect("overview event")
            {
                assert!(result.is_ok(), "{result:?}");
                app.apply_agents_overview_thread_refresh(app_server, request_id, result);
                return;
            }
        }
    })
    .await
    .expect("overview refresh completed");
}

#[tokio::test]
async fn hidden_system_thread_does_not_refresh_shared_overview() {
    let mut app = make_test_app().await;
    let app_server = crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref())
        .await
        .expect("embedded app server");
    let view = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
    app.chat_widget.show_bottom_pane_view(Box::new(view));

    let request_id = uuid::Uuid::new_v4();
    app.agents_overview.request_id = Some(request_id);

    let parent_thread_id = ThreadId::new();
    app.primary_thread_id = Some(parent_thread_id);
    app.active_thread_id = Some(parent_thread_id);
    app.ensure_thread_channel(parent_thread_id);
    app.agents_overview
        .dispatched_requests
        .insert(parent_thread_id, Vec::new());

    let hidden_thread_id = ThreadId::new();
    let mut thread = overview_thread(
        hidden_thread_id,
        Some(parent_thread_id),
        "Generate thread title",
        ThreadStatus::Idle,
    );
    thread.ephemeral = true;
    thread.thread_source = Some(ThreadSource::Feature("system".to_string()));

    app.handle_app_server_event(
        &app_server,
        AppServerEvent::ServerNotification(Box::new(ServerNotification::ThreadStarted(
            ThreadStartedNotification { thread },
        ))),
    )
    .await;

    assert_eq!(
        (
            app.agents_overview.request_id,
            app.agents_overview.refresh_pending,
        ),
        (Some(request_id), false)
    );
    assert!(!app.thread_event_channels.contains_key(&hidden_thread_id));
    assert!(
        !app.agents_overview
            .dispatched_requests
            .contains_key(&hidden_thread_id)
    );

    let visible_thread_id = ThreadId::new();
    let mut thread = overview_thread(
        visible_thread_id,
        Some(parent_thread_id),
        "Persisted system thread",
        ThreadStatus::Idle,
    );
    thread.thread_source = Some(ThreadSource::Feature("system".to_string()));

    app.handle_app_server_event(
        &app_server,
        AppServerEvent::ServerNotification(Box::new(ServerNotification::ThreadStarted(
            ThreadStartedNotification { thread },
        ))),
    )
    .await;

    assert_eq!(
        (
            app.agents_overview.request_id,
            app.agents_overview.refresh_pending,
        ),
        (Some(request_id), true)
    );
    assert!(app.thread_event_channels.contains_key(&visible_thread_id));
    assert!(
        app.agents_overview
            .dispatched_requests
            .contains_key(&visible_thread_id)
    );

    app_server.shutdown().await.expect("shutdown app server");
}


fn reasoning_delta(thread_id: ThreadId, item_id: &str, delta: &str) -> ServerNotification {
    ServerNotification::ReasoningSummaryTextDelta(ReasoningSummaryTextDeltaNotification {
        thread_id: thread_id.to_string(),
        turn_id: "turn".into(),
        item_id: item_id.into(),
        summary_index: 0,
        delta: delta.into(),
    })
}


#[test]
fn agents_overview_markdown_preview_preserves_layout_and_bounds() {
    let text = format!("a\r\n\t\u{1b}{}", "界".repeat(600));
    assert_eq!(
        super::super::agents_overview_details::preview_markdown(&text),
        format!("a\n\t{}", "界".repeat(509))
    );
}







#[tokio::test]
async fn filtered_dashboard_actions_use_configured_shortcuts() {
    let mut app = make_test_app().await;
    app.chat_widget.toggle_vim_mode_and_notify();
    let mut keymap = TuiKeymap::default();
    keymap.agents.search = Some(KeybindingsSpec::One(KeybindingSpec("f6".to_string())));
    keymap.agents.stop = Some(KeybindingsSpec::One(KeybindingSpec("f10".to_string())));
    keymap.agents.resume = Some(KeybindingsSpec::One(KeybindingSpec("f8".to_string())));
    keymap.list.cancel = Some(KeybindingsSpec::One(KeybindingSpec("f7".into())));
    keymap.list.move_down = Some(KeybindingsSpec::One(KeybindingSpec("v".into())));
    app.keymap = crate::keymap::RuntimeKeymap::from_config(&keymap).expect("runtime keymap");
    let first = ThreadId::new();
    let second = ThreadId::new();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut view = AgentsOverviewView::new(
        app.agents_overview_view(
            vec![
                overview_thread(
                    first,
                    /*parent_thread_id*/ None,
                    "First task",
                    ThreadStatus::Idle,
                ),
                overview_thread(
                    second,
                    /*parent_thread_id*/ None,
                    "Second task",
                    ThreadStatus::Active {
                        active_flags: Vec::new(),
                    },
                ),
            ],
            Some(first),
        )
        .rows,
        Some(first),
        /*worktrees_enabled*/ false,
        /*use_theme_colors*/ true,
        crate::app_event_sender::AppEventSender::new(event_tx),
        app.keymap.clone(),
        Arc::clone(&app.agents_overview.view_state),
    );

    view.handle_key_event(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE));
    assert!(event_rx.try_recv().is_err());
    view.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    view.handle_key_event(KeyCode::Esc.into());
    for (key, expected) in [('v', second), ('k', first), ('j', first)] {
        view.handle_key_event(KeyCode::Char(key).into());
        let selected = &view.rows[view.selected_index().unwrap()];
        assert_eq!(selected.thread_id, expected);
    }

    view.handle_key_event(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE));
    assert!(view.handle_paste("Second task".to_string()));
    view.handle_key_event(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE));
    assert!(matches!(
        event_rx.try_recv(),
        Ok(AppEvent::OpenResumePicker)
    ));
    assert!(!view.is_complete());
    view.handle_key_event(KeyEvent::new(KeyCode::F(10), KeyModifiers::NONE));
    assert!(matches!(
        event_rx.try_recv(),
        Ok(AppEvent::StopAgentsOverviewThread { thread_id }) if thread_id == second
    ));
    view.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(matches!(
        event_rx.try_recv(),
        Ok(AppEvent::SelectAgentsOverviewThread { thread_id }) if thread_id == second
    ));
    assert!(event_rx.try_recv().is_err());
    view.handle_key_event(KeyCode::Esc.into());
    for offline in [false, true] {
        app.agents_overview
            .view_state
            .lock()
            .unwrap()
            .connection_notice = offline.then_some("Offline");
        view.handle_key_event(KeyCode::F(6).into());
        view.handle_paste("no matching task".into());
        assert_eq!(view.selected_index(), Some(usize::MAX));
        view.handle_key_event(KeyCode::F(7).into());
        assert_eq!(view.selected_index(), Some(0));
    }
}

#[tokio::test]
async fn failed_root_switch_keeps_background_requests_on_the_active_session() -> Result<()> {
    let mut app = make_test_app().await;
    let mut app_server =
        crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref()).await?;
    app.primary_thread_id = Some(ThreadId::new());
    app.ensure_thread_channel(ThreadId::new())
        .store
        .lock()
        .await
        .active_turn_id = Some("running-turn".to_string());
    let mut tui = crate::tui::test_support::make_test_tui()?;

    app.select_agents_overview_thread(&mut tui, &mut app_server, ThreadId::new())
        .await?;

    assert!(app.agents_overview.dispatched_requests.is_empty());
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn root_switch_loads_local_preferences_from_disk() -> Result<()> {
    // Keep the large setup and root-switch futures off the test thread's stack.
    let mut app = Box::pin(make_test_app()).await;
    trust_fixture_folders(&mut app);
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await?;
    let previous = Box::pin(app_server.start_thread(&app.config)).await?;
    app.enqueue_primary_thread_session(previous.session, previous.turns)
        .await?;
    let target_thread_id = ThreadId::from_string(
        &app_test_support::create_fake_rollout(
            app.config.genex_home.as_path(),
            "2025-01-05T12-00-00",
            "2025-01-05T12:00:00Z",
            "Target task",
            /*git_info*/ None,
        )
        .expect("materialize target rollout"),
    )?;
    std::fs::write(
        app.local_settings.user_config_path.as_path(),
        "[tui]\ntheme = \"dracula\"\nresume_cwd = \"session\"\n[history]\npersistence = \"none\"\n",
    )?;
    let config = Box::pin(app.rebuild_config_for_cwd(app.config.cwd.to_path_buf())).await?;
    let expected = crate::local_settings::LocalSettings::from(&config);
    let mut tui = crate::tui::test_support::make_test_tui()?;

    Box::pin(app.select_agents_overview_thread(&mut tui, &mut app_server, target_thread_id))
        .await?;

    assert_eq!(app.current_displayed_thread_id(), Some(target_thread_id));
    assert_eq!(app.local_settings, expected);
    assert_eq!(app.chat_widget.local_settings, expected);
    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn root_switch_preserves_idle_root_with_running_subagent() -> Result<()> {
    let mut app = make_test_app().await;
    trust_fixture_folders(&mut app);
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(
        app.chat_widget.config_ref(),
    ))
    .await?;
    let previous = app_server.start_thread(&app.config).await?;
    let previous_root_id = previous.session.thread_id;
    app.enqueue_primary_thread_session(previous.session, previous.turns)
        .await?;
    let target_thread_id = ThreadId::from_string(
        &app_test_support::create_fake_rollout(
            app.config.genex_home.as_path(),
            "2025-01-05T12-00-00",
            "2025-01-05T12:00:00Z",
            "Target task",
            /*git_info*/ None,
        )
        .expect("materialize target rollout"),
    )?;
    let target = app_server
        .resume_thread(
            &app.local_settings,
            app.config.clone(),
            target_thread_id,
            crate::app_server_session::ResumeModelSettings::PreserveExistingThread,
        )
        .await?;
    let child_id = ThreadId::new();
    let idle_child_id = ThreadId::new();
    app.ensure_thread_channel(idle_child_id);
    app.upsert_agent_picker_thread(
        child_id, /*agent_nickname*/ None, /*agent_role*/ None, /*is_closed*/ false,
    );
    app.agent_navigation.mark_running(child_id);
    app.active_thread_id = Some(child_id);
    let mut tui = crate::tui::test_support::make_test_tui()?;

    app.select_agents_overview_thread(&mut tui, &mut app_server, target.session.thread_id)
        .await?;

    assert!(
        app.agents_overview
            .dispatched_requests
            .contains_key(&idle_child_id)
    );
    app.handle_app_server_event(
        &app_server,
        AppServerEvent::ServerRequest(Box::new(ServerRequest::CurrentTimeRead {
            request_id: RequestId::Integer(99),
            params: CurrentTimeReadParams {
                thread_id: idle_child_id.to_string(),
            },
        })),
    )
    .await;
    assert!(app.agents_overview.dispatched_requests[&idle_child_id].is_empty());
    let response: ThreadUnsubscribeResponse = app_server
        .request_handle()
        .request_typed(ClientRequest::ThreadUnsubscribe {
            request_id: RequestId::String("verify-root-subscription".to_string()),
            params: ThreadUnsubscribeParams {
                thread_id: previous_root_id.to_string(),
            },
        })
        .await?;
    assert_eq!(response.status, ThreadUnsubscribeStatus::Unsubscribed);
    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn overview_selection_applies_user_permissions_only_to_unloaded_threads() -> Result<()> {
    let mut app = make_test_app().await;
    trust_fixture_folders(&mut app);
    std::fs::write(
        app.config.genex_home.join("config.toml"),
        "[tui]\nresume_cwd = \"session\"\n",
    )?;
    let mut server_config = app.config.clone();
    server_config
        .permissions
        .set_permission_profile(PermissionProfile::workspace_write())?;
    server_config
        .permissions
        .approval_policy
        .set(genex_protocol::protocol::AskForApproval::Never)?;
    let mut thread_ids = Vec::new();
    for day in 1..=5 {
        thread_ids.push(ThreadId::from_string(
            &app_test_support::create_fake_rollout(
                &server_config.genex_home,
                &format!("2025-01-{day:02}T12-00-00"),
                &format!("2025-01-{day:02}T12:00:00Z"),
                "Historical session",
                /*git_info*/ None,
            )
            .expect("create historical session"),
        )?);
    }
    let mut app_server =
        Box::pin(crate::start_embedded_app_server_for_picker(&server_config)).await?;
    let loaded = app_server
        .resume_thread(
            &crate::local_settings::LocalSettings::from(&server_config),
            server_config.clone(),
            thread_ids[0],
            crate::app_server_session::ResumeModelSettings::RestoreFromThread,
        )
        .await?
        .session;
    app.harness_overrides.sandbox_mode = Some(genex_protocol::config_types::SandboxMode::ReadOnly);
    app.harness_overrides.approval_policy =
        Some(genex_protocol::protocol::AskForApproval::UnlessTrusted);
    app.config
        .permissions
        .set_permission_profile(PermissionProfile::read_only())?;
    app.runtime_permission_profile_override =
        Some(RuntimePermissionProfileOverride::from_config(&app.config));
    app.runtime_approval_policy_override = Some(RuntimeApprovalPolicyOverride::Explicit(
        AskForApproval::UnlessTrusted,
    ));
    let mut tui = crate::tui::test_support::make_test_tui()?;
    for (thread_id, expected_permissions, expected_approval) in [
        (
            loaded.thread_id,
            loaded.permission_profile,
            loaded.approval_policy,
        ),
        (
            thread_ids[1],
            PermissionProfile::read_only(),
            genex_app_server_protocol::AskForApproval::UnlessTrusted,
        ),
        (
            thread_ids[2],
            PermissionProfile::read_only(),
            genex_app_server_protocol::AskForApproval::UnlessTrusted,
        ),
        (
            thread_ids[3],
            PermissionProfile::read_only(),
            genex_app_server_protocol::AskForApproval::UnlessTrusted,
        ),
    ] {
        if thread_id == thread_ids[2] {
            // A session-only /permissions choice must survive subsequent cold resumes.
            std::fs::write(
                app.config.genex_home.join("config.toml"),
                "approvals_reviewer = \"auto_review\"\n[tui]\nresume_cwd = \"session\"\n",
            )?;
            app.harness_overrides.sandbox_mode =
                Some(genex_protocol::config_types::SandboxMode::WorkspaceWrite);
            app.harness_overrides.approval_policy =
                Some(genex_protocol::protocol::AskForApproval::Never);
            app.runtime_permission_profile_override =
                Some(RuntimePermissionProfileOverride::from_config(&app.config));
            app.runtime_approval_policy_override = Some(RuntimeApprovalPolicyOverride::Explicit(
                AskForApproval::UnlessTrusted,
            ));
        }
        app.select_agents_overview_thread(&mut tui, &mut app_server, thread_id)
            .await?;
        let observed = app_server
            .resume_thread(
                &app.local_settings,
                app.config.clone(),
                thread_id,
                crate::app_server_session::ResumeModelSettings::PreserveExistingThread,
            )
            .await?
            .session;
        assert_eq!(
            (
                app.primary_thread_id,
                observed.permission_profile,
                observed.approval_policy,
                observed.approvals_reviewer,
            ),
            (
                Some(thread_id),
                expected_permissions,
                expected_approval,
                ApprovalsReviewer::User
            ),
        );
    }
    let requirements = app.config.genex_home.join("requirements.toml");
    std::fs::write(
        &requirements,
        "allowed_approvals_reviewers = [\"auto_review\"]\n",
    )?;
    app.loader_overrides.system_requirements_path = Some(requirements.to_path_buf());
    app.select_agents_overview_thread(&mut tui, &mut app_server, thread_ids[4])
        .await?;
    assert_eq!(
        (
            app.primary_thread_id,
            app_server
                .thread_read(thread_ids[4], /*include_turns*/ false)
                .await?
                .status
        ),
        (
            Some(thread_ids[3]),
            genex_app_server_protocol::ThreadStatus::NotLoaded
        ),
    );
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn restored_server_permission_profile_survives_cd_without_turn_override() -> Result<()> {
    let mut app = make_test_app().await;
    let destination = app.config.genex_home.join("destination");
    std::fs::create_dir(&destination)?;
    crate::legacy_core::config::set_project_trust_level(
        app.config.genex_home.as_path(),
        &destination,
        genex_protocol::config_types::TrustLevel::Trusted,
    )
    .map_err(|error| color_eyre::eyre::eyre!(error.to_string()))?;
    let mut app_server =
        crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref()).await?;
    let previous = app_server.start_thread(&app.config).await?;
    app.enqueue_primary_thread_session(previous.session, previous.turns)
        .await?;
    let target = app_server.start_thread(&app.config).await?;
    let target_thread_id = target.session.thread_id;
    let mut tui = crate::tui::test_support::make_test_tui()?;

    app.select_agents_overview_thread(&mut tui, &mut app_server, target_thread_id)
        .await?;
    app.config
        .permissions
        .set_permission_profile(PermissionProfile::read_only())?;
    app.chat_widget.set_permission_profile_with_active_profile(
        PermissionProfile::read_only(),
        /*active_permission_profile*/ None,
    )?;
    app.runtime_permission_profile_override = Some(
        RuntimePermissionProfileOverride::from_restored_config(app.chat_widget.config_ref()),
    );

    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .effective_permission_profile(),
        PermissionProfile::read_only()
    );
    assert_eq!(
        app.runtime_permission_profile_override
            .as_ref()
            .map(|profile| profile.turn_override),
        Some(RuntimePermissionProfileTurnOverride::Preserve)
    );
    assert_eq!(
        App::turn_permissions_override_from_config(
            app.chat_widget.config_ref(),
            /*active_permission_profile*/ None,
            app.runtime_permission_profile_override
                .as_ref()
                .and_then(RuntimePermissionProfileOverride::turn_permission_profile),
        ),
        TurnPermissionsOverride::Preserve
    );

    app.change_working_directory(&mut tui, &mut app_server, destination.clone())
        .await;

    assert_eq!(app.chat_widget.config_ref().cwd, destination);
    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .effective_permission_profile(),
        PermissionProfile::read_only()
    );

    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn cancelling_resume_picker_preserves_command_center_state() -> Result<()> {
    for primary_thread_id in [None, Some(ThreadId::new())] {
        let mut app = make_test_app().await;
        app.config.disable_paste_burst = true;
        let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
        app.app_event_tx = crate::app_event_sender::AppEventSender::new(event_tx);
        app.primary_thread_id = primary_thread_id;
        let threads = ["First task", "Second task"].map(|name| {
            overview_thread(
                ThreadId::new(),
                /*parent_thread_id*/ None,
                name,
                ThreadStatus::Idle,
            )
        });
        let selected = ThreadId::from_string(&threads[1].id).unwrap();
        let view = app.agents_overview_view(threads.into(), Some(selected));
        app.chat_widget.show_bottom_pane_view(Box::new(view));
        for key in [
            KeyCode::Esc.into(),
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE),
            KeyCode::Esc.into(),
        ] {
            app.chat_widget.handle_key_event(key);
        }
        let before = render_bottom_popup(&app.chat_widget, /*width*/ 96);
        let selection = app
            .chat_widget
            .selected_index_for_present_view(AGENTS_OVERVIEW_VIEW_ID);
        while event_rx.try_recv().is_ok() {}
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        assert!(matches!(
            event_rx.try_recv(),
            Ok(AppEvent::OpenResumePicker)
        ));
        assert!(event_rx.try_recv().is_err());
        let mut tui = crate::tui::test_support::make_test_tui()?;
        let mut server = Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;
        for selection in [SessionSelection::StartFresh, SessionSelection::Exit] {
            assert!(matches!(
                app.apply_resume_picker_selection(&mut tui, &mut server, selection)
                    .await?,
                AppRunControl::Continue
            ));
            app.chat_widget.pre_draw_tick();
            assert_eq!(render_bottom_popup(&app.chat_widget, /*width*/ 96), before);
        }
        server.shutdown().await?;
        assert_eq!(
            app.chat_widget
                .selected_index_for_present_view(AGENTS_OVERVIEW_VIEW_ID),
            selection
        );
    }
    Ok(())
}

#[tokio::test]
async fn command_center_cursor_tracks_wrapped_footer() {
    let app = make_test_app().await;
    let mut view = app.agents_overview_view(
        vec![overview_thread(
            ThreadId::new(),
            /*parent_thread_id*/ None,
            "Task",
            ThreadStatus::Idle,
        )],
        /*selected_thread_id*/ None,
    );
    for (key, label) in [('f', "Search ›"), ('r', "Rename ›")] {
        view.handle_key_event(KeyCode::Esc.into());
        view.handle_key_event(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
        for width in [48, 96, 120] {
            let area =
                ratatui::layout::Rect::new(/*x*/ 0, /*y*/ 0, width, /*height*/ 24);
            let mut buffer = ratatui::buffer::Buffer::empty(area);
            view.render(area, &mut buffer);
            let prompt_row = buffer
                .content()
                .chunks(usize::from(width))
                .position(|row| {
                    row.iter()
                        .map(ratatui::buffer::Cell::symbol)
                        .collect::<String>()
                        .contains(label)
                })
                .expect("rendered prompt");
            assert_eq!(
                view.cursor_pos(area).map(|(_, y)| usize::from(y)),
                Some(prompt_row)
            );
            assert!(area.contains(view.cursor_pos(area).unwrap().into()));
        }
    }
}


#[tokio::test]
async fn resuming_active_session_closes_command_center() -> Result<()> {
    let mut app = make_test_app().await;
    let thread_id = ThreadId::new();
    app.active_thread_id = Some(thread_id);
    app.primary_thread_id = Some(thread_id);
    let view = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
    app.chat_widget.show_bottom_pane_view(Box::new(view));
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let mut server = Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;
    assert!(matches!(
        app.apply_resume_picker_selection(
            &mut tui,
            &mut server,
            SessionSelection::Resume(SessionTarget {
                path: None,
                thread_id,
                cwd: None,
                history_mode: None,
            })
        )
        .await?,
        AppRunControl::Continue
    ));
    assert_eq!(
        app.chat_widget
            .selected_index_for_present_view(AGENTS_OVERVIEW_VIEW_ID),
        None
    );
    server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn resume_picker_round_trip_preserves_each_threads_input() -> Result<()> {
    let mut app = make_test_app().await;
    trust_fixture_folders(&mut app);
    std::fs::write(
        app.config.genex_home.join("config.toml"),
        "[tui]\nresume_cwd = \"current\"\n",
    )?;
    let mut targets = Vec::new();
    for (timestamp, name) in [
        ("2025-01-05T12-00-00", "First task"),
        ("2025-01-05T13-00-00", "Second task"),
    ] {
        let id = app_test_support::create_fake_rollout(
            app.config.genex_home.as_path(),
            timestamp,
            "2025-01-05T12:00:00Z",
            name,
            /*git_info*/ None,
        )
        .expect("saved rollout");
        targets.push(SessionTarget {
            path: Some(app_test_support::rollout_path(
                app.config.genex_home.as_path(),
                timestamp,
                &id,
            )),
            thread_id: ThreadId::from_string(&id)?,
            cwd: None,
            history_mode: None,
        });
    }
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let mut server = Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;
    let mut expected_states = Vec::new();
    for target in targets.iter().chain(&targets) {
        let view = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
        app.chat_widget.show_bottom_pane_view(Box::new(view));
        app.apply_resume_picker_selection(
            &mut tui,
            &mut server,
            SessionSelection::Resume(target.clone()),
        )
        .await?;
        assert_eq!(app.chat_widget.thread_id(), Some(target.thread_id));
        if expected_states.len() < targets.len() {
            assert_eq!(app.chat_widget.composer_text_with_pending(), "");
            assert!(app.chat_widget.queued_user_message_texts().is_empty());
            app.chat_widget.handle_server_notification(
                ServerNotification::TurnStarted(
                    genex_app_server_protocol::TurnStartedNotification {
                        thread_id: target.thread_id.to_string(),
                        turn: genex_app_server_protocol::Turn {
                            id: "turn-with-follow-up".to_string(),
                            items_view: genex_app_server_protocol::TurnItemsView::Full,
                            items: Vec::new(),
                            status: genex_app_server_protocol::TurnStatus::InProgress,
                            error: None,
                            started_at: None,
                            completed_at: None,
                            duration_ms: None,
                        },
                    },
                ),
                /*replay_kind*/ None,
            );
            let follow_up = format!("Follow-up for {}", target.thread_id);
            app.chat_widget.apply_external_edit(follow_up.clone());
            app.chat_widget
                .handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
            assert_eq!(app.chat_widget.queued_user_message_texts(), vec![follow_up]);
            app.chat_widget
                .apply_external_edit(format!("Draft for {}", target.thread_id));
            let mut input_state = app.chat_widget.capture_thread_input_state().unwrap();
            // Recovered follow-ups stay paused until the user explicitly submits them.
            input_state.recovered_queue = true;
            app.chat_widget.restore_thread_input_state(
                Some(input_state),
                crate::chatwidget::ThreadInputStateRestoreMode {
                    preserve_in_flight_turn: false,
                },
            );
            expected_states.push(app.chat_widget.capture_thread_input_state());
        } else {
            let index = targets
                .iter()
                .position(|t| t.thread_id == target.thread_id)
                .unwrap();
            assert_eq!(
                app.chat_widget.capture_thread_input_state(),
                expected_states[index]
            );
        }
    }
    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn command_center_handles_resume_failure_and_success() -> Result<()> {
    let mut app = make_test_app().await;
    trust_fixture_folders(&mut app);
    std::fs::write(
        app.config.genex_home.join("config.toml"),
        "[tui]\nresume_cwd = \"current\"\n",
    )?;
    let timestamp = "2025-01-05T12-00-00";
    let id = app_test_support::create_fake_rollout(
        app.config.genex_home.as_path(),
        timestamp,
        "2025-01-05T12:00:00Z",
        "Saved task",
        /*git_info*/ None,
    )
    .expect("saved rollout");
    let thread_id = ThreadId::from_string(&id)?;
    let path = app_test_support::rollout_path(app.config.genex_home.as_path(), timestamp, &id);
    let view = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
    app.chat_widget.show_bottom_pane_view(Box::new(view));
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let mut server = Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;
    assert!(matches!(
        app.apply_resume_picker_selection(
            &mut tui,
            &mut server,
            SessionSelection::Resume(SessionTarget {
                path: None,
                thread_id: ThreadId::new(),
                cwd: None,
                history_mode: None,
            })
        )
        .await?,
        AppRunControl::Continue
    ));
    assert!(
        app.chat_widget
            .selected_index_for_present_view(AGENTS_OVERVIEW_VIEW_ID)
            .is_some()
    );
    assert!(
        render_bottom_popup(&app.chat_widget, /*width*/ 96).contains("Unable to resume session")
    );
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(
        app.apply_resume_picker_selection(
            &mut tui,
            &mut server,
            SessionSelection::Resume(SessionTarget {
                path: Some(path),
                thread_id,
                cwd: None,
                history_mode: None,
            })
        )
        .await?,
        AppRunControl::Continue
    ));
    assert_eq!(app.chat_widget.thread_id(), Some(thread_id));
    assert_eq!(
        app.chat_widget
            .selected_index_for_present_view(AGENTS_OVERVIEW_VIEW_ID),
        None
    );
    server.shutdown().await?;
    Ok(())
}



#[tokio::test]
async fn command_center_action_failures_remain_visible() -> Result<()> {
    let mut app = Box::pin(make_test_app()).await;
    let mut server = Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let thread_id = ThreadId::new();
    let view = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
    app.chat_widget.show_bottom_pane_view(Box::new(view));
    for (event, expected) in [
        (
            AppEvent::SelectAgentsOverviewThread { thread_id },
            "is unavailable",
        ),
        (
            AppEvent::RenameAgentsOverviewThread {
                thread_id,
                name: "New name".into(),
            },
            "Failed to rename task",
        ),
        (
            AppEvent::StopAgentsOverviewThread { thread_id },
            "Failed to stop background task",
        ),
    ] {
        Box::pin(app.handle_event(&mut tui, &mut server, event)).await?;
        let rendered = render_bottom_popup(&app.chat_widget, /*width*/ 96);
        assert!(rendered.contains("Unable to complete action"), "{rendered}");
        assert!(rendered.contains(expected), "{rendered}");
        app.chat_widget.handle_key_event(KeyCode::Esc.into());
        assert!(
            render_bottom_popup(&app.chat_widget, /*width*/ 96).contains("Agent command center")
        );
    }
    // The retained primary task can also fail in the nested agent-attachment path.
    app.primary_thread_id = Some(thread_id);
    Box::pin(app.handle_event(
        &mut tui,
        &mut server,
        AppEvent::SelectAgentsOverviewThread { thread_id },
    ))
    .await?;
    let rendered = render_bottom_popup(&app.chat_widget, /*width*/ 96);
    assert!(rendered.contains("Unable to complete action"), "{rendered}");
    assert!(rendered.contains("is no longer available"), "{rendered}");
    server.shutdown().await?;
    Ok(())
}
#[path = "agents_overview_actions_tests.rs"]
mod actions;

fn trust_fixture_folders(app: &mut App) {
    let projects = serde_json::json!({
        test_path_buf("/").display().to_string(): {"trust_level": "trusted"},
        app.config.cwd.display().to_string(): {"trust_level": "trusted"},
    });
    app.cli_kv_overrides.push((
        "projects".into(),
        toml::Value::try_from(projects).expect("trust fixture"),
    ));
}


#[tokio::test]
async fn command_center_escape_cancels_editors_and_never_closes_list() {
    for (vim, offline, empty, running) in [
        (false, false, false, false),
        (true, false, false, false),
        (false, true, false, false),
        (false, false, true, false),
        (false, false, false, true),
    ] {
        let (mut app, mut rx, mut op_rx) = crate::app::tests::make_test_app_with_channels().await;
        if vim {
            app.chat_widget.toggle_vim_mode_and_notify();
        }
        if running {
            app.chat_widget.handle_server_notification(
                ServerNotification::TurnStarted(
                    genex_app_server_protocol::TurnStartedNotification {
                        thread_id: ThreadId::new().to_string(),
                        turn: genex_app_server_protocol::Turn {
                            id: "running".into(),
                            items_view: genex_app_server_protocol::TurnItemsView::Full,
                            items: Vec::new(),
                            status: genex_app_server_protocol::TurnStatus::InProgress,
                            error: None,
                            started_at: None,
                            completed_at: None,
                            duration_ms: None,
                        },
                    },
                ),
                /*replay_kind*/ None,
            );
            assert!(app.chat_widget.is_task_running_for_test());
        }
        while rx.try_recv().is_ok() {}
        let rows = if empty {
            Vec::new()
        } else {
            vec![overview_thread(
                ThreadId::new(),
                /*parent_thread_id*/ None,
                "Task",
                ThreadStatus::Idle,
            )]
        };
        let mut view = app.agents_overview_view(rows, /*selected_thread_id*/ None);
        if offline {
            app.agents_overview
                .view_state
                .lock()
                .unwrap()
                .connection_notice = Some("Offline");
        }
        view.handle_key_event(KeyCode::Char('f').into());
        view.handle_paste("nwogrxfha".into());
        assert!(rx.try_recv().is_err());
        view.handle_key_event(KeyCode::Esc.into());
        if !empty {
            assert_eq!(view.selected_index(), Some(0));
        }
        assert_eq!(
            view.on_ctrl_c(),
            crate::bottom_pane::CancellationEvent::NotHandled
        );
        app.chat_widget.show_bottom_pane_view(Box::new(view));
        for _ in 0..3 {
            app.chat_widget.handle_key_event(KeyCode::Esc.into());
        }
        assert!(
            app.chat_widget
                .selected_index_for_present_view(AGENTS_OVERVIEW_VIEW_ID)
                .is_some()
        );
        assert!(rx.try_recv().is_err());
        app.chat_widget
            .handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(op_rx.try_recv().is_err());
        assert!(matches!(
            rx.try_recv(),
            Ok(AppEvent::Exit(crate::app_event::ExitMode::ShutdownFirst))
        ));
    }
}

#[tokio::test]
async fn command_center_new_actions_use_selection_and_leave_metadata_text_alone() {
    let mut app = make_test_app().await;
    app.config.features.enable(Feature::Worktrees).unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    app.app_event_tx = AppEventSender::new(tx);
    let id = ThreadId::new();
    let mut target = overview_thread(
        id,
        /*parent_thread_id*/ None,
        "Task",
        ThreadStatus::Idle,
    );
    target.cwd = test_path_buf("/tmp/checkout/subdir").abs();
    let mut view = app.agents_overview_view(vec![target.clone()], Some(id));
    for _ in 0..3 {
        view.handle_key_event(KeyCode::Char('n').into());
        assert!(
            matches!(rx.try_recv(), Ok(AppEvent::NewAgentsOverviewSession { cwd: Some(cwd) }) if cwd == target.cwd)
        );
        view.handle_key_event(KeyCode::Char('w').into());
        assert!(
            matches!(rx.try_recv(), Ok(AppEvent::NewAgentsOverviewWorktree { cwd: Some(cwd) }) if cwd == target.cwd)
        );
        view.handle_key_event(KeyCode::Char('g').into());
    }
    view.handle_key_event(KeyCode::Char('r').into());
    for character in "nwogrxfha".chars() {
        view.handle_key_event(KeyCode::Char(character).into());
    }
    assert!(rx.try_recv().is_err());
    view.handle_key_event(KeyCode::Enter.into());
    assert!(
        matches!(rx.try_recv(), Ok(AppEvent::RenameAgentsOverviewThread { name, .. }) if name.ends_with("nwogrxfha"))
    );
    let mut empty = app.agents_overview_view(Vec::new(), /*selected_thread_id*/ None);
    empty.handle_key_event(KeyCode::Char('n').into());
    assert!(matches!(
        rx.try_recv(),
        Ok(AppEvent::NewAgentsOverviewSession { cwd: None })
    ));
    app.config.features.disable(Feature::Worktrees).unwrap();
    let mut disabled = app.agents_overview_view(vec![target], Some(id));
    disabled.handle_key_event(KeyCode::Char('w').into());
    assert!(rx.try_recv().is_err());
}
