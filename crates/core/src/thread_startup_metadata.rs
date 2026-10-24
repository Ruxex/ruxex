//! Retained thread startup metadata, separate from replay history.

use ruxex_protocol::SessionId;
use ruxex_protocol::ThreadId;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::SessionConfiguredEvent;

/// Identity and configuration captured when a thread started.
///
/// Initial messages are deliberately excluded from this retained metadata. They are replay data
/// and are supplied afresh when a resumed thread emits its startup event.
#[derive(Clone, Debug)]
pub struct ThreadStartupMetadata {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    session_configured: SessionConfiguredEvent,
}

impl From<&SessionConfiguredEvent> for ThreadStartupMetadata {
    fn from(event: &SessionConfiguredEvent) -> Self {
        let mut session_configured = event.clone();
        session_configured.initial_messages = None;
        Self {
            session_id: event.session_id,
            thread_id: event.thread_id,
            session_configured,
        }
    }
}

impl ThreadStartupMetadata {
    pub(crate) fn to_session_configured_event(
        &self,
        initial_messages: Option<Vec<EventMsg>>,
    ) -> SessionConfiguredEvent {
        let mut event = self.session_configured.clone();
        event.initial_messages = initial_messages;
        event
    }
}
