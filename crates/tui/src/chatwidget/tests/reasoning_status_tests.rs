//! Reasoning activity retains the latest usable line through empty items and status-row replacement.

use super::*;
use pretty_assertions::assert_eq;

fn delta(chat: &mut ChatWidget, id: &str, text: &str) {
    chat.handle_server_notification(
        ServerNotification::ReasoningSummaryTextDelta(ReasoningSummaryTextDeltaNotification {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: id.to_string(),
            delta: text.to_string(),
            summary_index: 0,
        }),
        /*replay_kind*/ None,
    );
}

fn complete(chat: &mut ChatWidget, id: &str) {
    chat.handle_server_notification(
        ServerNotification::ItemCompleted(ItemCompletedNotification {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            completed_at_ms: 0,
            item: AppServerThreadItem::Reasoning {
                id: id.to_string(),
                summary: Vec::new(),
                content: Vec::new(),
            },
        }),
        /*replay_kind*/ None,
    );
}



#[tokio::test]
async fn voice_handoff_preserves_typed_reasoning_and_ignores_private_items() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();
    handle_agent_reasoning_started(&mut chat, "typed");
    delta(&mut chat, "typed", "**Checking repository**");

    chat.remember_realtime_delegated_reasoning_turn("turn-1");
    handle_agent_reasoning_started(&mut chat, "private");
    delta(&mut chat, "private", "**Private voice reasoning**");
    complete(&mut chat, "private");
    assert_eq!(
        chat.status_state.reasoning_item_id.as_deref(),
        Some("typed")
    );
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Checking repository"
    );

    delta(&mut chat, "typed", "\n**Verifying changes**");
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Verifying changes"
    );
    complete(&mut chat, "typed");
    assert_eq!(chat.status_state.reasoning_item_id, None);
}

#[tokio::test]
async fn reasoning_status_preserves_an_explicit_wait_and_restores_the_heading() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();
    chat.status_state
        .pending_guardian_review_status
        .start_or_update("review-1".to_string(), "Checking a command".to_string());
    chat.set_status_header("Reviewing approval request".to_string());
    handle_agent_reasoning_started(&mut chat, "first");
    delta(&mut chat, "first", "**Choosing the missing-data fix**");
    complete(&mut chat, "first");
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Reviewing approval request"
    );
    chat.status_state
        .pending_guardian_review_status
        .finish("review-1");
    chat.restore_reasoning_status_header();
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Choosing the missing-data fix"
    );
}

#[tokio::test]
async fn reasoning_status_replay_retains_last_usable_heading() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();
    for (id, summary, expected) in [
        (
            "first",
            vec!["**First heading**".to_string()],
            "First heading",
        ),
        (
            "second",
            vec!["**Second heading**".to_string()],
            "Second heading",
        ),
        ("third", Vec::new(), "Second heading"),
    ] {
        chat.handle_thread_item(
            AppServerThreadItem::Reasoning {
                id: id.to_string(),
                summary,
                content: Vec::new(),
            },
            "turn-1".to_string(),
            ThreadItemRenderSource::Replay(ReplayKind::ThreadSnapshot),
        );
        assert_eq!(chat.bottom_pane.status_widget().unwrap().header(), expected);
    }
}

#[tokio::test]
async fn reasoning_status_accepts_plain_lines_and_ignores_empty_sections() {
    let (mut chat, _rx, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_task_started();
    handle_agent_reasoning_started(&mut chat, "first");
    delta(
        &mut chat,
        "first",
        "# Checking files\n<!-- progress -->\nPreparing response",
    );
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Preparing response"
    );
    complete(&mut chat, "first");
    handle_agent_reasoning_started(&mut chat, "second");
    delta(&mut chat, "second", "<!-- no public update -->");
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Preparing response"
    );
    delta(&mut chat, "second", "\n**Verifying results**");
    assert_eq!(
        chat.bottom_pane.status_widget().unwrap().header(),
        "Verifying results"
    );
}
