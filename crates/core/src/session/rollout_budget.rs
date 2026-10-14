use super::session::Session;
use super::turn_context::TurnContext;
use crate::context::ContextualUserFragment;
use ruxex_protocol::error::RuxexErr;
use ruxex_protocol::error::Result as RuxexResult;
use ruxex_protocol::protocol::TokenUsage;

pub(super) async fn maybe_record_reminder(
    sess: &Session,
    turn_context: &TurnContext,
    window_id: &str,
) {
    let budget = sess.services.agent_control.rollout_budget();
    let Some(reminder) = budget.pending_reminder(sess.thread_id(), window_id) else {
        return;
    };
    let response_item = ContextualUserFragment::into(crate::context::RolloutBudgetContext {
        remaining_tokens: reminder.remaining_tokens,
    });
    sess.record_conversation_items(
        turn_context,
        turn_context.model_info(),
        std::slice::from_ref(&response_item),
    )
    .await;
    budget.mark_reminder_delivered(sess.thread_id(), window_id, reminder);
}

impl Session {
    pub(crate) fn record_rollout_budget_usage(&self, usage: &TokenUsage) -> RuxexResult<()> {
        if self
            .services
            .agent_control
            .rollout_budget()
            .record_usage(usage)?
        {
            return Err(RuxexErr::SessionBudgetExceeded);
        }
        Ok(())
    }
}
