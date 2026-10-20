//! Status indicator and terminal-title state for `ChatWidget`.

use crate::status_indicator_widget::STATUS_DETAILS_DEFAULT_MAX_LINES;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StatusIndicatorState {
    pub(super) header: String,
    pub(super) details: Option<String>,
    pub(super) details_max_lines: usize,
}
impl StatusIndicatorState {
    pub(super) fn working() -> Self {
        Self {
            header: String::from("Working"),
            details: None,
            details_max_lines: STATUS_DETAILS_DEFAULT_MAX_LINES,
        }
    }
}

/// Compact runtime states that can be rendered into the terminal title.
///
/// This is intentionally smaller than the full status-header vocabulary. The
/// title needs short, stable labels, so callers map richer lifecycle events
/// onto one of these buckets before rendering.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum TerminalTitleStatusKind {
    Working,
    WaitingForBackgroundTerminal,
    #[default]
    Thinking,
}

#[derive(Debug)]
pub(super) struct StatusState {
    /// Only the active reasoning item may update the streamed heading.
    pub(super) reasoning_item_id: Option<String>,
    /// An in-progress snapshot can omit the start of its next live reasoning update.
    pub(super) reasoning_resume_turn_id: Option<String>,
    /// A restored stream may have missed earlier deltas; use its complete item at finalization.
    pub(super) reasoning_recovered_after_refresh: bool,
    pub(super) compaction: Option<super::compaction::ActiveCompaction>,
    pub(super) current_status: StatusIndicatorState,
    pub(super) terminal_title_status_kind: TerminalTitleStatusKind,
    pub(super) retry_status_header: Option<String>,
    pub(super) pending_status_indicator_restore: bool,
    pub(super) thread_title_generation_pending: bool,
}

impl Default for StatusState {
    fn default() -> Self {
        Self {
            reasoning_item_id: None,
            reasoning_resume_turn_id: None,
            reasoning_recovered_after_refresh: false,
            compaction: None,
            current_status: StatusIndicatorState::working(),
            terminal_title_status_kind: TerminalTitleStatusKind::Working,
            retry_status_header: None,
            pending_status_indicator_restore: false,
            thread_title_generation_pending: false,
        }
    }
}

impl StatusState {
    pub(super) fn set_status(&mut self, status: StatusIndicatorState) {
        self.current_status = status;
    }

    pub(super) fn take_retry_status_header(&mut self) -> Option<String> {
        self.retry_status_header.take()
    }

    pub(super) fn remember_retry_status_header(&mut self) {
        if self.retry_status_header.is_none() {
            self.retry_status_header = Some(self.current_status.header.clone());
        }
    }
}
