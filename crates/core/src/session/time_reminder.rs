use chrono::DateTime;
use chrono::Utc;
use genex_features::CurrentTimeReminderDeliveryMode;
use genex_features::Feature;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result as GenexResult;
use genex_protocol::models::ResponseItem;

use super::session::Session;
use super::turn_context::TurnContext;
use crate::config::Config;
use crate::config::CurrentTimeReminderConfig;
use crate::context::ContextualUserFragment;
use crate::context::CurrentTimeUnavailable;
use crate::context_manager::is_user_turn_boundary;

pub(super) fn apply_persistent_defaults(config: &mut Config) {
    if config.current_time_reminder.is_some()
        || config
            .config_layer_stack
            .effective_config()
            .get("features")
            .and_then(|features| features.get("current_time_reminder"))
            .is_some()
    {
        return;
    }

    // Apply defaults only to this turn; explicit settings and managed policy win.
    if config.features.enable(Feature::CurrentTimeReminder).is_ok()
        && config.features.enabled(Feature::CurrentTimeReminder)
    {
        config.current_time_reminder = Some(CurrentTimeReminderConfig {
            sleep_tool: true,
            ..CurrentTimeReminderConfig::default()
        });
    }
}

#[derive(Default)]
pub(crate) struct CurrentTimeReminderState {
    last_delivery_time: Option<DateTime<Utc>>,
    last_window_id: Option<String>,
    last_clock_failure: Option<(String, uuid::Uuid)>,
    pending_user_or_tool_output_boundary: bool,
}

impl CurrentTimeReminderState {
    pub(super) fn note_recorded_items(&mut self, items: &[ResponseItem]) {
        if items.iter().any(|item| {
            is_user_turn_boundary(item)
                || matches!(
                    item,
                    ResponseItem::FunctionCallOutput { .. }
                        | ResponseItem::CustomToolCallOutput { .. }
                        | ResponseItem::ToolSearchOutput { .. }
                )
        }) {
            self.pending_user_or_tool_output_boundary = true;
        }
    }

    fn take_reminder_due(
        &mut self,
        window_id: &str,
        current_time: DateTime<Utc>,
        interval_seconds: u64,
        delivery_mode: CurrentTimeReminderDeliveryMode,
    ) -> bool {
        let is_new_window = self.last_window_id.as_deref() != Some(window_id);
        // Consume the boundary for this inference even if the interval suppresses delivery.
        let follows_user_or_tool_output =
            std::mem::take(&mut self.pending_user_or_tool_output_boundary);
        if delivery_mode == CurrentTimeReminderDeliveryMode::AfterUserOrToolOutput
            && !is_new_window
            && !follows_user_or_tool_output
        {
            return false;
        }

        let reminder_is_due = is_new_window
            || interval_seconds == 0
            || self.last_delivery_time.is_none_or(|last_delivery_time| {
                current_time
                    .signed_duration_since(last_delivery_time)
                    .num_seconds()
                    >= i64::try_from(interval_seconds).unwrap_or(i64::MAX)
            });

        if reminder_is_due {
            self.last_delivery_time = Some(current_time);
            self.last_window_id = Some(window_id.to_string());
        }

        reminder_is_due
    }
}

impl Session {
    pub(super) async fn read_clock_for_context(
        &self,
        turn_context: &TurnContext,
        clock_read: &'static str,
    ) -> GenexResult<Option<DateTime<Utc>>> {
        let error = match self
            .services
            .time_provider
            .current_time(self.thread_id)
            .await
        {
            Ok(time) => {
                let mut state = self.state.lock().await;
                state.current_time_reminder.last_clock_failure = None;
                return Ok(Some(time));
            }
            Err(error) => error,
        };
        if !turn_context
            .config
            .features
            .enabled(Feature::NonfatalClockReadErrors)
        {
            return Err(GenexErr::Fatal(format!(
                "failed to read current time: {error:#}"
            )));
        }
        tracing::error!(
            clock_read,
            thread_id = %self.thread_id,
            turn_id = %turn_context.sub_id,
            "failed to read current time; the clock provider may be stalled"
        );
        {
            let mut state = self.state.lock().await;
            let failure = (
                turn_context.sub_id.clone(),
                state.auto_compact_window_ids().window_id,
            );
            // A compacted window may no longer contain the earlier notice.
            if state.current_time_reminder.last_clock_failure.as_ref() == Some(&failure) {
                return Ok(None);
            }
            state.current_time_reminder.last_clock_failure = Some(failure);
        }
        let response_item = ContextualUserFragment::into(CurrentTimeUnavailable);
        self.record_conversation_items(
            turn_context,
            turn_context.model_info(),
            std::slice::from_ref(&response_item),
        )
        .await;
        Ok(None)
    }
}

pub(super) async fn maybe_record_current_time_reminder(
    sess: &Session,
    turn_context: &TurnContext,
    window_id: &str,
) -> GenexResult<()> {
    if !turn_context
        .config
        .features
        .enabled(Feature::CurrentTimeReminder)
    {
        return Ok(());
    }
    let Some(config) = turn_context.config.current_time_reminder else {
        return Ok(());
    };

    let Some(current_time) = sess
        .read_clock_for_context(turn_context, "reminder")
        .await?
    else {
        return Ok(());
    };

    let reminder_is_due = {
        let mut state = sess.state.lock().await;
        state.current_time_reminder.take_reminder_due(
            window_id,
            current_time,
            config.reminder_interval_seconds,
            config.delivery_mode,
        )
    };
    if !reminder_is_due {
        return Ok(());
    }

    let response_item =
        ContextualUserFragment::into(crate::context::CurrentTimeReminder::new(current_time));
    sess.record_conversation_items(
        turn_context,
        turn_context.model_info(),
        std::slice::from_ref(&response_item),
    )
    .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn time(seconds: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(seconds, 0)
            .single()
            .expect("valid timestamp")
    }

    fn user_message() -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![genex_protocol::models::ContentItem::InputText {
                text: "hello".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        }
    }

    #[test]
    fn delivers_on_new_window_and_after_interval() {
        let mut state = CurrentTimeReminderState::default();

        assert!(state.take_reminder_due(
            "window-1",
            time(0),
            60,
            CurrentTimeReminderDeliveryMode::AnyInference
        ));
        assert!(!state.take_reminder_due(
            "window-1",
            time(30),
            60,
            CurrentTimeReminderDeliveryMode::AnyInference
        ));
        assert!(state.take_reminder_due(
            "window-1",
            time(60),
            60,
            CurrentTimeReminderDeliveryMode::AnyInference
        ));
        assert!(state.take_reminder_due(
            "window-2",
            time(61),
            60,
            CurrentTimeReminderDeliveryMode::AnyInference
        ));
    }

    #[test]
    fn after_user_or_tool_output_consumes_recorded_boundary() {
        let mut state = CurrentTimeReminderState::default();
        let mode = CurrentTimeReminderDeliveryMode::AfterUserOrToolOutput;

        assert!(state.take_reminder_due("window-1", time(0), 0, mode));
        assert!(!state.take_reminder_due("window-1", time(1), 0, mode));

        state.note_recorded_items(&[user_message()]);
        assert!(state.take_reminder_due("window-1", time(2), 0, mode));
        assert!(!state.take_reminder_due("window-1", time(3), 0, mode));

        state.note_recorded_items(&[ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("call-1".to_string()),
            name: None,
            namespace: None,
            output: genex_protocol::models::FunctionCallOutputPayload::from_text("done".into()),
            internal_chat_message_metadata_passthrough: None,
        }]);
        assert!(state.take_reminder_due("window-1", time(4), 0, mode));
    }
}
