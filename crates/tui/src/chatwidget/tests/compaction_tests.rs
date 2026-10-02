use super::*;
use pretty_assertions::assert_eq;

fn normalize_compaction_snapshot(text: String) -> String {
    let elapsed = regex_lite::Regex::new(r"\b\d+(?:h \d+m \d+s|m \d+s|s)\b").unwrap();
    elapsed
        .replace_all(&normalize_snapshot_paths(text), "<elapsed>")
        .into_owned()
}

fn compaction_started(id: &str) -> ServerNotification {
    ServerNotification::ItemStarted(ItemStartedNotification {
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
        started_at_ms: chrono::Utc::now().timestamp_millis(),
        item: AppServerThreadItem::ContextCompaction { id: id.to_string() },
    })
}

fn compaction_completed(id: &str) -> ServerNotification {
    ServerNotification::ItemCompleted(ItemCompletedNotification {
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
        completed_at_ms: 0,
        item: AppServerThreadItem::ContextCompaction { id: id.to_string() },
    })
}



#[tokio::test]
async fn compaction_status_clears_when_turn_ends_without_item_completion() {
    for status in [
        AppServerTurnStatus::Completed,
        AppServerTurnStatus::Interrupted,
        AppServerTurnStatus::Failed,
    ] {
        let (mut chat, mut rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
        handle_turn_started(&mut chat, "turn-1");
        chat.handle_server_notification(compaction_started("compact-1"), /*replay_kind*/ None);
        chat.handle_server_notification(
            ServerNotification::TurnCompleted(TurnCompletedNotification {
                thread_id: "thread-1".to_string(),
                turn: app_server_turn(
                    "turn-1", status, /*duration_ms*/ None, /*error*/ None,
                ),
            }),
            /*replay_kind*/ None,
        );
        assert!(chat.status_state.compaction.is_none());
        assert!(!chat.bottom_pane.status_indicator_visible());
        let history = drain_insert_history(&mut rx);
        let lines: Vec<_> = history.into_iter().flatten().collect();
        assert!(!lines_to_single_string(&lines).contains("Context compacted"));
        handle_turn_started(&mut chat, "turn-2");
        assert_eq!(
            chat.bottom_pane.status_widget().unwrap().header(),
            "Working"
        );
    }
}

#[tokio::test]
async fn compaction_history_does_not_start_a_timer_or_finish_live_compaction() {
    let (mut chat, mut rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.handle_server_notification(
        compaction_started("old"),
        Some(ReplayKind::ResumeInitialMessages),
    );
    assert!(chat.status_state.compaction.is_none());
    assert!(!chat.bottom_pane.status_indicator_visible());
    handle_turn_started(&mut chat, "turn-1");
    chat.handle_server_notification(compaction_started("compact-1"), /*replay_kind*/ None);
    chat.handle_server_notification(
        compaction_completed("old"),
        Some(ReplayKind::ThreadSnapshot),
    );
    assert_eq!(
        chat.status_state.compaction.as_ref().unwrap().id,
        "compact-1"
    );
    let history = drain_insert_history(&mut rx);
    let lines: Vec<_> = history.into_iter().flatten().collect();
    assert_eq!(lines_to_single_string(&lines).trim(), "• Context compacted");
}

#[tokio::test]
async fn compaction_snapshot_restores_elapsed_time_and_clears_on_replayed_completion() {
    let (mut chat, mut rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    handle_turn_started(&mut chat, "turn-1");
    let ServerNotification::ItemStarted(mut started) = compaction_started("compact-1") else {
        unreachable!();
    };
    started.started_at_ms -= 12_000;
    chat.handle_server_notification(
        ServerNotification::ItemStarted(started),
        Some(ReplayKind::ThreadSnapshot),
    );
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Compacting context"
    );
    assert!(
        chat.status_state
            .compaction
            .as_ref()
            .unwrap()
            .started_at
            .elapsed()
            >= Duration::from_secs(/*secs*/ 12)
    );

    chat.handle_server_notification(
        compaction_completed("compact-1"),
        Some(ReplayKind::ThreadSnapshot),
    );
    assert!(chat.status_state.compaction.is_none());
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Working"
    );
    let lines: Vec<_> = drain_insert_history(&mut rx)
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(lines_to_single_string(&lines).trim(), "• Context compacted");
}

#[tokio::test]
async fn compaction_retry_status_returns_to_compacting() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    handle_turn_started(&mut chat, "turn-1");
    chat.handle_server_notification(compaction_started("compact-1"), /*replay_kind*/ None);
    chat.handle_server_notification(
        ServerNotification::Error(ErrorNotification {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            will_retry: true,
            error: genex_app_server_protocol::TurnError {
                message: "Reconnecting".to_string(),
                genex_error_info: None,
                additional_details: None,
                misalignment: None,
            },
        }),
        /*replay_kind*/ None,
    );
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Reconnecting"
    );
    chat.handle_server_notification(compaction_started("compact-1"), /*replay_kind*/ None);
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Compacting context"
    );
}
