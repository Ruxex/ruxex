//! Replays persisted token usage snapshots when a client attaches to an existing thread.
//!
//! The message processor decides when replay is allowed and preserves JSON-RPC response
//! ordering. This module owns notification construction and the attribution rules that
//! map the latest persisted `TokenCount` back to a v2 turn id.
//!
//! Rollout histories can contain explicit turn ids or generated turn ids. When explicit
//! ids do not match the rebuilt thread, replay falls back to the active turn position at
//! the time the `TokenCount` was persisted so the notification still targets the
//! corresponding rebuilt turn.

use std::sync::Arc;

use ruxex_app_server_protocol::ServerNotification;
use ruxex_app_server_protocol::ThreadHistoryBuilder;
use ruxex_app_server_protocol::ThreadTokenUsage;
use ruxex_app_server_protocol::ThreadTokenUsageUpdatedNotification;
use ruxex_app_server_protocol::Turn;
use ruxex_app_server_protocol::TurnStatus;
use ruxex_core::RuxexThread;
use ruxex_protocol::ThreadId;
use ruxex_protocol::protocol::EventMsg;
use ruxex_rollout::RolloutItem;

use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::OutgoingMessageSender;

/// Sends a restored token usage update to the connection that attached to a thread.
///
/// This is lifecycle replay rather than a model event: the rollout already contains
/// the original `TokenCount`, and emitting through `send_event` here would duplicate
/// persisted usage records. Keeping replay connection-scoped also avoids
/// surprising other subscribers with a historical usage update while they may be
/// rendering live turn events.
pub(super) async fn send_thread_token_usage_update_to_connection(
    outgoing: &Arc<OutgoingMessageSender>,
    connection_id: ConnectionId,
    thread_id: ThreadId,
    conversation: &RuxexThread,
    token_usage_turn_id: String,
) {
    let Some(info) = conversation.token_usage_info().await else {
        return;
    };
    let notification = ThreadTokenUsageUpdatedNotification {
        thread_id: thread_id.to_string(),
        turn_id: token_usage_turn_id,
        token_usage: ThreadTokenUsage::from(info),
    };
    outgoing
        .send_server_notification_to_connections(
            &[connection_id],
            ServerNotification::ThreadTokenUsageUpdated(notification),
        )
        .await;
}

pub(super) fn restored_token_usage_turn_id(
    rollout_items: &[RolloutItem],
    turns: &[Turn],
) -> String {
    latest_token_usage_turn_id_from_rollout_items(rollout_items, turns)
        .unwrap_or_else(|| latest_token_usage_turn_id(turns))
}

/// Restores the latest thread-scoped usage snapshot stored in rollout history.
pub(super) fn restored_thread_token_usage(
    rollout_items: &[RolloutItem],
) -> Option<ThreadTokenUsage> {
    rollout_items.iter().rev().find_map(|item| match item {
        RolloutItem::EventMsg(EventMsg::TokenCount(event)) => {
            event.info.clone().map(ThreadTokenUsage::from)
        }
        RolloutItem::TokenUsageRecord(record) => Some(thread_token_usage_from_record(record)),
        RolloutItem::Compacted(compacted) => compacted
            .latest_token_usage_record
            .as_ref()
            .map(thread_token_usage_from_record),
        _ => None,
    })
}

fn thread_token_usage_from_record(
    record: &ruxex_protocol::protocol::TokenUsageRecord,
) -> ThreadTokenUsage {
    ThreadTokenUsage {
        total: record.thread_token_usage.clone().into(),
        last: record.usage.clone().into(),
        total_amount: record.thread_amount,
        last_amount: record.amount,
        model_context_window: None,
    }
}

/// Identifies the turn that was active when the latest `TokenCount` record appeared.
///
/// The id is preferred when it still appears in the rebuilt thread. The position is a
/// fallback for histories whose implicit turn ids are regenerated during reconstruction.
fn latest_token_usage_turn_id_from_rollout_items(
    rollout_items: &[RolloutItem],
    turns: &[Turn],
) -> Option<String> {
    let token_count_index = rollout_items
        .iter()
        .rposition(|item| matches!(item, RolloutItem::EventMsg(EventMsg::TokenCount(_))))?;
    let mut builder = ThreadHistoryBuilder::new();
    for item in &rollout_items[..token_count_index] {
        builder.handle_rollout_item(item);
    }

    if turns.is_empty() {
        return builder.active_turn_id_if_explicit();
    }

    let active_turn_id = builder.active_turn_id()?;
    if turns.iter().any(|turn| turn.id == active_turn_id) {
        Some(active_turn_id.to_string())
    } else {
        builder
            .active_turn_position()
            .and_then(|position| turns.get(position))
            .map(|turn| turn.id.clone())
    }
}

/// Chooses a fallback turn id that should own a replayed token usage update.
///
/// Normal replay derives the owner from the rollout position of the latest
/// `TokenCount` event. This fallback only preserves a stable wire shape for
/// unusual histories where that rollout information cannot be read.
fn latest_token_usage_turn_id(turns: &[Turn]) -> String {
    turns
        .iter()
        .rev()
        .find(|turn| matches!(turn.status, TurnStatus::Completed | TurnStatus::Failed))
        .or_else(|| turns.last())
        .map(|turn| turn.id.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use ruxex_app_server_protocol::build_turns_from_rollout_items;
    use ruxex_protocol::SessionId;
    use ruxex_protocol::protocol::AgentMessageEvent;
    use ruxex_protocol::protocol::TokenCountEvent;
    use ruxex_protocol::protocol::TokenUsage;
    use ruxex_protocol::protocol::TokenUsageInfo;
    use ruxex_protocol::protocol::TokenUsageRecord;
    use ruxex_protocol::protocol::UserMessageEvent;
    use ruxex_rollout::CompactedItem;

    #[test]
    fn restored_thread_token_usage_restores_legacy_token_count() {
        let mut rollout_items = token_usage_history();
        rollout_items[2] = RolloutItem::EventMsg(EventMsg::TokenCount(TokenCountEvent {
            info: Some(TokenUsageInfo {
                total_token_usage: token_usage(150),
                last_token_usage: token_usage(90),
                total_amount: Some(0.56),
                last_amount: Some(0.03),
                model_context_window: Some(200_000),
            }),
            rate_limits: None,
        }));

        let usage = restored_thread_token_usage(&rollout_items).unwrap();

        assert_eq!(usage.total.total_tokens, 150);
        assert_eq!(usage.last.total_tokens, 90);
        assert_eq!(usage.total_amount, Some(0.56));
        assert_eq!(usage.last_amount, Some(0.03));
        assert_eq!(usage.model_context_window, Some(200_000));
    }

    #[test]
    fn restored_thread_token_usage_returns_none_without_snapshot() {
        assert_eq!(restored_thread_token_usage(&token_usage_history()), None);
    }

    #[test]
    fn restored_thread_token_usage_restores_usage_record() {
        let rollout_items = vec![RolloutItem::TokenUsageRecord(token_usage_record())];

        let usage = restored_thread_token_usage(&rollout_items).unwrap();

        assert_eq!(usage.total.total_tokens, 150);
        assert_eq!(usage.last.total_tokens, 90);
        assert_eq!(usage.total_amount, Some(0.56));
        assert_eq!(usage.last_amount, Some(0.03));
        assert_eq!(usage.model_context_window, None);
    }

    #[test]
    fn restored_thread_token_usage_uses_latest_record_and_skips_empty_token_count() {
        let mut newer_record = token_usage_record();
        newer_record.thread_token_usage = token_usage(250);
        newer_record.usage = token_usage(100);
        newer_record.thread_amount = Some(0.88);
        newer_record.amount = Some(0.05);
        let rollout_items = vec![
            RolloutItem::TokenUsageRecord(token_usage_record()),
            RolloutItem::TokenUsageRecord(newer_record),
            RolloutItem::EventMsg(EventMsg::TokenCount(TokenCountEvent {
                info: None,
                rate_limits: None,
            })),
        ];

        let usage = restored_thread_token_usage(&rollout_items).unwrap();

        assert_eq!(usage.total.total_tokens, 250);
        assert_eq!(usage.last.total_tokens, 100);
        assert_eq!(usage.total_amount, Some(0.88));
        assert_eq!(usage.last_amount, Some(0.05));
    }

    #[test]
    fn restored_thread_token_usage_restores_compacted_snapshot() {
        let rollout_items = vec![RolloutItem::Compacted(compacted_with_record(
            token_usage_record(),
        ))];

        let usage = restored_thread_token_usage(&rollout_items).unwrap();

        assert_eq!(usage.total.total_tokens, 150);
        assert_eq!(usage.last.total_tokens, 90);
        assert_eq!(usage.total_amount, Some(0.56));
        assert_eq!(usage.last_amount, Some(0.03));
    }

    #[test]
    fn restored_thread_token_usage_obeys_compacted_snapshot_order() {
        let mut older_record = token_usage_record();
        older_record.thread_token_usage = token_usage(250);
        let rollout_items = vec![
            RolloutItem::TokenUsageRecord(older_record),
            RolloutItem::Compacted(compacted_with_record(token_usage_record())),
        ];

        let usage = restored_thread_token_usage(&rollout_items).unwrap();

        assert_eq!(usage.total.total_tokens, 150);
        assert_eq!(usage.total_amount, Some(0.56));
    }

    #[test]
    fn replay_attribution_uses_already_loaded_history() {
        let rollout_items = token_usage_history();
        let turns = build_turns_from_rollout_items(&rollout_items);

        assert_eq!(
            latest_token_usage_turn_id_from_rollout_items(&rollout_items, turns.as_slice()),
            Some(turns[0].id.clone())
        );
    }

    #[test]
    fn replay_attribution_falls_back_to_rebuilt_turn_position() {
        let rollout_items = token_usage_history();
        let mut turns = build_turns_from_rollout_items(&rollout_items);
        turns[0].id = "rebuilt-turn-id".to_string();

        assert_eq!(
            latest_token_usage_turn_id_from_rollout_items(&rollout_items, turns.as_slice()),
            Some("rebuilt-turn-id".to_string())
        );
    }

    #[test]
    fn replay_attribution_rejects_suffix_generated_turn_ids() {
        let rollout_items = token_usage_history();

        assert_eq!(
            latest_token_usage_turn_id_from_rollout_items(&rollout_items, /*turns*/ &[]),
            None
        );
    }

    #[test]
    fn replay_attribution_uses_latest_token_count_and_ignores_tail_turn() {
        let mut rollout_items = token_usage_history();
        rollout_items.extend(token_usage_history());
        let turns = build_turns_from_rollout_items(&rollout_items);

        assert_eq!(
            latest_token_usage_turn_id_from_rollout_items(&rollout_items, turns.as_slice()),
            Some(turns[2].id.clone())
        );
    }

    fn token_usage_history() -> Vec<RolloutItem> {
        vec![
            RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "first turn".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
                ..Default::default()
            })),
            RolloutItem::EventMsg(EventMsg::AgentMessage(AgentMessageEvent {
                message: "first answer".to_string(),
                phase: None,
                delivery: None,
                questions: None,
            })),
            RolloutItem::EventMsg(EventMsg::TokenCount(TokenCountEvent {
                info: None,
                rate_limits: None,
            })),
            RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
                message: "second turn".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
                ..Default::default()
            })),
        ]
    }

    fn token_usage(amount: i64) -> TokenUsage {
        TokenUsage {
            input_tokens: amount,
            cached_input_tokens: 20,
            cache_write_input_tokens: 2,
            output_tokens: 30,
            reasoning_output_tokens: 10,
            total_tokens: amount,
            ruxex_rollout_budget_units: None,
        }
    }

    fn token_usage_record() -> TokenUsageRecord {
        TokenUsageRecord {
            thread_id: ThreadId::default(),
            turn_id: "turn-1".to_string(),
            session_id: SessionId::default(),
            root_turn_id: "turn-1".to_string(),
            response_id: "response-1".to_string(),
            usage: token_usage(90),
            amount: Some(0.03),
            turn_token_usage: token_usage(90),
            thread_token_usage: token_usage(150),
            turn_amount: Some(0.03),
            thread_amount: Some(0.56),
        }
    }

    fn compacted_with_record(record: TokenUsageRecord) -> CompactedItem {
        CompactedItem {
            message: String::new(),
            replacement_history: None,
            retained_context: None,
            window_number: None,
            first_window_id: None,
            previous_window_id: None,
            window_id: None,
            compaction_response_id: None,
            latest_token_usage_record: Some(record),
        }
    }
}
