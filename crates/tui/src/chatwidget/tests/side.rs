use super::*;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;


#[tokio::test]
async fn suppressed_interrupted_turn_notice_skips_history_warning() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.set_interrupted_turn_notice_mode(InterruptedTurnNoticeMode::Suppress);
    chat.on_task_started();
    chat.on_agent_message_delta("partial output".to_string());

    chat.on_interrupted_turn(TurnAbortReason::Interrupted);

    let inserted = drain_insert_history(&mut rx);
    assert!(
        inserted.iter().all(|cell| {
            let rendered = lines_to_single_string(cell);
            !rendered.contains("Conversation interrupted - tell the model what to do differently.")
                && !rendered.contains("Model interrupted to submit steer instructions.")
        }),
        "unexpected interrupted-turn notice in side conversation: {inserted:?}"
    );
}

fn assert_side_rename_rejected(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
    op_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Op>,
) {
    let event = rx
        .try_recv()
        .expect("expected side conversation rename error");
    match event {
        AppEvent::InsertHistoryCell(cell) => {
            let rendered = lines_to_single_string(&cell.display_lines(/*width*/ 80));
            assert!(
                rendered.contains("Side conversations are ephemeral and cannot be renamed."),
                "expected side conversation rename error, got {rendered:?}"
            );
        }
        other => panic!("expected InsertHistoryCell error, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "expected no follow-up events");
    assert!(op_rx.try_recv().is_err(), "expected no rename op");
}

#[tokio::test]
async fn slash_rename_is_rejected_for_side_threads() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_thread_rename_block_message(
        "Side conversations are ephemeral and cannot be renamed.".to_string(),
    );

    chat.dispatch_command(SlashCommand::Rename);
    assert_side_rename_rejected(&mut rx, &mut op_rx);
}

#[tokio::test]
async fn slash_rename_with_args_is_rejected_for_side_threads() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_thread_rename_block_message(
        "Side conversations are ephemeral and cannot be renamed.".to_string(),
    );

    chat.dispatch_command_with_args(SlashCommand::Rename, "investigate".to_string(), Vec::new());
    assert_side_rename_rejected(&mut rx, &mut op_rx);
}

#[tokio::test]
async fn slash_commands_without_side_flag_are_rejected_for_side_threads() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_side_conversation_active(/*active*/ true);

    chat.dispatch_command(SlashCommand::Review);

    let event = rx
        .try_recv()
        .expect("expected side conversation slash command error");
    match event {
        AppEvent::InsertHistoryCell(cell) => {
            let rendered = lines_to_single_string(&cell.display_lines(/*width*/ 80));
            assert!(
                rendered.contains(
                    "'/review' is unavailable in side conversations. Press Ctrl+C to return to the main thread first."
                ),
                "expected side conversation slash command error, got {rendered:?}"
            );
        }
        other => panic!("expected InsertHistoryCell error, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "expected no follow-up events");
    assert!(op_rx.try_recv().is_err(), "expected no review op");
}

#[tokio::test]
async fn slash_side_is_rejected_for_side_threads() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.set_side_conversation_active(/*active*/ true);

    chat.dispatch_command(SlashCommand::Side);

    let event = rx
        .try_recv()
        .expect("expected side conversation slash command error");
    match event {
        AppEvent::InsertHistoryCell(cell) => {
            let rendered = lines_to_single_string(&cell.display_lines(/*width*/ 80));
            assert!(
                rendered.contains(
                    "'/side' is unavailable in side conversations. Press Ctrl+C to return to the main thread first."
                ),
                "expected side conversation slash command error, got {rendered:?}"
            );
        }
        other => panic!("expected InsertHistoryCell error, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "expected no follow-up events");
    assert!(
        op_rx.try_recv().is_err(),
        "expected no side conversation op"
    );
}

#[tokio::test]
async fn slash_side_is_rejected_during_review_mode() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.review.is_review_mode = true;

    chat.dispatch_command(SlashCommand::Side);

    let event = rx
        .try_recv()
        .expect("expected review-mode side conversation error");
    match event {
        AppEvent::InsertHistoryCell(cell) => {
            let rendered = lines_to_single_string(&cell.display_lines(/*width*/ 80));
            assert!(
                rendered.contains("'/side' is unavailable while code review is running."),
                "expected review-mode side conversation error, got {rendered:?}"
            );
        }
        other => panic!("expected InsertHistoryCell error, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "expected no follow-up events");
    assert!(
        op_rx.try_recv().is_err(),
        "expected no side conversation op"
    );
}

#[tokio::test]
async fn slash_btw_is_rejected_during_review_mode() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.review.is_review_mode = true;

    chat.dispatch_command(SlashCommand::Btw);

    let event = rx
        .try_recv()
        .expect("expected review-mode btw conversation error");
    match event {
        AppEvent::InsertHistoryCell(cell) => {
            let rendered = lines_to_single_string(&cell.display_lines(/*width*/ 80));
            assert!(
                rendered.contains("'/btw' is unavailable while code review is running."),
                "expected review-mode btw conversation error, got {rendered:?}"
            );
        }
        other => panic!("expected InsertHistoryCell error, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "expected no follow-up events");
    assert!(
        op_rx.try_recv().is_err(),
        "expected no side conversation op"
    );
}

#[tokio::test]
async fn slash_btw_is_rejected_before_the_session_starts() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.dispatch_command(SlashCommand::Btw);

    let event = rx
        .try_recv()
        .expect("expected pre-session btw conversation error");
    match event {
        AppEvent::InsertHistoryCell(cell) => {
            let rendered = lines_to_single_string(&cell.display_lines(/*width*/ 80));
            assert!(
                rendered.contains("'/btw' is unavailable before the session starts."),
                "expected pre-session btw conversation error, got {rendered:?}"
            );
        }
        other => panic!("expected InsertHistoryCell error, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "expected no follow-up events");
    assert!(
        op_rx.try_recv().is_err(),
        "expected no side conversation op"
    );
}

#[tokio::test]
async fn submit_user_message_as_plain_user_turn_does_not_run_shell_commands() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());

    chat.submit_user_message_as_plain_user_turn("!echo hello".into());

    match next_submit_op(&mut op_rx) {
        Op::UserTurn { items, .. } => assert_eq!(
            items,
            vec![UserInput::Text {
                text: "!echo hello".to_string(),
                text_elements: Vec::new(),
            }]
        ),
        other => {
            panic!("expected Op::UserTurn for side-conversation shell-like input, got {other:?}")
        }
    }
}

#[tokio::test]
async fn slash_side_without_args_starts_empty_side_conversation() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let parent_thread_id = ThreadId::new();
    chat.thread_id = Some(parent_thread_id);
    chat.on_task_started();
    chat.bottom_pane
        .set_composer_text("/side".to_string(), Vec::new(), Vec::new());

    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_matches!(
        rx.try_recv(),
        Ok(AppEvent::StartSide {
            parent_thread_id: emitted_parent_thread_id,
            user_message: None,
        }) if emitted_parent_thread_id == parent_thread_id
    );
    assert!(
        op_rx.try_recv().is_err(),
        "bare /side should not submit an op on the parent thread"
    );
    assert!(chat.input_queue.queued_user_messages.is_empty());
}

#[tokio::test]
async fn slash_btw_without_args_starts_empty_side_conversation() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let parent_thread_id = ThreadId::new();
    chat.thread_id = Some(parent_thread_id);
    chat.on_task_started();
    chat.bottom_pane
        .set_composer_text("/btw".to_string(), Vec::new(), Vec::new());

    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_matches!(
        rx.try_recv(),
        Ok(AppEvent::StartSide {
            parent_thread_id: emitted_parent_thread_id,
            user_message: None,
        }) if emitted_parent_thread_id == parent_thread_id
    );
    assert!(
        op_rx.try_recv().is_err(),
        "bare /btw should not submit an op on the parent thread"
    );
    assert!(chat.input_queue.queued_user_messages.is_empty());
}


#[tokio::test]
async fn slash_btw_requests_forked_side_question_while_task_running() {
    let (mut chat, mut rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let parent_thread_id = ThreadId::new();
    chat.thread_id = Some(parent_thread_id);
    chat.on_task_started();
    chat.bottom_pane.set_composer_text(
        "/btw explore the codebase".to_string(),
        Vec::new(),
        Vec::new(),
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_matches!(
        rx.try_recv(),
        Ok(AppEvent::StartSide {
            parent_thread_id: emitted_parent_thread_id,
            user_message: Some(user_message),
        }) if emitted_parent_thread_id == parent_thread_id
            && user_message
                == UserMessage {
                    text: "explore the codebase".to_string(),
                    local_images: Vec::new(),
                    remote_image_urls: Vec::new(),
                    text_elements: Vec::new(),
                    mention_bindings: Vec::new(),
                }
    );
    assert!(
        op_rx.try_recv().is_err(),
        "expected no op on the parent thread"
    );
}
