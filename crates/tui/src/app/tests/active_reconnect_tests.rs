//! Active recovery keeps local input and policy while reusing ordinary thread resume.

use super::*;
use crate::app::reconnect::ReconnectPresentation;
use crate::app::reconnect::reconnect;
use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::net::TcpListener;

use super::disconnect::serve_reconnect_requests;


#[tokio::test]
async fn reconnect_reconciles_offscreen_pending_profile_before_restoring_permissions() -> Result<()>
{
    let (mut app, mut events, _) = make_test_app_with_channels().await;
    let primary = ThreadId::new();
    let displayed = ThreadId::new();
    let cwd = app.config.cwd.to_path_buf();
    app.config
        .permissions
        .set_permission_profile(PermissionProfile::Disabled)?;
    app.runtime_permission_profile_override =
        Some(RuntimePermissionProfileOverride::from_config(&app.config));
    app.primary_thread_id = Some(primary);
    app.active_thread_id = Some(displayed);
    for id in [primary, displayed] {
        let mut session = test_thread_session(id, cwd.clone());
        session.permission_profile = PermissionProfile::Disabled;
        app.ensure_thread_channel(id)
            .store
            .lock()
            .await
            .set_session(session.clone(), Vec::new());
        if id == primary {
            app.primary_session_configured = Some(session);
            app.upsert_agent_picker_thread(
                id, /*agent_nickname*/ None, /*agent_role*/ None,
                /*is_closed*/ false,
            );
        } else {
            app.chat_widget.handle_thread_session(session);
        }
    }
    app.pending_server_profiles.insert(
        primary,
        PermissionProfileSelection {
            profile_id: "server-only".into(),
            approval_policy: None,
            approvals_reviewer: None,
            display_label: "server-only".into(),
        },
    );

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    app.app_server_target = AppServerTarget::Remote {
        endpoint: crate::resolve_remote_addr(&format!("ws://{}", listener.local_addr()?))?,
    };
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        serve_reconnect_requests(tokio_tungstenite::accept_async(stream).await?, |request| {
            let id = request.params.as_ref().and_then(|params| params["threadId"].as_str());
            let thread = |id: ThreadId| json!({
                "id": id, "sessionId": id, "preview": "task", "ephemeral": false,
                "modelProvider": "test-provider", "createdAt": 1, "updatedAt": 2,
                "status": {"type": "idle"}, "cwd": cwd, "cliVersion": "0.0.0",
                "source": "cli", "turns": []
            });
            std::future::ready(Some(match request.method.as_str() {
                "thread/resume" => {
                    let pending = id == Some(primary.to_string().as_str());
                    json!({"result": {"thread": thread(if pending { primary } else { displayed }),
                        "model": "gpt-test", "modelProvider": "test-provider", "cwd": cwd,
                        "approvalPolicy": if pending { "on-request" } else { "never" },
                        "approvalsReviewer": "user",
                        "sandbox": {"type": if pending { "readOnly" } else { "dangerFullAccess" }},
                        "activePermissionProfile": if pending { Some(json!({"id": "server-only"})) } else { None },
                        "reasoningEffort": null}})
                }
                "thread/read" => json!({"result": {"thread": thread(primary)}}),
                "turn/start" => {
                    let params = request.params.as_ref().unwrap();
                    assert_eq!(params["permissions"], "server-only");
                    assert_eq!(params["approvalPolicy"], "on-request");
                    assert_eq!(params["sandboxPolicy"], json!(null));
                    json!({"result": {"turn": {"id": "fresh", "items": [], "status": "inProgress"}}})
                }
                "thread/list" | "thread/loaded/list" =>
                    json!({"result": {"data": [], "nextCursor": null}}),
                "thread/goal/get" => json!({"result": {"goal": null}}),
                method => panic!("unexpected reconnect request: {method}"),
            }))
        })
        .await
    });
    let mut session = crate::start_embedded_app_server_for_picker(&app.config).await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    app.begin_reconnect();
    let connected = reconnect(
        app.app_server_target.clone(),
        app.config.clone(),
        app.local_settings.clone(),
        Some(displayed),
        /*remote_cwd*/ None,
        session.thread_tool_transport(),
        ReconnectPresentation::Conversation,
    )
    .await?;
    app.finish_reconnect(
        &mut tui,
        &mut session,
        &mut events,
        connected,
        CODEX_CLI_VERSION,
    )
    .await?;
    assert!(app.pending_server_profiles.contains_key(&primary));

    app.select_agent_thread(&mut tui, &mut session, primary)
        .await?;
    assert!(!app.thread_unavailable(primary));
    assert!(!app.pending_server_profiles.contains_key(&primary));
    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .permission_profile(),
        &PermissionProfile::read_only()
    );
    assert_eq!(
        app.chat_widget
            .config_ref()
            .permissions
            .active_permission_profile()
            .as_ref()
            .map(|profile| profile.id.as_str()),
        Some("server-only")
    );
    app.chat_widget
        .restore_user_message_to_composer("safe follow-up".into());
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    while let Ok(event) = events.try_recv() {
        if matches!(event, AppEvent::CodexOp(AppCommand::UserTurn { .. })) {
            app.handle_event(&mut tui, &mut session, event).await?;
        }
    }
    session.shutdown().await?;
    let methods = server.await??;
    assert_eq!(
        methods
            .iter()
            .filter(|method| *method == "turn/start")
            .count(),
        1
    );
    Ok(())
}



pub(super) async fn drain_history(
    app: &mut App,
    tui: &mut tui::Tui,
    session: &mut AppServerSession,
    events: &mut mpsc::UnboundedReceiver<AppEvent>,
) -> Result<String> {
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(
            event,
            AppEvent::CodexOp(AppCommand::UserTurn { .. })
        ));
        if matches!(
            event,
            AppEvent::InsertHistoryCell(_)
                | AppEvent::BeginThreadSwitchHistoryReplayBuffer
                | AppEvent::EndInitialHistoryReplayBuffer
        ) {
            app.handle_event(tui, session, event).await?;
        }
    }
    Ok(lines_to_single_string(
        &app.transcript_cells
            .iter()
            .flat_map(|cell| cell.display_lines(/*width*/ 80))
            .collect::<Vec<_>>(),
    ))
}
