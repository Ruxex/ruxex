//! Code-mode tracing for handlers and queued nested dispatch.

use genex_protocol::ThreadId;
use std::time::Duration;
use tracing::Span;

pub(super) fn record_code_mode_host_duration(
    thread_id: &str,
    turn_id: &str,
    call_id: &str,
    cell_id: Option<&str>,
    tool_name: &str,
    duration: Duration,
) {
    let Ok(code_mode_host_duration_ns) = u64::try_from(duration.as_nanos()) else {
        return;
    };
    tracing::info!(
        target: "genex_code_mode::timing",
        {
            event.name = "codex.code_mode.host_timing",
            conversation_id = thread_id,
            turn_id,
            call_id,
            cell_id,
            tool_name,
            code_mode_host_duration_ns,
        },
        "code-mode host operation completed"
    );
}

pub(super) struct NestedToolDispatchTrace {
    thread_id: ThreadId,
    pub(super) call_id: String,
    pub(super) interruption: Option<DispatchInterruption>,
    pub(super) span: Span,
}

impl NestedToolDispatchTrace {
    pub(super) fn new(thread_id: ThreadId, call_id: String) -> Self {
        Self {
            thread_id,
            call_id,
            interruption: Some(DispatchInterruption::Abandoned),
            span: Span::current(),
        }
    }
}

impl Drop for NestedToolDispatchTrace {
    fn drop(&mut self) {
        let Some(outcome) = self.interruption.take() else {
            return;
        };
        let outcome = match outcome {
            DispatchInterruption::Cancelled => "cancelled",
            DispatchInterruption::CellClosed => "cell_closed",
            DispatchInterruption::Abandoned => "abandoned",
        };
        tracing::event!(
            name: "codex.code_mode.nested_tool_dispatch_interrupted",
            target: "codex_otel.trace_safe",
            parent: &self.span,
            tracing::Level::INFO,
            event.name = "codex.code_mode.nested_tool_dispatch_interrupted",
            conversation.id = %self.thread_id,
            call_id = self.call_id.as_str(),
            outcome,
        );
    }
}

pub(super) enum DispatchInterruption {
    Cancelled,
    CellClosed,
    Abandoned,
}

pub(super) fn trace_id(id: &str) -> Option<&str> {
    (!id.is_empty() && id.len() <= 256).then_some(id)
}
