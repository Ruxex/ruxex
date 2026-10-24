//! Demand-driven estimated-cost state for the currently visible enterprise thread.

use super::ChatWidget;
use super::Duration;
use super::Instant;
use super::PlanType;
use super::StatusLineItem;
use super::TerminalTitleItem;
use super::ThreadId;
use crate::status::StatusHistoryHandle;
use ruxex_app_server_protocol::ThreadUsage;

const THREAD_USAGE_SETTLEMENT_DELAYS: [Duration; 3] = [
    Duration::from_secs(/*secs*/ 15),
    Duration::from_secs(/*secs*/ 60),
    Duration::from_secs(/*secs*/ 120),
];
#[derive(Debug, Default)]
struct SelectedThreadUsage {
    credits: bool,
    estimated_cost: bool,
}

#[derive(Debug, Default)]
pub(super) struct ThreadUsageState {
    thread_id: Option<ThreadId>,
    estimate: Option<ThreadUsage>,
    pending_request_id: Option<u64>,
    next_request_id: u64,
    requested: bool,
    feature_disabled: bool,
    status_history_handles: Vec<StatusHistoryHandle>,
    status_requested: bool,
    retry_attempts: usize,
    retry_due_at: Option<Instant>,
    settlement_attempts: usize,
    settlement_baseline_credits_micros: Option<i64>,
    settlement_baseline_usd_micros: Option<i64>,
    settlement_request_id: Option<u64>,
    settlement_refresh_due_at: Option<Instant>,
    pub(super) replaying_turn_completion: bool,
}

impl ChatWidget {
    pub(super) fn clear_thread_usage_state(&mut self) {
        let next_request_id = self.thread_usage.next_request_id;
        self.thread_usage = ThreadUsageState {
            next_request_id,
            ..ThreadUsageState::default()
        };
    }

    pub(super) fn ensure_thread_usage_requested(&mut self) {
        let Some(thread_id) = self.thread_id else {
            return;
        };
        if self.thread_usage.thread_id != Some(thread_id) {
            self.clear_thread_usage_state();
            self.thread_usage.thread_id = Some(thread_id);
        }
        if self.thread_usage.requested {
            return;
        }
        self.request_thread_usage();
    }

    pub(super) fn refresh_thread_usage_after_turn(&mut self) {
        if self.thread_usage.replaying_turn_completion
            || !self.thread_usage_is_selected()
            || !self.thread_usage_is_available()
        {
            return;
        }

        self.thread_usage.settlement_attempts = 0;
        self.thread_usage.settlement_request_id = None;
        self.thread_usage.settlement_baseline_credits_micros = Some(
            self.thread_usage
                .estimate
                .as_ref()
                .map(|estimate| estimate.estimated_usage_credits_micros)
                .unwrap_or_default(),
        );
        self.thread_usage.settlement_baseline_usd_micros = self
            .thread_usage
            .estimate
            .as_ref()
            .and_then(|estimate| estimate.estimated_usage_usd_micros);
        self.schedule_next_thread_usage_settlement();
        self.request_thread_usage();
    }

    pub(super) fn refresh_thread_usage_if_settlement_due(&mut self) {
        let now = Instant::now();
        let settlement_due = self
            .thread_usage
            .settlement_refresh_due_at
            .is_some_and(|due_at| now >= due_at);
        let retry_due = self
            .thread_usage
            .retry_due_at
            .is_some_and(|due_at| now >= due_at);
        if !settlement_due && !retry_due {
            if let Some(next_due_at) = [
                self.thread_usage.settlement_refresh_due_at,
                self.thread_usage.retry_due_at,
            ]
            .into_iter()
            .flatten()
            .min()
            {
                self.frame_requester
                    .schedule_frame_in(next_due_at.saturating_duration_since(now));
            }
            return;
        }

        if !self.thread_usage_is_selected() {
            self.cancel_thread_usage_polling();
            return;
        }

        if settlement_due {
            self.thread_usage.settlement_refresh_due_at = None;
        }
        if retry_due {
            self.thread_usage.retry_due_at = None;
        }
        self.request_thread_usage();
    }

    pub(super) fn request_thread_usage_for_status(&mut self, handle: StatusHistoryHandle) {
        if !self.thread_usage_is_available() {
            return;
        }

        let Some(thread_id) = self.thread_id else {
            return;
        };
        if self.thread_usage.thread_id != Some(thread_id) {
            self.clear_thread_usage_state();
            self.thread_usage.thread_id = Some(thread_id);
        }

        self.thread_usage.status_history_handles.push(handle);
        self.thread_usage.status_requested = true;
        self.thread_usage.retry_attempts = 0;
        self.request_thread_usage();
    }

    pub(super) fn cancel_thread_usage_polling(&mut self) {
        if self.thread_usage.status_requested {
            return;
        }

        self.thread_usage.pending_request_id = None;
        self.thread_usage.requested = false;
        self.thread_usage.retry_attempts = 0;
        self.thread_usage.retry_due_at = None;
        self.thread_usage.settlement_attempts = 0;
        self.thread_usage.settlement_baseline_credits_micros = None;
        self.thread_usage.settlement_baseline_usd_micros = None;
        self.thread_usage.settlement_request_id = None;
        self.thread_usage.settlement_refresh_due_at = None;
    }

    pub(super) fn estimated_thread_usage(&self) -> Option<&ThreadUsage> {
        self.thread_usage.estimate.as_ref()
    }

    fn request_thread_usage(&mut self) {
        self.thread_usage.requested = true;
    }

    pub(super) fn thread_usage_is_available(&self) -> bool {
        self.has_ruxex_backend_auth
            && !self.thread_usage.feature_disabled
            && self.thread_id.is_some()
            && matches!(
                self.plan_type,
                Some(
                    PlanType::Business
                        | PlanType::EnterpriseCbpUsageBased
                        | PlanType::EnterpriseCbpAutomation
                )
            )
    }

    fn thread_usage_is_selected(&self) -> bool {
        let selected = self.selected_thread_usage();
        selected.credits || selected.estimated_cost
    }

    fn selected_thread_usage(&self) -> SelectedThreadUsage {
        let mut selected = SelectedThreadUsage::default();
        for item in self.configured_status_line_items() {
            match item.parse::<StatusLineItem>() {
                Ok(StatusLineItem::ThreadCredits) => selected.credits = true,
                Ok(StatusLineItem::EstimatedThreadCost) => selected.estimated_cost = true,
                Ok(_) | Err(_) => {}
            }
        }
        for item in self.configured_terminal_title_items() {
            match item.parse::<TerminalTitleItem>() {
                Ok(TerminalTitleItem::ThreadCredits) => selected.credits = true,
                Ok(TerminalTitleItem::EstimatedThreadCost) => selected.estimated_cost = true,
                Ok(_) | Err(_) => {}
            }
        }
        selected
    }

    fn schedule_next_thread_usage_settlement(&mut self) {
        let Some(delay) = THREAD_USAGE_SETTLEMENT_DELAYS
            .get(self.thread_usage.settlement_attempts)
            .copied()
        else {
            self.thread_usage.settlement_baseline_credits_micros = None;
            self.thread_usage.settlement_baseline_usd_micros = None;
            self.thread_usage.settlement_request_id = None;
            return;
        };
        self.thread_usage.settlement_attempts += 1;
        self.thread_usage.settlement_refresh_due_at = Some(Instant::now() + delay);
        self.frame_requester.schedule_frame_in(delay);
    }
}
