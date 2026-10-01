//! Thread event buffering and replay state for the TUI app.
//!
//! This module owns the per-thread event store used when the TUI switches between the main
//! conversation, subagents, and side conversations. It keeps buffered app-server notifications,
//! pending interactive request replay state, active-turn tracking, and saved composer state close
//! together with the replay behavior that consumes them.

use super::*;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub(super) struct ThreadEventSnapshot {
    pub(super) session: Option<ThreadSessionState>,
    pub(super) turns: Vec<Turn>,
    pub(super) events: Vec<ThreadBufferedEvent>,
    pub(super) active_reasoning_item: Option<genex_app_server_protocol::ItemStartedNotification>,
    pub(super) input_state: Option<ThreadInputState>,
}

#[derive(Debug, Clone)]
pub(super) enum ThreadBufferedEvent {
    Notification(Box<ServerNotification>),
    Request(Box<ServerRequest>),
    HistoryEntryResponse(HistoryLookupResponse),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ThreadEventAttachment {
    Live,
    ReplayOnly,
    ExternalWriter,
}

#[derive(Debug)]
pub(super) struct ThreadEventStore {
    pub(super) session: Option<ThreadSessionState>,
    pub(super) turns: Vec<Turn>,
    pub(super) buffer: VecDeque<ThreadBufferedEvent>,
    pub(super) pending_interactive_replay: PendingInteractiveReplayState,
    pub(super) active_turn_id: Option<String>,
    // Retain the active item even if its start falls out of the bounded replay buffer.
    pub(super) active_reasoning_item: Option<genex_app_server_protocol::ItemStartedNotification>,
    // Lifecycle identity must survive bounded replay-buffer eviction.
    pub(super) latest_turn_id: Option<String>,
    pub(super) pending_interrupt_turn_id: Option<String>,
    pub(super) input_state: Option<ThreadInputState>,
    pub(super) capacity: usize,
    pub(super) active: bool,
    pub(super) buffered_agent_message_delta_bytes: usize,
    recap_progress: recap::RecapProgress,
}

impl ThreadEventStore {
    pub(super) fn event_survives_session_refresh(event: &mut ThreadBufferedEvent) -> bool {
        if let ThreadBufferedEvent::Notification(notification) = event
            && let ServerNotification::ItemCompleted(notification) = notification.as_mut()
            && let ThreadItem::AgentMessage {
                questions: Some(_),
                text,
                ..
            } = &mut notification.item
        {
            // Refreshed turns contain the text; retain only the live question state.
            text.clear();
            return true;
        }
        match event {
            ThreadBufferedEvent::Request(_) => true,
            ThreadBufferedEvent::Notification(notification) => matches!(
                notification.as_ref(),
                ServerNotification::HookStarted(_)
                    | ServerNotification::HookCompleted(_)
                    | ServerNotification::McpServerStatusUpdated(_)
            ),
            ThreadBufferedEvent::HistoryEntryResponse(_) => false,
        }
    }

    pub(super) fn new(capacity: usize) -> Self {
        Self {
            session: None,
            turns: Vec::new(),
            buffer: VecDeque::new(),
            pending_interactive_replay: PendingInteractiveReplayState::default(),
            active_turn_id: None,
            active_reasoning_item: None,
            latest_turn_id: None,
            pending_interrupt_turn_id: None,
            input_state: None,
            capacity,
            active: false,
            buffered_agent_message_delta_bytes: 0,
            recap_progress: recap::RecapProgress::default(),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn new_with_session(
        capacity: usize,
        session: ThreadSessionState,
        turns: Vec<Turn>,
    ) -> Self {
        let mut store = Self::new(capacity);
        store.session = Some(session);
        store.set_turns(turns);
        store
    }

    pub(super) fn set_session(&mut self, session: ThreadSessionState, turns: Vec<Turn>) {
        self.session = Some(session);
        self.set_turns(turns);
    }

    pub(super) fn rebase_buffer_after_session_refresh(&mut self) {
        self.buffer.retain_mut(Self::event_survives_session_refresh);
        self.buffered_agent_message_delta_bytes = 0;
    }

    pub(super) fn set_turns(&mut self, turns: Vec<Turn>) {
        if self.active_reasoning_item.as_ref().is_some_and(|started| {
            turns.iter().any(|turn| {
                (turn.id == started.turn_id && turn.status != TurnStatus::InProgress)
                    || (turn.id != started.turn_id && turn.status == TurnStatus::InProgress)
            })
        }) {
            self.active_reasoning_item = None;
        }
        self.recap_progress
            .merge(recap::RecapProgress::from_turns(&turns));
        self.active_turn_id = turns
            .iter()
            .rev()
            .find(|turn| matches!(turn.status, TurnStatus::InProgress))
            .map(|turn| turn.id.clone());
        self.latest_turn_id = turns.last().map(|turn| turn.id.clone());
        self.turns = turns;
    }

    pub(super) fn set_active_turn_id(&mut self, turn_id: String) {
        self.latest_turn_id = Some(turn_id.clone());
        self.active_turn_id = Some(turn_id);
    }

    pub(super) fn push_notification(&mut self, notification: ServerNotification) {
        self.push_notification_inner(Cow::Owned(notification));
    }

    pub(super) fn push_notification_ref(&mut self, notification: &ServerNotification) {
        self.push_notification_inner(Cow::Borrowed(notification));
    }

    fn push_notification_inner(&mut self, notification: Cow<'_, ServerNotification>) {
        let user_item = match notification.as_ref() {
            ServerNotification::ItemStarted(n) => Some((&n.turn_id, &n.item)),
            ServerNotification::ItemCompleted(n) => Some((&n.turn_id, &n.item)),
            _ => None,
        };
        let _ = user_item;
        self.pending_interactive_replay
            .note_server_notification(notification.as_ref());
        match notification.as_ref() {
            ServerNotification::TurnStarted(turn) => {
                self.set_active_turn_id(turn.turn.id.clone());
                if self
                    .active_reasoning_item
                    .as_ref()
                    .is_some_and(|started| started.turn_id != turn.turn.id)
                {
                    self.active_reasoning_item = None;
                }
            }
            ServerNotification::TurnCompleted(turn) => {
                if self
                    .active_reasoning_item
                    .as_ref()
                    .is_some_and(|started| started.turn_id == turn.turn.id)
                {
                    self.active_reasoning_item = None;
                }
                if self.active_turn_id.is_none() {
                    self.latest_turn_id = Some(turn.turn.id.clone());
                }
                if matches!(turn.turn.status, TurnStatus::Completed) {
                    self.recap_progress.completed_turns += 1;
                }
                if self.active_turn_id.as_deref() == Some(turn.turn.id.as_str()) {
                    self.active_turn_id = None;
                }
                if self.pending_interrupt_turn_id.as_deref() == Some(turn.turn.id.as_str()) {
                    self.pending_interrupt_turn_id = None;
                }
            }
            ServerNotification::Error(n)
                if self.active_turn_id.is_none()
                    && !n.will_retry
                    && n.error.codex_error_info
                        == Some(AppServerCodexErrorInfo::MisalignmentPolicyViolation) =>
            {
                self.latest_turn_id = Some(n.turn_id.clone());
            }
            ServerNotification::ThreadClosed(_) => {
                self.active_turn_id = None;
                self.active_reasoning_item = None;
                self.pending_interrupt_turn_id = None;
            }
            ServerNotification::ItemStarted(
                started @ genex_app_server_protocol::ItemStartedNotification {
                    item: ThreadItem::Reasoning { id, .. },
                    ..
                },
            ) => {
                self.active_reasoning_item =
                    Some(genex_app_server_protocol::ItemStartedNotification {
                        thread_id: started.thread_id.clone(),
                        turn_id: started.turn_id.clone(),
                        item: ThreadItem::Reasoning {
                            id: id.clone(),
                            summary: Vec::new(),
                            content: Vec::new(),
                        },
                        started_at_ms: started.started_at_ms,
                    });
            }
            ServerNotification::ItemCompleted(completed)
                if matches!(&completed.item, ThreadItem::Reasoning { .. })
                    && self.active_reasoning_item.as_ref().is_some_and(|started| {
                        started.turn_id == completed.turn_id
                            && started.item.id() == completed.item.id()
                    }) =>
            {
                self.active_reasoning_item = None;
            }
            _ => {}
        }

        // These notifications are either handled before routing or ignored by ChatWidget on
        // replay. In particular, raw response items and realtime audio can carry large payloads,
        // so cloning them into every thread's replay buffer only retains data the TUI cannot use.
        if matches!(
            notification.as_ref(),
            ServerNotification::RawResponseItemCompleted(_)
                | ServerNotification::FileChangePatchUpdated(_)
                | ServerNotification::ServerRequestResolved(_)
                | ServerNotification::McpToolCallProgress(_)
                | ServerNotification::ThreadAttachmentUpdated(_)
                | ServerNotification::CommandExecOutputDelta(_)
                | ServerNotification::ProcessOutputDelta(_)
                | ServerNotification::ProcessExited(_)
        ) {
            return;
        }

        self.push_replay_notification(notification);
    }

    pub(super) fn push_request(&mut self, request: ServerRequest) {
        self.pending_interactive_replay
            .note_server_request(&request);
        self.push_buffered_event(ThreadBufferedEvent::Request(Box::new(request)));
    }

    pub(super) fn pending_replay_requests(&self) -> Vec<ServerRequest> {
        self.buffer
            .iter()
            .filter_map(|event| match event {
                ThreadBufferedEvent::Request(request)
                    if self
                        .pending_interactive_replay
                        .should_replay_snapshot_request(request.as_ref()) =>
                {
                    Some(request.as_ref().clone())
                }
                ThreadBufferedEvent::Request(_)
                | ThreadBufferedEvent::Notification(_)
                | ThreadBufferedEvent::HistoryEntryResponse(_) => None,
            })
            .collect()
    }

    pub(super) fn file_change_changes(
        &self,
        turn_id: &str,
        item_id: &str,
    ) -> Option<Vec<genex_app_server_protocol::FileUpdateChange>> {
        file_change_changes(self.buffer.iter(), &self.turns, turn_id, item_id)
    }

    pub(super) fn snapshot(&self) -> ThreadEventSnapshot {
        let mut snapshot = ThreadEventSnapshot {
            session: self.session.clone(),
            turns: self.turns.clone(),
            // Thread switches replay buffered events into a rebuilt ChatWidget. Only replay
            // interactive prompts that are still pending, or answered approvals/input will reappear.
            events: self
                .buffer
                .iter()
                .filter(|event| match event {
                    ThreadBufferedEvent::Request(request) => self
                        .pending_interactive_replay
                        .should_replay_snapshot_request(request.as_ref()),
                    ThreadBufferedEvent::Notification(_)
                    | ThreadBufferedEvent::HistoryEntryResponse(_) => true,
                })
                .cloned()
                .collect(),
            active_reasoning_item: self.active_reasoning_item.clone(),
            input_state: self.input_state.clone(),
        };
        if let Some(latest_turn_id) = &self.latest_turn_id {
            replay_filter::omit_resolved_misalignment_errors(&mut snapshot, latest_turn_id);
        }
        snapshot
    }

    pub(super) fn recap_progress(&self) -> recap::RecapProgress {
        self.recap_progress
    }

    pub(super) fn merge_recap_progress(&mut self, progress: recap::RecapProgress) {
        self.recap_progress.merge(progress);
    }

    pub(super) fn note_outbound_op<T>(&mut self, op: T)
    where
        T: Into<AppCommand>,
    {
        self.pending_interactive_replay.note_outbound_op(op);
    }

    pub(super) fn op_can_change_pending_replay_state<T>(op: T) -> bool
    where
        T: Into<AppCommand>,
    {
        PendingInteractiveReplayState::op_can_change_state(op)
    }

    pub(super) fn has_pending_thread_approvals(&self) -> bool {
        self.pending_interactive_replay
            .has_pending_thread_approvals()
    }

    pub(super) fn side_parent_pending_status(&self) -> Option<SideParentStatus> {
        if self
            .pending_interactive_replay
            .has_pending_thread_user_input()
        {
            Some(SideParentStatus::NeedsInput)
        } else if self
            .pending_interactive_replay
            .has_pending_thread_approvals()
        {
            Some(SideParentStatus::NeedsApproval)
        } else {
            None
        }
    }

    pub(super) fn active_turn_id(&self) -> Option<&str> {
        self.active_turn_id.as_deref()
    }

    pub(super) fn clear_active_turn_id(&mut self) {
        self.active_turn_id = None;
    }
}

fn turn_id_matches(request_turn_id: &str, candidate_turn_id: &str) -> bool {
    request_turn_id.is_empty() || request_turn_id == candidate_turn_id
}

pub(super) fn file_change_changes<'a>(
    events: impl DoubleEndedIterator<Item = &'a ThreadBufferedEvent>,
    turns: &'a [Turn],
    turn_id: &str,
    item_id: &str,
) -> Option<Vec<genex_app_server_protocol::FileUpdateChange>> {
    let event_items = events.rev().filter_map(|event| {
        let ThreadBufferedEvent::Notification(notification) = event else {
            return None;
        };
        let (candidate_turn_id, item) = match notification.as_ref() {
            ServerNotification::ItemStarted(notification) => {
                (&notification.turn_id, &notification.item)
            }
            ServerNotification::ItemCompleted(notification) => {
                (&notification.turn_id, &notification.item)
            }
            _ => return None,
        };
        turn_id_matches(turn_id, candidate_turn_id).then_some(item)
    });
    event_items
        .chain(
            turns
                .iter()
                .rev()
                .filter(|turn| turn_id_matches(turn_id, &turn.id))
                .flat_map(|turn| turn.items.iter().rev()),
        )
        .find_map(|item| file_change_item_changes(item, item_id))
}

fn file_change_item_changes(
    item: &ThreadItem,
    item_id: &str,
) -> Option<Vec<genex_app_server_protocol::FileUpdateChange>> {
    match item {
        ThreadItem::FileChange { id, changes, .. } if id == item_id => Some(changes.clone()),
        _ => None,
    }
}

#[derive(Debug)]
pub(super) struct ThreadEventChannel {
    pub(super) sender: mpsc::Sender<ThreadBufferedEvent>,
    pub(super) receiver: Option<mpsc::Receiver<ThreadBufferedEvent>>,
    pub(super) store: Arc<Mutex<ThreadEventStore>>,
    attachment: ThreadEventAttachment,
}

impl ThreadEventChannel {
    pub(super) fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Some(receiver),
            store: Arc::new(Mutex::new(ThreadEventStore::new(capacity))),
            attachment: ThreadEventAttachment::Live,
        }
    }

    pub(super) fn mark_replay_only(&mut self) {
        self.attachment = ThreadEventAttachment::ReplayOnly;
    }

    pub(super) fn mark_external_writer(&mut self) {
        self.attachment = ThreadEventAttachment::ExternalWriter;
    }

    pub(super) fn attachment(&self) -> ThreadEventAttachment {
        self.attachment
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn new_with_session(
        capacity: usize,
        session: ThreadSessionState,
        turns: Vec<Turn>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Some(receiver),
            store: Arc::new(Mutex::new(ThreadEventStore::new_with_session(
                capacity, session, turns,
            ))),
            attachment: ThreadEventAttachment::Live,
        }
    }
}
