//! Offline editing, uncertain submissions, and stale events never consume local work.

use super::*;
use crate::app_server_session::ThreadParamsMode;
use ruxex_app_server_protocol::JSONRPCMessage;
use ruxex_app_server_protocol::JSONRPCRequest;
use futures::SinkExt;
use futures::StreamExt;
use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;



#[tokio::test]
async fn remote_disconnect_retires_voice_before_reconnecting() -> Result<()> {
    let (mut app, _events, _ops) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    app.active_thread_id = Some(thread_id);
    app.chat_widget
        .handle_thread_session_quiet(test_thread_session(thread_id, app.config.cwd.to_path_buf()));
    app.app_server_target = AppServerTarget::Remote {
        endpoint: crate::resolve_remote_addr("ws://127.0.0.1:9")?,
    };
    crate::chatwidget::activate_voice_for_thread(&mut app.chat_widget, thread_id);
    assert!(app.chat_widget.is_current_realtime_attempt(
        thread_id, /*attempt_id*/ 0, /*input_generation*/ 0
    ));

    assert!(app.begin_reconnect());
    assert_eq!(
        (
            app.reconnect.offline,
            app.chat_widget.is_current_realtime_attempt(
                thread_id, /*attempt_id*/ 0, /*input_generation*/ 0
            ),
        ),
        (true, false)
    );
    assert!(app.begin_reconnect());
    assert!(!app.chat_widget.is_current_realtime_attempt(
        thread_id, /*attempt_id*/ 0, /*input_generation*/ 0
    ));
    Ok(())
}

// Common framing/bootstrap for both transports. Returning None deliberately loses a reply.
pub(super) async fn serve_reconnect_requests<S, F>(
    mut socket: tokio_tungstenite::WebSocketStream<S>,
    mut respond: impl FnMut(JSONRPCRequest) -> F,
) -> Result<Vec<String>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    F: std::future::Future<Output = Option<serde_json::Value>>,
{
    let mut methods = Vec::new();
    while let Some(Ok(frame)) = socket.next().await {
        let Message::Text(text) = frame else { continue };
        let JSONRPCMessage::Request(request) = serde_json::from_str(&text)? else {
            continue;
        };
        methods.push(request.method.clone());
        let request_id = request.id.clone();
        let response = match request.method.as_str() {
            "initialize" => Some(json!({"result": {"userAgent": "reconnect-test/2.0.0"}})),
            "account/read" => {
                Some(json!({"result": {"account": null, "requiresproviderAuth": false}}))
            }
            "model/list" => Some(json!({"result": {"data": [], "nextCursor": null}})),
            "collaborationMode/list" => Some(json!({"result": {"data": []}})),
            "configRequirements/read" => Some(json!({"result": {"requirements": null}})),
            _ => respond(request).await,
        };
        let Some(mut response) = response else {
            socket.close(None).await?;
            break;
        };
        response["id"] = json!(request_id);
        socket
            .send(Message::Text(response.to_string().into()))
            .await?;
    }
    Ok(methods)
}
#[tokio::test]
async fn lost_initial_thread_reply_keeps_startup_draft_offline() -> Result<()> {
    for disconnect_first in [false, true] {
        let (mut app, _events, mut ops) = make_test_app_with_channels().await;
        app.pending_startup_thread_start = true;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let endpoint = crate::resolve_remote_addr(&format!("ws://{}", listener.local_addr()?))?;
        app.app_server_target = AppServerTarget::Remote {
            endpoint: endpoint.clone(),
        };
        app.chat_widget
            .restore_user_message_to_composer("startup draft".into());
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await?;
            serve_reconnect_requests(tokio_tungstenite::accept_async(stream).await?, |request| {
                assert_eq!(request.method, "thread/start");
                std::future::ready(None)
            })
            .await
        });
        let mut session = AppServerSession::new(
            crate::connect_remote_app_server(endpoint).await?,
            ThreadParamsMode::Remote,
        );
        let result = crate::app_server_session::start_thread_with_request_handle(
            session.request_handle(),
            &app.local_settings,
            app.config.clone(),
            ThreadParamsMode::Remote,
            /*remote_cwd_override*/ None,
            session.thread_tool_transport(),
        )
        .await;
        let mut tui = crate::tui::test_support::make_test_tui()?;
        if disconnect_first {
            app.begin_reconnect();
        }
        app.handle_event(
            &mut tui,
            &mut session,
            AppEvent::StartupThreadStarted { result },
        )
        .await?;
        assert!(app.reconnect.offline);
        app.handle_tui_event(
            &mut tui,
            &mut session,
            TuiEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        )
        .await?;
        assert_eq!(
            app.chat_widget.composer_text_with_pending(),
            "startup draft"
        );
        assert!(
            !std::iter::from_fn(|| ops.try_recv().ok()).any(|op| matches!(op, Op::UserTurn { .. }))
        );
        session.shutdown().await?;
        assert_eq!(server.await??, vec!["initialize", "thread/start"]);
    }
    Ok(())
}
