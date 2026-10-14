//! Verify question input delivery, retained drafts, and visible editor behavior.
use super::*;
use ruxex_protocol::items::AsyncUserInputQuestion;
use pretty_assertions::assert_eq;

fn questions() -> Vec<AsyncUserInputQuestion> {
    vec![
        question("Which way?", /*options*/ None),
        question("Any details?", /*options*/ None),
    ]
}

#[tokio::test]
async fn unavailable_send_keeps_answer_and_skip_remains_available() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.add_async_questions("message", &questions());
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    chat.bottom_pane.handle_paste("kept answer".into());
    chat.blocks_direct_input = true;
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_eq!(question_count(&chat), 2);
    chat.handle_key_event(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL));
    assert_eq!(question_count(&chat), 1);
}



#[tokio::test]
async fn local_shell_command_preserves_unanswered_questions() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.add_async_questions("old", &questions());
    chat.bottom_pane
        .set_composer_text("!echo hi".into(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    assert_eq!(question_count(&chat), 2);
    assert_matches!(
        op_rx.try_recv(),
        Ok(Op::RunUserShellCommand { command }) if command == "echo hi"
    );
}

#[tokio::test]
async fn queued_prompt_clears_questions_arriving_after_enqueue_when_it_starts() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.on_task_started();
    chat.add_async_questions("old", &questions());
    chat.bottom_pane
        .set_composer_text("New prompt".into(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::from(KeyCode::Tab));
    assert_eq!(question_count(&chat), 0);

    chat.add_async_questions("late", &questions());
    assert_eq!(question_count(&chat), 2);
    chat.turn_lifecycle.finish();
    chat.update_task_running_state();
    assert!(chat.maybe_send_next_queued_input());
    assert_answer(op_rx.try_recv().unwrap(), "New prompt");
    assert_eq!(question_count(&chat), 0);
    chat.add_async_questions("late", &questions());
    assert_eq!(question_count(&chat), 0);
}

#[tokio::test]
async fn queued_question_answer_preserves_other_questions_on_delivery() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.input_queue.suppress_queue_autosend = true;
    chat.add_async_questions("old", &questions());
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    chat.bottom_pane.handle_paste("answer".into());
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_eq!(question_count(&chat), 1);

    chat.input_queue.suppress_queue_autosend = false;
    assert!(chat.maybe_send_next_queued_input());
    assert_answer(op_rx.try_recv().unwrap(), "> Which way?\n\nanswer");
    assert_eq!(question_count(&chat), 1);
}

#[tokio::test]
async fn recovered_question_answers_preserve_other_questions() {
    for restore_thread in [false, true] {
        let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        chat.thread_id = Some(ThreadId::new());
        chat.on_task_started();
        chat.add_async_questions("old", &questions());
        chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
        chat.bottom_pane.handle_paste("answer".into());
        chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
        assert_answer(op_rx.try_recv().unwrap(), "> Which way?\n\nanswer");
        assert_eq!(question_count(&chat), 1);
        chat.add_async_questions("late", &[question("Late?", /*options*/ None)]);

        if restore_thread {
            let state = chat.capture_thread_input_state().unwrap();
            chat.restore_thread_input_state(
                Some(state),
                ThreadInputStateRestoreMode {
                    preserve_in_flight_turn: false,
                },
            );
        } else {
            assert!(chat.enqueue_rejected_steer());
            chat.turn_lifecycle.finish();
            chat.update_task_running_state();
        }
        assert!(chat.maybe_send_next_queued_input());
        assert_answer(op_rx.try_recv().unwrap(), "> Which way?\n\nanswer");
        assert_eq!(question_count(&chat), 2);
    }
}

#[tokio::test]
async fn slash_and_mode_prompts_clear_unanswered_questions_after_delivery() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.add_async_questions("old", &questions());
    chat.handle_composer_input_result(
        InputResult::CommandWithArgs(SlashCommand::Plan, "New plan".into(), Vec::new()),
        /*had_modal_or_popup*/ false,
    );
    assert_answer(op_rx.try_recv().unwrap(), "New plan");
    assert_eq!(question_count(&chat), 0);

    chat.input_queue.user_turn_pending_start = false;
    chat.add_async_questions("next", &questions());
    let mode = collaboration_modes::default_mask(chat.model_catalog.as_ref()).unwrap();
    chat.submit_user_message_with_mode("Implement".to_string(), mode);
    assert_answer(op_rx.try_recv().unwrap(), "Implement");
    assert_eq!(question_count(&chat), 0);
}

#[tokio::test]
async fn accepted_review_clears_questions_but_rejected_review_preserves_them() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.add_async_questions("old", &questions());
    chat.handle_composer_input_result(
        InputResult::CommandWithArgs(SlashCommand::Review, "check this".into(), Vec::new()),
        /*had_modal_or_popup*/ false,
    );
    assert_matches!(
        op_rx.try_recv(),
        Ok(Op::Review {
            target: ReviewTarget::Custom { instructions }
        }) if instructions == "check this"
    );
    assert_eq!(question_count(&chat), 0);

    chat.add_async_questions("new", &questions());
    chat.blocks_direct_input = true;
    chat.handle_composer_input_result(
        InputResult::CommandWithArgs(SlashCommand::Review, "try again".into(), Vec::new()),
        /*had_modal_or_popup*/ false,
    );
    assert_eq!(question_count(&chat), 2);
    assert!(op_rx.try_recv().is_err());
}

#[tokio::test]
async fn queued_model_slash_prompt_clears_questions_but_local_slash_command_does_not() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.on_task_started();
    chat.add_async_questions("old", &questions());
    for text in ["/model", "/goal clear"] {
        chat.handle_composer_input_result(
            InputResult::Queued {
                text: text.into(),
                text_elements: Vec::new(),
                action: QueuedInputAction::ParseSlash,
                pending_pastes: Vec::new(),
            },
            /*had_modal_or_popup*/ false,
        );
        assert_eq!(question_count(&chat), 2);
    }
    chat.handle_composer_input_result(
        InputResult::Queued {
            text: "/plan New work".into(),
            text_elements: Vec::new(),
            action: QueuedInputAction::ParseSlash,
            pending_pastes: Vec::new(),
        },
        /*had_modal_or_popup*/ false,
    );
    assert_eq!(question_count(&chat), 0);
    assert!(op_rx.try_recv().is_err());

    chat.add_async_questions("new", &questions());
    chat.handle_composer_input_result(
        InputResult::Queued {
            text: "/diff explain the change".into(),
            text_elements: Vec::new(),
            action: QueuedInputAction::ParseSlash,
            pending_pastes: Vec::new(),
        },
        /*had_modal_or_popup*/ false,
    );
    assert_eq!(question_count(&chat), 0);
}

#[tokio::test]
async fn rejected_synchronous_literal_prompt_keeps_questions() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.add_async_questions("old", &questions());
    chat.set_model("");
    chat.handle_composer_input_result(
        InputResult::Queued {
            text: "literal prompt".into(),
            text_elements: Vec::new(),
            action: QueuedInputAction::Literal,
            pending_pastes: Vec::new(),
        },
        /*had_modal_or_popup*/ false,
    );
    assert_eq!(question_count(&chat), 2);
    assert!(op_rx.try_recv().is_err());
}

#[tokio::test]
async fn reconnect_preserves_buffered_question_answer_source() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.add_async_questions("old", &questions());
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    chat.bottom_pane.handle_paste("answer".into());
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_answer(op_rx.try_recv().unwrap(), "> Which way?\n\nanswer");
    assert_eq!(question_count(&chat), 1);
    chat.pause_for_disconnect();
    let state = chat.capture_thread_input_state();

    let (mut restored, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    restored.thread_id = Some(ThreadId::new());
    restored.restore_reconnected_input(state);
    assert_eq!(
        restored
            .input_queue
            .queued_user_messages
            .front()
            .unwrap()
            .source,
        UserMessageSource::QuestionAnswer,
    );
    // Recovered input stays paused until the user elects to retry it.
    restored.input_queue.recovered_queue = false;
    assert!(restored.maybe_send_next_queued_input());
    assert_answer(op_rx.try_recv().unwrap(), "> Which way?\n\nanswer");
    assert_eq!(question_count(&restored), 1);
}

#[tokio::test]
async fn blocked_follow_up_keeps_unanswered_questions() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    chat.add_async_questions("old", &questions());
    chat.set_model("");
    chat.bottom_pane
        .set_composer_text("Blocked prompt".into(), Vec::new(), Vec::new());
    chat.handle_key_event(KeyEvent::from(KeyCode::Enter));

    assert_eq!(question_count(&chat), 2);
    assert!(op_rx.try_recv().is_err());
}

#[tokio::test]
async fn disconnected_questions_remain_editable_without_sending() {
    let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.add_async_questions("message", &questions());
    chat.pause_for_disconnect();
    chat.handle_disconnected_key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    chat.bottom_pane.handle_paste("offline draft".into());
    chat.handle_disconnected_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(question_count(&chat), 2);
    assert!(op_rx.try_recv().is_err());
    chat.handle_disconnected_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL));
    assert_eq!(question_count(&chat), 2);
}

fn question_count(chat: &ChatWidget) -> usize {
    chat.bottom_pane
        .questions
        .as_ref()
        .unwrap()
        .unanswered_count()
}



#[tokio::test]
async fn question_history_search_uses_updated_bindings() {
    for (binding, word) in [("f12", "earlier"), ("ctrl-x r", "persistent")] {
        let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        let thread_id = ThreadId::new();
        chat.thread_id = Some(thread_id);
        chat.bottom_pane
            .set_history_metadata(thread_id, /*log_id*/ 7, /*entry_count*/ 1);
        chat.bottom_pane.record_replayed_user_message_history(
            crate::bottom_pane::HistoryEntry::with_pending_and_remote(
                "[Image #2]earlier answer".into(),
                vec![TextElement::new((0..10).into(), Some("[Image #2]".into()))],
                vec![PathBuf::from("/tmp/image.png")],
                Vec::new(),
                vec!["https://example.com/remote.png".into()],
            ),
        );
        chat.add_async_questions("message", &questions());
        let config = toml::from_str(&format!(
            "[composer]\nhistory_search_previous = '{binding}'"
        ))
        .unwrap();
        let keymap = RuntimeKeymap::from_config(&config).unwrap();
        chat.apply_keymap_update(config, &keymap);
        chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
        chat.bottom_pane.record_replayed_user_message_history(
            crate::bottom_pane::HistoryEntry::new("newest answer".into()),
        );
        for (key, expected) in [
            (KeyCode::Up, "newest answer"),
            (KeyCode::Up, "earlier answer"),
            (KeyCode::Down, "newest answer"),
        ] {
            chat.handle_key_event(KeyEvent::from(key));
            let rendered = render_bottom_popup(&chat, /*width*/ 80);
            assert!(
                rendered.contains(expected),
                "{key:?}: expected {expected}, got {rendered}"
            );
        }
        chat.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        // Chords reach the editor using the dispatch binding resolved by the app.
        let (key, modifiers) = keymap.composer.history_search_previous[0].parts();
        chat.handle_key_event(KeyEvent::new(key, modifiers));
        for ch in word.chars() {
            chat.handle_key_event(KeyEvent::from(KeyCode::Char(ch)));
        }
        if binding != "f12" {
            chat.bottom_pane.on_history_lookup_response(
                crate::app_event::HistoryLookupResponse::Entry {
                    offset: 0,
                    log_id: 7,
                    entry: Some("persistent answer".into()),
                },
            );
        }
        chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
        assert!(op_rx.try_recv().is_err());
        chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
        assert_answer(
            op_rx.try_recv().unwrap(),
            &format!("> Which way?\n\n{word} answer"),
        );
    }
}


#[tokio::test]
async fn async_questions_reject_blank_answers_but_allow_explicit_skip() {
    for options in [None, Some(vec!["Named".to_string()])] {
        let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        chat.thread_id = Some(ThreadId::new());
        chat.add_async_questions("message", &[question("Which way?", options.clone())]);
        chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
        if options.is_some() {
            chat.handle_key_event(KeyEvent::from(KeyCode::Char('2')));
        }
        for text in ["", " \t\n"] {
            chat.bottom_pane.handle_paste(text.into());
            let before = render_bottom_popup(&chat, /*width*/ 80);
            chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
            assert_eq!(render_bottom_popup(&chat, /*width*/ 80), before);
            assert!(chat.input_queue.queued_user_messages.is_empty());
            assert!(op_rx.try_recv().is_err());
        }
        chat.handle_key_event(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::CONTROL));
        assert_eq!(question_count(&chat), 0);
        assert!(op_rx.try_recv().is_err());
    }
}

fn open_questions(chat: &mut ChatWidget, options: Option<Vec<String>>) {
    chat.thread_id = Some(ThreadId::new());
    chat.on_agent_message_item_completed(
        AgentMessageItem {
            id: "review".into(),
            content: Vec::new(),
            phase: None,
            delivery: None,
            questions: Some(vec![
                AsyncUserInputQuestion {
                    title: "First?".into(),
                    options,
                },
                question("Second?", Some(vec!["Next".into()])),
            ]),
        },
        "turn",
        /*from_replay*/ false,
    );
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    render_bottom_popup(chat, /*width*/ 80);
}

#[tokio::test]
async fn question_queue_key_does_not_steer_the_running_turn() {
    let (mut chat, _rx, mut ops) = make_chatwidget_manual(/*model_override*/ None).await;
    open_questions(&mut chat, /*options*/ None);
    chat.on_task_started();
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    chat.bottom_pane.handle_paste("kept".into());
    chat.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
    chat.bottom_pane.handle_paste("  later  ".into());
    chat.handle_key_event(KeyEvent::from(KeyCode::Tab));
    assert_eq!(
        chat.input_queue.queued_user_messages.front().unwrap().text,
        "> First?\n\nlater"
    );
    let mut repeat = KeyEvent::from(KeyCode::Tab);
    repeat.kind = KeyEventKind::Repeat;
    chat.handle_key_event(repeat);
    assert_eq!(question_count(&chat), 1);
    assert!(ops.try_recv().is_err());
}

#[tokio::test]
async fn question_choices_deliver_digits_and_leading_letters() {
    for (key, text, expected) in [
        ('2', "", "Second"),
        ('j', "ust text", "just text"),
        ('k', "eep it", "keep it"),
    ] {
        let (mut chat, _rx, mut ops) = make_chatwidget_manual(/*model_override*/ None).await;
        open_questions(&mut chat, Some(vec!["First".into(), "Second".into()]));
        chat.handle_key_event(KeyEvent::from(KeyCode::Char(key)));
        if !text.is_empty() {
            chat.bottom_pane.handle_paste(text.into());
            chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
        }
        assert_answer(ops.try_recv().unwrap(), &format!("> First?\n\n{expected}"));
        assert_eq!(question_count(&chat), 1);
    }
}

#[tokio::test]
async fn question_key_repeats_do_not_consume_another_question() {
    for key in [KeyCode::Enter, KeyCode::Char('1')] {
        let (mut chat, _rx, mut ops) = make_chatwidget_manual(/*model_override*/ None).await;
        open_questions(&mut chat, Some(vec!["First".into()]));
        chat.handle_key_event(KeyEvent::from(key));
        ops.try_recv().unwrap();
        let mut repeat = KeyEvent::from(key);
        repeat.kind = KeyEventKind::Repeat;
        chat.handle_key_event(repeat);
        assert!(ops.try_recv().is_err());
        assert_eq!(question_count(&chat), 1);
        chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
        assert!(ops.try_recv().is_err());
    }
}


#[tokio::test]
async fn question_drafts_survive_navigation_and_snapshot_replay() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.bottom_pane
        .set_composer_text("main draft".into(), Vec::new(), Vec::new());
    open_questions(&mut chat, /*options*/ None);
    chat.bottom_pane.handle_paste("first draft".into());
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    chat.handle_key_event(KeyEvent::from(KeyCode::Char('2')));
    chat.bottom_pane.handle_paste("second draft".into());
    let saved = chat.capture_thread_input_state();
    let (mut chat, _rx, mut ops) = make_chatwidget_manual(/*model_override*/ None).await;
    let item = AppServerThreadItem::AgentMessage {
        id: "buffered".into(),
        text: String::new(),
        phase: None,
        delivery: None,
        questions: Some(vec![question("Buffered?", /*options*/ None)]),
    };
    for kind in [
        ReplayKind::ResumeInitialMessages,
        ReplayKind::ThreadSnapshot,
    ] {
        chat.replay_thread_item(item.clone(), "turn".into(), kind);
        assert!(chat.bottom_pane.questions.is_none());
    }
    chat.handle_server_notification(
        ServerNotification::ItemCompleted(ItemCompletedNotification {
            item,
            thread_id: "thread".into(),
            turn_id: "turn".into(),
            completed_at_ms: 0,
        }),
        Some(ReplayKind::ThreadSnapshot),
    );
    assert_eq!(question_count(&chat), 1);
    chat.restore_reconnected_input(saved);
    assert!(render_bottom_popup(&chat, /*width*/ 80).contains("second draft"));
    chat.pause_unavailable_thread();
    chat.handle_question_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::CONTROL));
    chat.handle_question_key(KeyEvent::from(KeyCode::Enter));
    assert_eq!(question_count(&chat), 3);
    assert!(ops.try_recv().is_err());
    chat.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
    assert!(render_bottom_popup(&chat, /*width*/ 80).contains("first draft"));
    chat.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
    assert_eq!(chat.bottom_pane.composer_text(), "main draft");
}


fn assert_answer(op: Op, expected: &str) {
    let Op::UserTurn { items, .. } = op else {
        panic!("user turn")
    };
    assert_eq!(
        items,
        vec![UserInput::Text {
            text: expected.into(),
            text_elements: Vec::new()
        }]
    );
}

fn question(title: &str, options: Option<Vec<String>>) -> AsyncUserInputQuestion {
    AsyncUserInputQuestion {
        title: title.into(),
        options,
    }
}
