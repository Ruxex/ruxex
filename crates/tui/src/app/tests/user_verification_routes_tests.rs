use super::*;
use pretty_assertions::assert_eq;

fn user_verification_server_request(thread_id: ThreadId, request_id: i64) -> ServerRequest {
    ServerRequest::McpServerElicitationRequest {
        request_id: AppServerRequestId::Integer(request_id),
        params: McpServerElicitationRequestParams {
            thread_id: thread_id.to_string(),
            turn_id: Some("turn-verification".to_string()),
            server_name: "deployments".to_string(),
            request: McpServerElicitationRequest::UserVerification {
                title: "Approve deployment?".to_string(),
                description: "Verify the production deployment.".to_string(),
                challenge: "AQID".to_string(),
            },
        },
    }
}

async fn next_submitted_thread_op(
    app_event_rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
) -> (ThreadId, Op) {
    time::timeout(std::time::Duration::from_secs(/*secs*/ 1), async {
        loop {
            match app_event_rx.recv().await {
                Some(AppEvent::SubmitThreadOp { thread_id, op }) => break (thread_id, op),
                Some(_) => {}
                None => panic!("app event channel closed before submitting an operation"),
            }
        }
    })
    .await
    .expect("user-verification operation should be submitted promptly")
}



#[tokio::test]
async fn discarding_side_cancels_verification_and_ignores_late_proof() -> Result<()> {
    let (mut app, mut events, _ops) = make_test_app_with_channels().await;
    let mut app_server =
        crate::start_embedded_app_server_for_picker(app.chat_widget.config_ref()).await?;
    let primary_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    let request = user_verification_server_request(side_thread_id, /*request_id*/ 19);
    app.active_thread_id = Some(primary_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(primary_thread_id));
    app.thread_event_channels
        .insert(side_thread_id, ThreadEventChannel::new(/*capacity*/ 4));
    app.pending_app_server_requests
        .note_server_request(&request);
    let attempt = app
        .pending_app_server_requests
        .user_verification
        .begin("deployments", request.id())
        .expect("pending side-thread verification");

    app.discard_side_thread_in_background(&mut app_server, side_thread_id)
        .await;
    assert!(attempt.cancelled.is_cancelled());
    assert!(
        !app.pending_app_server_requests
            .contains_server_request(&request)
    );

    app.finish_user_verification(
        &mut app_server,
        side_thread_id,
        "deployments".to_string(),
        request.id().clone(),
        attempt.id,
        Ok(genex_app_server_protocol::UserVerificationProof {
            credential_id: "discarded-credential".to_string(),
            signature: "discarded-signature".to_string(),
        }),
    )
    .await?;
    assert!(
        !std::iter::from_fn(|| events.try_recv().ok())
            .any(|event| matches!(event, AppEvent::SubmitThreadOp { .. }))
    );
    app_server.shutdown().await?;
    Ok(())
}


#[tokio::test]
async fn active_thread_user_verification_cancels_through_original_request() {
    let (mut app, mut app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let thread_id = ThreadId::new();
    let request = user_verification_server_request(thread_id, /*request_id*/ 12);

    let ServerRequest::McpServerElicitationRequest { request_id, params } = request else {
        panic!("elicitation request")
    };
    app.chat_widget
        .handle_elicitation_request_now(request_id, params);
    app.chat_widget
        .handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    let (submitted_thread_id, op) = next_submitted_thread_op(&mut app_event_rx).await;
    assert_eq!(submitted_thread_id, thread_id);
    assert_eq!(
        op,
        Op::ResolveUserVerification {
            server_name: "deployments".to_string(),
            request_id: AppServerRequestId::Integer(12),
            response: crate::app_command::UserVerificationResponse::Cancel,
        }
    );
}
