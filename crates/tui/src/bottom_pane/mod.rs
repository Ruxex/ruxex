//! The bottom pane is the interactive footer of the chat UI.
//!
//! The pane owns the [`ChatComposer`] (editable prompt input) and a stack of transient
//! [`BottomPaneView`]s (popups/modals) that temporarily replace the composer for focused
//! interactions like selection lists.
//!
//! Input routing is layered: `BottomPane` decides which local surface receives a key (view vs
//! composer), while higher-level intent such as "interrupt" or "quit" is decided by the parent
//! widget (`ChatWidget`). This split matters for Ctrl+C/Ctrl+D: the bottom pane gives the active
//! view the first chance to consume Ctrl+C (typically to dismiss itself), then lets an active
//! composer history search consume Ctrl+C as cancellation, and `ChatWidget` may treat an unhandled
//! Ctrl+C as an interrupt or as the first press of a double-press quit shortcut.
//!
//! Some UI is time-based rather than input-based, such as the transient "press again to quit"
//! hint. The pane schedules redraws so those hints can expire even when the UI is otherwise idle.
//! Number shortcuts apply only to an empty, idle composer; drafts, paste bursts, and active dialogs
//! keep their normal input routing.
use std::collections::VecDeque;
use std::path::PathBuf;

use crate::app::app_server_requests::ResolvedAppServerRequest;
use crate::app_event::AppEvent;
use crate::app_event::HistoryLookupResponse;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::pending_input_preview::PendingInputPreview;
use crate::bottom_pane::pending_thread_approvals::PendingThreadApprovals;
use crate::bottom_pane::unified_exec_footer::UnifiedExecFooter;
use crate::key_hint;
use crate::key_hint::KeyBinding;
use crate::key_hint::KeyBindingListExt;
use crate::keymap::KeymapContext;
use crate::keymap::KeymapContextSet;
use crate::keymap::RuntimeKeymap;
use crate::render::renderable::FlexRenderable;
use crate::render::renderable::Renderable;
use crate::render::renderable::RenderableItem;
use crate::terminal_palette::effective_stdout_color_level;
use crate::tui::FrameRequester;
pub(crate) use bottom_pane_view::BottomPaneView;
pub(crate) use bottom_pane_view::ViewCompletion;
use codex_app_server_protocol::SkillMetadata;
use codex_app_server_protocol::ToolRequestUserInputParams;
use codex_features::Features;
use codex_file_search::FileMatch;
use codex_plugin::PluginCapabilitySummary;
use codex_protocol::ThreadId;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::user_input::TextElement;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use std::time::Duration;
use std::time::Instant;

mod action_required_title;
mod apply_patch_header;
mod approval_overlay;
mod async_questions;
mod hook_status;
mod mcp_server_elicitation;
mod multi_select_picker;
mod questions;
mod request_user_input;
mod status_line_setup;
mod status_line_style;
mod status_surface_preview;
mod title_setup;
pub(crate) mod user_verification;
pub(crate) use action_required_title::ACTION_REQUIRED_PREVIEW_PREFIX;
pub(crate) use action_required_title::build_action_required_title_text;
pub(crate) use approval_overlay::ApplyPatchApprovalRequest;
pub(crate) use approval_overlay::ApprovalOverlay;
pub(crate) use approval_overlay::ApprovalRequest;
pub(crate) use approval_overlay::ExecApprovalRequest;
pub(crate) use approval_overlay::McpElicitationApprovalRequest;
pub(crate) use approval_overlay::PermissionsApprovalRequest;
pub(crate) use approval_overlay::format_requested_permissions_rule;
pub(crate) use async_questions::AsyncQuestions;
pub(crate) use async_questions::QuestionState;
pub(crate) use async_questions::QuestionSubmission;
pub(crate) use mcp_server_elicitation::McpServerElicitationFormRequest;
pub(crate) use mcp_server_elicitation::McpServerElicitationOverlay;
pub(crate) use request_user_input::RequestUserInputOverlay;
pub(crate) use status_line_style::status_line_from_segments;
mod bottom_pane_view;
mod effort_ignition;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalImageAttachment {
    pub(crate) placeholder: String,
    pub(crate) path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MentionBinding {
    /// Visible mention sigil (`$` or `@`).
    pub(crate) sigil: char,
    /// Mention token text without the leading sigil (`$` or `@`).
    pub(crate) mention: String,
    /// Canonical mention target (for example `app://...` or absolute SKILL.md path).
    pub(crate) path: String,
}
mod chat_composer;
mod chat_composer_history;
mod command_popup;
pub(crate) mod custom_prompt_view;
mod effort_status_line;
mod file_search_popup;
mod footer;
mod list_selection_view;
mod mentions_v2;
pub(crate) mod prompt_args;
mod skill_popup;
mod skills_toggle_view;
pub(crate) mod slash_commands;
pub(crate) use footer::CollaborationModeIndicator;
pub(crate) use footer::GoalStatusIndicator;
#[cfg(test)]
pub(crate) use footer::goal_status_indicator_line;
pub(crate) use list_selection_view::ColumnWidthMode;
pub(crate) use list_selection_view::ListSelectionView;
pub(crate) use list_selection_view::OnSelectionChangedCallback;
pub(crate) use list_selection_view::SelectionDescriptionLayout;
pub(crate) use list_selection_view::SelectionRowDisplay;
pub(crate) use list_selection_view::SelectionToggle;
pub(crate) use list_selection_view::SelectionViewParams;
pub(crate) use list_selection_view::SideContentWidth;
pub(crate) use list_selection_view::popup_content_width;
pub(crate) use list_selection_view::side_by_side_layout_widths;
use slash_commands::ServiceTierCommand;
mod hooks_browser_view;
pub(crate) use skills_toggle_view::SkillsToggleItem;
pub(crate) use skills_toggle_view::SkillsToggleView;
pub(crate) use status_line_setup::StatusLineItem;
pub(crate) use status_line_setup::StatusLineSetupView;
pub(crate) use status_surface_preview::StatusSurfacePreviewData;
pub(crate) use status_surface_preview::StatusSurfacePreviewItem;
pub(crate) use title_setup::TerminalTitleItem;
pub(crate) use title_setup::TerminalTitleSetupView;
#[cfg(test)]
pub(crate) use title_setup::preview_line_for_title_items;
mod paste_burst;
mod pending_input_preview;
mod pending_thread_approvals;
pub(crate) mod popup_consts;
mod scroll_state;
mod selection_popup_common;
mod selection_row_layout;
mod selection_tabs;
mod startup;
mod textarea;
mod unified_exec_footer;
pub(crate) use hooks_browser_view::HooksBrowserView;
pub(crate) use selection_tabs::SelectionTab;

/// How long the "press again to quit" hint stays visible.
///
/// This is shared between:
/// - `ChatWidget`: arming the double-press quit shortcut.
/// - `BottomPane`/`ChatComposer`: rendering and expiring the footer hint.
///
/// Keeping a single value ensures Ctrl+C and Ctrl+D behave identically.
pub(crate) const QUIT_SHORTCUT_TIMEOUT: Duration = Duration::from_secs(1);

const APPROVAL_PROMPT_TYPING_IDLE_DELAY: Duration = Duration::from_secs(1);

/// Whether Ctrl+C/Ctrl+D require a second press to quit.
///
/// This UX experiment was enabled by default, but requiring a double press to quit feels janky in
/// practice (especially for users accustomed to shells and other TUIs). Disable it for now while we
/// rethink a better quit/interrupt design.
pub(crate) const DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED: bool = false;

/// The result of offering a cancellation key to a bottom-pane surface.
///
/// This is primarily used for Ctrl+C routing: active views can consume the key to dismiss
/// themselves, and the caller can decide what higher-level action (if any) to take when the key is
/// not handled locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CancellationEvent {
    Handled,
    NotHandled,
}

use crate::bottom_pane::prompt_args::parse_slash_name;
pub(crate) use chat_composer::ChatComposer;
pub(crate) use chat_composer::ChatComposerConfig;
pub(crate) use chat_composer::ComposerDraftSnapshot;
pub(crate) use chat_composer::InputResult;
pub(crate) use chat_composer::QueuedInputAction;
pub(crate) use chat_composer_history::HistoryEntry;
pub(crate) use textarea::KillBufferSnapshot;

use crate::status_indicator_widget::StatusDetailsCapitalization;
use crate::status_indicator_widget::StatusIndicatorWidget;
pub(crate) use list_selection_view::SELECTION_TOGGLE_BLOCKED_PREFIX;
pub(crate) use list_selection_view::SELECTION_TOGGLE_UNAVAILABLE_PREFIX;
pub(crate) use list_selection_view::SelectionAction;
pub(crate) use list_selection_view::SelectionItem;
pub(crate) use list_selection_view::SelectionSecondaryAction;

struct DelayedApprovalRequest {
    request: ApprovalRequest,
    features: Features,
}

/// Pane displayed in the lower half of the chat UI.
///
/// This is the owning container for the prompt input (`ChatComposer`) and the view stack
/// (`BottomPaneView`). It performs local input routing and renders time-based hints, while leaving
/// process-level decisions (quit, interrupt, shutdown) to `ChatWidget`.
pub(crate) struct BottomPane {
    /// Composer is retained even when a BottomPaneView is displayed so the
    /// input state is retained when the view is closed.
    composer: ChatComposer,

    /// Stack of views displayed instead of the composer (e.g. popups/modals).
    view_stack: Vec<Box<dyn BottomPaneView>>,
    pub(crate) questions: Option<Box<AsyncQuestions>>,
    delayed_approval_requests: VecDeque<DelayedApprovalRequest>,
    last_composer_activity_at: Option<Instant>,

    app_event_tx: AppEventSender,
    frame_requester: FrameRequester,
    thread_id: Option<ThreadId>,

    has_input_focus: bool,
    enhanced_keys_supported: bool,
    disable_paste_burst: bool,
    is_task_running: bool,
    esc_backtrack_hint: bool,
    animations_enabled: bool,

    /// Inline status indicator shown above the composer while a task is running.
    status: Option<StatusIndicatorWidget>,
    /// Running-hook summary supplied by the lifecycle owner after its reveal delay.
    hook_status_message: Option<String>,
    /// Streaming may drop the row without losing its elapsed time or modal pause.
    status_timer: crate::status_indicator_widget::StatusTimer,
    /// Unified exec session summary source.
    ///
    /// When a status row exists, this summary is mirrored inline in that row;
    /// when no status row exists, it renders as its own footer row.
    unified_exec_footer: UnifiedExecFooter,
    /// Preview of pending steers and queued drafts shown above the composer.
    pending_input_preview: PendingInputPreview,
    /// Inactive threads with pending approval requests.
    pending_thread_approvals: PendingThreadApprovals,
    context_window_percent: Option<i64>,
    context_window_used_tokens: Option<i64>,
    keymap: RuntimeKeymap,
}

pub(crate) struct BottomPaneParams {
    pub(crate) app_event_tx: AppEventSender,
    pub(crate) frame_requester: FrameRequester,
    pub(crate) has_input_focus: bool,
    pub(crate) enhanced_keys_supported: bool,
    pub(crate) placeholder_text: String,
    pub(crate) disable_paste_burst: bool,
    pub(crate) animations_enabled: bool,
    pub(crate) skills: Option<Vec<SkillMetadata>>,
}

impl BottomPane {
    pub fn new(params: BottomPaneParams) -> Self {
        Self::new_with_composer_config(params, ChatComposerConfig::default())
    }

    /// Construct a bottom pane with explicitly restricted composer behavior.
    pub(crate) fn new_with_composer_config(
        params: BottomPaneParams,
        composer_config: ChatComposerConfig,
    ) -> Self {
        let BottomPaneParams {
            app_event_tx,
            frame_requester,
            has_input_focus,
            enhanced_keys_supported,
            placeholder_text,
            disable_paste_burst,
            animations_enabled,
            skills,
        } = params;
        let mut composer = ChatComposer::new_with_config(
            has_input_focus,
            app_event_tx.clone(),
            enhanced_keys_supported,
            placeholder_text,
            disable_paste_burst,
            composer_config,
        );
        composer.set_frame_requester(frame_requester.clone());
        let keymap = RuntimeKeymap::defaults();
        composer.set_keymap_bindings(&keymap);
        composer.set_skill_mentions(skills);
        Self {
            composer,
            view_stack: Vec::new(),
            questions: None,
            delayed_approval_requests: VecDeque::new(),
            last_composer_activity_at: None,
            app_event_tx,
            frame_requester,
            thread_id: None,
            has_input_focus,
            enhanced_keys_supported,
            disable_paste_burst,
            is_task_running: false,
            status: None,
            hook_status_message: None,
            status_timer: crate::status_indicator_widget::StatusTimer::default(),
            unified_exec_footer: UnifiedExecFooter::new(),
            pending_input_preview: PendingInputPreview::new(),
            pending_thread_approvals: PendingThreadApprovals::new(),
            esc_backtrack_hint: false,
            animations_enabled,
            context_window_percent: None,
            context_window_used_tokens: None,
            keymap,
        }
    }

    pub fn set_skills(&mut self, skills: Option<Vec<SkillMetadata>>) {
        self.composer.set_skill_mentions(skills);
        self.request_redraw();
    }

    /// Update image-paste behavior for the active composer and repaint immediately.
    ///
    /// Callers use this to keep composer affordances aligned with model capabilities.
    pub fn set_image_paste_enabled(&mut self, enabled: bool) {
        self.composer.set_image_paste_enabled(enabled);
        self.request_redraw();
    }

    /// Mirrors the effective reasoning effort into the composer so its next
    /// visible frame can play a one-shot Max/Ultra effect.
    pub(crate) fn set_active_reasoning_effort(&mut self, effort: Option<&ReasoningEffort>) {
        let animations_enabled = effort_ignition::effort_animation_enabled(
            self.animations_enabled,
            effective_stdout_color_level(),
        );
        if self
            .composer
            .set_active_reasoning_effort(effort, animations_enabled)
        {
            self.request_redraw();
        }
    }

    /// Establishes a restored thread's effort without replaying its one-shot animation.
    pub(crate) fn set_active_reasoning_effort_baseline(
        &mut self,
        effort: Option<&ReasoningEffort>,
    ) {
        self.composer.set_active_reasoning_effort_baseline(effort);
    }

    pub fn set_plugin_mentions(&mut self, plugins: Option<Vec<PluginCapabilitySummary>>) {
        self.composer.set_plugin_mentions(plugins);
        self.request_redraw();
    }

    pub(crate) fn set_agents_navigation_enabled(&mut self, enabled: bool) {
        self.composer.set_agents_navigation_enabled(enabled);
        self.request_redraw();
    }

    pub(crate) fn set_task_mentions_enabled(&mut self, enabled: bool) {
        self.composer.set_task_mentions_enabled(enabled);
        self.request_redraw();
    }

    pub(crate) fn task_mentions_enabled(&self) -> bool {
        self.composer.task_mentions_enabled()
    }

    pub(crate) fn on_task_search_result(
        &mut self,
        query: &str,
        matches: Vec<crate::task_mentions::TaskMention>,
    ) {
        self.composer.on_task_search_result(query, matches);
        self.request_redraw();
    }

    pub fn set_plugins_command_enabled(&mut self, enabled: bool) {
        self.composer.set_plugins_command_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_token_activity_command_enabled(&mut self, enabled: bool) {
        self.composer.set_token_activity_command_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_mentions_v2_enabled(&mut self, enabled: bool) {
        self.composer.set_mentions_v2_enabled(enabled);
        self.request_redraw();
    }

    pub fn take_mention_bindings(&mut self) -> Vec<MentionBinding> {
        self.composer.take_mention_bindings()
    }

    pub fn take_recent_submission_mention_bindings(&mut self) -> Vec<MentionBinding> {
        self.composer.take_recent_submission_mention_bindings()
    }

    /// Add a staged slash-command draft to the composer's local recall list.
    ///
    /// This should be called exactly once after `ChatWidget` dispatches a recognized command.
    /// Slash recall records the submitted command text regardless of whether the command succeeds.
    pub(crate) fn record_pending_slash_command_history(&mut self) {
        self.composer.record_pending_slash_command_history();
    }

    /// Replace all bottom-pane keymap caches from one resolved runtime keymap.
    ///
    /// The bottom pane owns several input surfaces: composer, overlays, and
    /// selection views. Applying one snapshot through this method keeps those
    /// surfaces synchronized after config reloads or interactive remaps. Callers
    /// should not update the composer directly unless they deliberately want
    /// overlays and selection views to continue using the previous bindings.
    pub fn set_keymap_bindings(&mut self, keymap: &RuntimeKeymap) {
        self.keymap = keymap.clone();
        self.composer.set_keymap_bindings(keymap);
        if let Some(questions) = &mut self.questions {
            questions.set_keymap(keymap);
        }
        let interrupt_binding = keymap.primary_hint(KeymapContext::Chat, "interrupt_turn");
        self.pending_input_preview
            .set_interrupt_binding(interrupt_binding);
        if let Some(status) = self.status.as_mut() {
            status.set_interrupt_binding(interrupt_binding);
        }
        self.request_redraw();
    }

    pub(crate) fn transcript_shortcut_hint(&self) -> Option<crate::key_hint::ShortcutHint> {
        self.keymap
            .primary_hint(KeymapContext::Global, "open_transcript")
    }

    /// Clear pending attachments and mention bindings e.g. when a slash command doesn't submit text.
    pub(crate) fn drain_pending_submission_state(&mut self) {
        let _ = self.take_recent_submission_images_with_placeholders();
        let _ = self.take_remote_image_urls();
        let _ = self.take_recent_submission_mention_bindings();
        let _ = self.take_mention_bindings();
    }

    pub fn set_collaboration_modes_enabled(&mut self, enabled: bool) {
        self.composer.set_collaboration_modes_enabled(enabled);
        self.request_redraw();
    }

    #[cfg(target_os = "windows")]
    pub fn set_windows_degraded_sandbox_active(&mut self, enabled: bool) {
        self.composer.set_windows_degraded_sandbox_active(enabled);
        self.request_redraw();
    }

    pub fn set_collaboration_mode_indicator(
        &mut self,
        indicator: Option<CollaborationModeIndicator>,
    ) {
        self.composer.set_collaboration_mode_indicator(indicator);
        self.request_redraw();
    }

    pub fn set_goal_status_indicator(&mut self, indicator: Option<GoalStatusIndicator>) {
        self.composer.set_goal_status_indicator(indicator);
        self.request_redraw();
    }

    pub fn set_ide_context_active(&mut self, active: bool) {
        self.composer.set_ide_context_active(active);
        self.request_redraw();
    }

    pub fn set_worktrees_enabled(&mut self, enabled: bool) {
        self.composer.set_worktrees_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_service_tier_commands_enabled(&mut self, enabled: bool) {
        self.composer.set_service_tier_commands_enabled(enabled);
        self.request_redraw();
    }

    pub fn set_service_tier_commands(&mut self, commands: Vec<ServiceTierCommand>) {
        self.composer.set_service_tier_commands(commands);
        self.request_redraw();
    }

    pub fn set_goal_command_enabled(&mut self, enabled: bool) {
        self.composer.set_goal_command_enabled(enabled);
        self.request_redraw();
    }

    pub(crate) fn set_side_conversation_active(&mut self, active: bool) {
        self.composer.set_side_conversation_active(active);
        self.request_redraw();
    }

    pub(crate) fn set_placeholder_text(&mut self, placeholder: String) {
        self.composer.set_placeholder_text(placeholder);
        self.request_redraw();
    }

    pub(crate) fn set_parent_owned_thread(&mut self) {
        self.composer.set_parent_owned_thread();
        self.request_redraw();
    }

    /// Update the key hint shown next to queued messages so it matches the
    /// binding that `ChatWidget` actually listens for.
    pub(crate) fn set_queued_message_edit_binding(
        &mut self,
        binding: Option<crate::key_hint::ShortcutHint>,
    ) {
        self.pending_input_preview.set_edit_binding(binding);
        if let Some(questions) = &mut self.questions {
            questions.next_hint = binding;
        }
        self.request_redraw();
    }

    pub(crate) fn set_vim_enabled(&mut self, enabled: bool) {
        self.composer.set_vim_enabled(enabled);
        if let Some(questions) = &mut self.questions {
            questions.set_vim_enabled(enabled);
        }
        self.request_redraw();
    }

    pub(crate) fn take_kill_buffer_snapshot(&mut self) -> KillBufferSnapshot {
        self.composer.take_kill_buffer_snapshot()
    }

    pub(crate) fn restore_kill_buffer_snapshot(&mut self, snapshot: KillBufferSnapshot) {
        self.composer.restore_kill_buffer_snapshot(snapshot);
    }

    pub(crate) fn toggle_vim_enabled(&mut self) -> bool {
        let enabled = self.composer.toggle_vim_enabled();
        if let Some(questions) = &mut self.questions {
            questions.set_vim_enabled(enabled);
        }
        self.request_redraw();
        enabled
    }

    pub fn status_widget(&self) -> Option<&StatusIndicatorWidget> {
        self.status.as_ref()
    }

    pub(crate) fn status_elapsed(&self) -> Option<Duration> {
        self.is_task_running
            .then(|| self.status_timer.elapsed_at(Instant::now()))
    }

    pub(crate) fn reset_status_timer(&mut self, elapsed: Duration) {
        self.status_timer.reset(elapsed);
    }

    pub(crate) fn set_status_timer_origin(&mut self, started_at: Option<Instant>) {
        self.status_timer.display_started_at = started_at;
        self.request_redraw();
    }

    pub fn skills(&self) -> Option<&Vec<SkillMetadata>> {
        self.composer.skills()
    }

    pub fn plugins(&self) -> Option<&Vec<PluginCapabilitySummary>> {
        self.composer.plugins()
    }

    #[cfg(test)]
    pub(crate) fn context_window_percent(&self) -> Option<i64> {
        self.context_window_percent
    }

    #[cfg(test)]
    pub(crate) fn context_window_used_tokens(&self) -> Option<i64> {
        self.context_window_used_tokens
    }

    fn active_view(&self) -> Option<&dyn BottomPaneView> {
        self.view_stack.last().map(std::convert::AsRef::as_ref)
    }

    fn push_view(&mut self, view: Box<dyn BottomPaneView>) {
        self.view_stack.push(view);
        self.schedule_active_view_frame();
        self.request_redraw();
    }

    fn pop_active_view_with_completion(&mut self, completion: Option<ViewCompletion>) {
        if self.view_stack.pop().is_some() {
            match completion {
                Some(ViewCompletion::Accepted) => {
                    while self
                        .view_stack
                        .last()
                        .is_some_and(|view| view.dismiss_after_child_accept())
                    {
                        self.view_stack.pop();
                    }
                }
                Some(ViewCompletion::Cancelled) => {
                    if let Some(view) = self.view_stack.last_mut() {
                        view.clear_dismiss_after_child_accept();
                    }
                }
                None => {}
            }
            self.on_view_stack_depth_decreased();
        }
    }

    fn on_view_stack_depth_decreased(&mut self) {
        if self.view_stack.is_empty() {
            self.on_active_view_complete();
        }
    }

    fn approval_prompt_delay_remaining(&self, now: Instant) -> Option<Duration> {
        self.last_composer_activity_at.and_then(|last_activity_at| {
            last_activity_at
                .checked_add(APPROVAL_PROMPT_TYPING_IDLE_DELAY)
                .and_then(|show_at| show_at.checked_duration_since(now))
                .filter(|delay| !delay.is_zero())
        })
    }

    fn record_composer_activity_at(&mut self, now: Instant) {
        self.last_composer_activity_at = Some(now);
        if self.has_pending_approval()
            && let Some(delay) = self.approval_prompt_delay_remaining(now)
        {
            self.request_redraw_in(delay);
        }
    }

    fn maybe_show_delayed_approval_requests_at(&mut self, now: Instant) {
        if !self.has_pending_approval() || !self.view_stack.is_empty() {
            return;
        }
        if let Some(delay) = self.approval_prompt_delay_remaining(now) {
            self.request_redraw_in(delay);
            return;
        }

        // Promote the oldest delayed approval once typing has been idle long enough.
        // `ApprovalOverlay` advances its internal queue with `pop()`, so drain the
        // remaining delayed approvals from the back to preserve FIFO display order.
        let Some(first) = self.delayed_approval_requests.pop_front() else {
            return;
        };
        let mut modal = ApprovalOverlay::new(
            first.request,
            self.app_event_tx.clone(),
            first.features,
            self.keymap.approval.clone(),
            self.keymap.list.clone(),
        );
        while let Some(delayed) = self.delayed_approval_requests.pop_back() {
            modal.enqueue_request(delayed.request);
        }
        self.pause_status_timer_for_modal();
        self.push_view(Box::new(modal));
    }

    /// Edit the draft without invoking popups, submissions, or remote actions.
    pub(crate) fn handle_disconnected_key(&mut self, key: KeyEvent) {
        self.view_stack.clear();
        self.delayed_approval_requests.clear();
        self.composer
            .set_input_enabled(/*enabled*/ true, /*placeholder*/ None);
        self.composer.handle_disconnected_key(key);
        self.request_redraw();
    }

    /// Forward a key event to the active view or the composer.
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> InputResult {
        let records_composer_activity =
            matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                && !key_hint::has_ctrl_or_alt(key_event.modifiers)
                && matches!(
                    key_event.code,
                    KeyCode::Char(_)
                        | KeyCode::Backspace
                        | KeyCode::Delete
                        | KeyCode::Enter
                        | KeyCode::Tab
                );
        if self.view_stack.is_empty()
            && let Some(questions) = self.questions.as_mut().filter(|q| q.expanded)
        {
            questions
                .composer
                .copy_history_for_key(&self.composer, key_event);
            questions.composer.set_task_running(self.is_task_running);
            questions.handle_key_event(key_event);
            if records_composer_activity {
                self.record_composer_activity_at(Instant::now());
            }
            self.request_redraw();
            self.schedule_active_view_frame();
            return InputResult::None;
        }
        // If a modal/view is active, handle it here; otherwise forward to composer.
        if !self.view_stack.is_empty() {
            if key_event.kind == KeyEventKind::Release {
                return InputResult::None;
            }

            // We need three pieces of information after routing the key:
            // whether Esc completed the view, whether the view finished for any
            // reason, and whether a paste-burst timer should be scheduled.
            let (ctrl_c_completed, view_complete, completion, view_in_paste_burst) = {
                let last_index = self.view_stack.len() - 1;
                let view = &mut self.view_stack[last_index];
                let prefer_esc =
                    key_event.code == KeyCode::Esc && view.prefer_esc_to_handle_key_event();
                let ctrl_c_completed = key_event.code == KeyCode::Esc
                    && !prefer_esc
                    && matches!(view.on_ctrl_c(), CancellationEvent::Handled)
                    && view.is_complete();
                if ctrl_c_completed {
                    (true, true, view.completion(), false)
                } else {
                    view.handle_key_event(key_event);
                    (
                        false,
                        view.is_complete(),
                        view.completion(),
                        view.is_in_paste_burst(),
                    )
                }
            };

            if ctrl_c_completed {
                self.pop_active_view_with_completion(completion);
                if let Some(next_view) = self.view_stack.last()
                    && next_view.is_in_paste_burst()
                {
                    self.request_redraw_in(ChatComposer::recommended_paste_flush_delay());
                }
            } else if view_complete {
                self.pop_active_view_with_completion(completion);
            } else if view_in_paste_burst {
                self.request_redraw_in(ChatComposer::recommended_paste_flush_delay());
            }
            self.request_redraw();
            InputResult::None
        } else {
            // Allow the configured interrupt while a task is running, even when its
            // status row is hidden or the composer has focus.
            // When a popup is active, prefer dismissing it over interrupting the task.
            if self.should_interrupt_running_task(key_event) {
                if let Some(status) = &self.status {
                    status.interrupt();
                } else {
                    self.app_event_tx.interrupt();
                }
                self.request_redraw();
                return InputResult::None;
            }
            let (input_result, needs_redraw) = self.composer.handle_key_event(key_event);
            if records_composer_activity {
                self.record_composer_activity_at(Instant::now());
            }
            if needs_redraw {
                self.request_redraw();
            }
            if self.composer.is_in_paste_burst() {
                self.request_redraw_in(ChatComposer::recommended_paste_flush_delay());
            }
            input_result
        }
    }

    /// Return the contexts whose ordinary handlers can consume the next key.
    pub(crate) fn keymap_contexts(&self) -> KeymapContextSet {
        if let Some(view) = self.view_stack.last() {
            view.keymap_contexts()
        } else if let Some(questions) = self.questions.as_ref().filter(|q| q.expanded) {
            questions.keymap_contexts()
        } else {
            self.composer.keymap_contexts()
        }
    }

    /// Handles a Ctrl+C press within the bottom pane.
    ///
    /// An active modal view is given the first chance to consume the key (typically to dismiss
    /// itself). If no view is active, Ctrl+C cancels active history search before falling back to
    /// clearing draft composer input.
    ///
    /// This method may show the quit shortcut hint as a user-visible acknowledgement that Ctrl+C
    /// was received, but it does not decide whether the process should exit; `ChatWidget` owns the
    /// quit/interrupt state machine and uses the result to decide what happens next.
    pub(crate) fn on_ctrl_c(&mut self) -> CancellationEvent {
        if self.view_stack.is_empty()
            && let Some(questions) = self.questions.as_mut().filter(|q| q.expanded)
        {
            return questions.on_ctrl_c();
        }
        if let Some(view) = self.view_stack.last_mut() {
            let event = view.on_ctrl_c();
            let view_complete = view.is_complete();
            let completion = view.completion();
            if matches!(event, CancellationEvent::Handled) {
                if view_complete {
                    self.pop_active_view_with_completion(completion);
                }
                self.show_quit_shortcut_hint(key_hint::ctrl(KeyCode::Char('c')));
                self.request_redraw();
            }
            event
        } else if self.composer.cancel_vim_search() || self.composer.cancel_history_search() {
            self.request_redraw();
            CancellationEvent::Handled
        } else if self.composer_is_empty() {
            CancellationEvent::NotHandled
        } else {
            self.view_stack.pop();
            self.clear_composer_for_ctrl_c();
            self.show_quit_shortcut_hint(key_hint::ctrl(KeyCode::Char('c')));
            self.request_redraw();
            CancellationEvent::Handled
        }
    }

    pub fn handle_paste(&mut self, pasted: String) {
        if self.view_stack.is_empty()
            && let Some(questions) = self.questions.as_mut().filter(|q| q.expanded)
        {
            questions.handle_paste(pasted);
            self.record_composer_activity_at(Instant::now());
            self.request_redraw();
            return;
        }
        let has_pasted_text = !pasted.is_empty();
        if let Some(view) = self.view_stack.last_mut() {
            let needs_redraw = view.handle_paste(pasted);
            let view_complete = view.is_complete();
            if view_complete {
                self.view_stack.clear();
                self.on_active_view_complete();
            }
            if needs_redraw || view_complete {
                self.request_redraw();
            }
        } else {
            let needs_redraw = self.composer.handle_paste(pasted);
            if has_pasted_text {
                self.record_composer_activity_at(Instant::now());
            }
            if needs_redraw {
                self.request_redraw();
            }
        }
    }

    pub(crate) fn insert_str(&mut self, text: &str) {
        self.composer.insert_str(text);
        self.request_redraw();
    }

    pub(crate) fn pre_draw_tick(&mut self) {
        self.pre_draw_tick_at(Instant::now());
    }

    fn pre_draw_tick_at(&mut self, now: Instant) {
        self.composer.sync_popups();
        self.maybe_show_delayed_approval_requests_at(now);
        self.tick_active_view(now);
        self.schedule_active_view_frame();
    }

    fn tick_active_view(&mut self, now: Instant) {
        let Some(view) = self.view_stack.last_mut() else {
            return;
        };
        let needs_redraw = view.pre_draw_tick(now);
        let view_complete = view.is_complete();
        if view_complete {
            self.view_stack.clear();
            self.on_active_view_complete();
        }
        if needs_redraw || view_complete {
            self.request_redraw();
        }
    }

    fn schedule_active_view_frame(&self) {
        if let Some(questions) = &self.questions {
            if let Some(delay) = questions.next_frame_delay() {
                self.request_redraw_in(delay);
            }
            if questions.is_in_paste_burst() {
                self.request_redraw_in(ChatComposer::recommended_paste_flush_delay());
            }
        }
        if let Some(delay) = self.active_view().map_or_else(
            || self.composer.footer_flash_delay(),
            BottomPaneView::next_frame_delay,
        ) {
            self.request_redraw_in(delay);
        }
    }

    /// Replace the composer text with `text`.
    ///
    /// This is intended for fresh input where mention linkage does not need to
    /// survive; it routes to `ChatComposer::set_text_content`, which resets
    /// mention bindings.
    pub(crate) fn set_composer_text(
        &mut self,
        text: String,
        text_elements: Vec<TextElement>,
        local_image_paths: Vec<PathBuf>,
    ) {
        self.composer
            .set_text_content(text, text_elements, local_image_paths);
        self.composer.move_cursor_to_end();
        self.request_redraw();
    }

    /// Replace the composer text while preserving mention link targets.
    ///
    /// Use this when rehydrating a draft after a local validation/gating
    /// failure (for example unsupported image submit) so previously selected
    /// mention targets remain stable across retry.
    pub(crate) fn set_composer_text_with_mention_bindings(
        &mut self,
        text: String,
        text_elements: Vec<TextElement>,
        local_image_paths: Vec<PathBuf>,
        mention_bindings: Vec<MentionBinding>,
    ) {
        self.composer.set_text_content_with_mention_bindings(
            text,
            text_elements,
            local_image_paths,
            mention_bindings,
        );
        self.composer.move_cursor_to_end();
        self.request_redraw();
    }

    #[allow(dead_code)]
    pub(crate) fn set_composer_input_enabled(
        &mut self,
        enabled: bool,
        placeholder: Option<String>,
    ) {
        self.composer.set_input_enabled(enabled, placeholder);
        self.request_redraw();
    }

    pub(crate) fn show_shutdown_in_progress(&mut self) {
        self.view_stack.clear();
        self.composer.show_shutdown_in_progress();
        self.request_redraw();
    }

    pub(crate) fn clear_composer_for_ctrl_c(&mut self) {
        if let Some(text) = self.composer.clear_for_ctrl_c() {
            if let Some(thread_id) = self.thread_id {
                self.app_event_tx
                    .send(AppEvent::AppendMessageHistoryEntry { thread_id, text });
            } else {
                tracing::warn!(
                    "failed to append Ctrl+C-cleared draft to history: no active thread id"
                );
            }
        }
        self.request_redraw();
    }

    /// Get the current composer text (for tests and programmatic checks).
    pub(crate) fn composer_text(&self) -> String {
        self.composer.current_text()
    }

    #[cfg(test)]
    pub(crate) fn composer_cursor(&self) -> usize {
        self.composer.cursor()
    }

    #[cfg(test)]
    pub(crate) fn composer_text_elements(&self) -> Vec<TextElement> {
        self.composer.text_elements()
    }

    pub(crate) fn composer_local_images(&self) -> Vec<LocalImageAttachment> {
        self.composer.local_images()
    }

    #[cfg(test)]
    pub(crate) fn composer_local_image_paths(&self) -> Vec<PathBuf> {
        self.composer.local_image_paths()
    }

    pub(crate) fn composer_text_with_pending(&self) -> String {
        self.composer.current_text_with_pending()
    }

    /// Returns whether the composer currently accepts interactive draft edits.
    pub(crate) fn composer_input_enabled(&self) -> bool {
        self.composer.input_enabled()
    }

    pub(crate) fn composer_pending_pastes(&self) -> Vec<(String, String)> {
        self.composer.pending_pastes()
    }

    pub(crate) fn apply_external_edit(&mut self, text: String) {
        self.composer.apply_external_edit(text);
        self.request_redraw();
    }

    pub(crate) fn set_footer_hint_override(&mut self, items: Option<Vec<(String, String)>>) {
        self.composer.set_footer_hint_override(items);
        self.request_redraw();
    }

    pub(crate) fn set_remote_image_urls(&mut self, urls: Vec<String>) {
        self.composer.set_remote_image_urls(urls);
        self.request_redraw();
    }

    pub(crate) fn remote_image_urls(&self) -> Vec<String> {
        self.composer.remote_image_urls()
    }

    pub(crate) fn take_remote_image_urls(&mut self) -> Vec<String> {
        let urls = self.composer.take_remote_image_urls();
        self.request_redraw();
        urls
    }

    pub(crate) fn set_composer_pending_pastes(&mut self, pending_pastes: Vec<(String, String)>) {
        self.composer.set_pending_pastes(pending_pastes);
        self.request_redraw();
    }

    /// Update the status indicator header (defaults to "Working") and details below it.
    ///
    /// Passing `None` clears any existing details. Returns whether the active status indicator
    /// was updated and requested a redraw.
    pub(crate) fn update_status(
        &mut self,
        header: String,
        details: Option<String>,
        details_capitalization: StatusDetailsCapitalization,
        details_max_lines: usize,
    ) -> bool {
        if let Some(status) = self.status.as_mut() {
            status.update_header(header);
            status.update_details(details, details_capitalization, details_max_lines.max(1));
            self.request_redraw();
            return true;
        }
        false
    }

    /// Show the transient "press again to quit" hint for `key`.
    ///
    /// `ChatWidget` owns the quit shortcut state machine (it decides when quit is
    /// allowed), while the bottom pane owns rendering. We also schedule a redraw
    /// after [`QUIT_SHORTCUT_TIMEOUT`] so the hint disappears even if the user
    /// stops typing and no other events trigger a draw.
    pub(crate) fn show_quit_shortcut_hint(&mut self, key: KeyBinding) {
        if !DOUBLE_PRESS_QUIT_SHORTCUT_ENABLED {
            return;
        }

        self.composer
            .show_quit_shortcut_hint(key, self.has_input_focus);
        let frame_requester = self.frame_requester.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                tokio::time::sleep(QUIT_SHORTCUT_TIMEOUT).await;
                frame_requester.schedule_frame();
            });
        } else {
            // In tests (and other non-Tokio contexts), fall back to a thread so
            // the hint can still expire without requiring an explicit draw.
            std::thread::spawn(move || {
                std::thread::sleep(QUIT_SHORTCUT_TIMEOUT);
                frame_requester.schedule_frame();
            });
        }
        self.request_redraw();
    }

    /// Clear the "press again to quit" hint immediately.
    pub(crate) fn clear_quit_shortcut_hint(&mut self) {
        self.composer.clear_quit_shortcut_hint(self.has_input_focus);
        self.request_redraw();
    }

    #[cfg(test)]
    pub(crate) fn quit_shortcut_hint_visible(&self) -> bool {
        self.composer.quit_shortcut_hint_visible()
    }

    #[cfg(test)]
    pub(crate) fn status_indicator_visible(&self) -> bool {
        self.status_widget().is_some()
    }

    #[cfg(test)]
    pub(crate) fn status_line_text(&self) -> Option<String> {
        self.composer.status_line_text()
    }

    pub(crate) fn show_esc_backtrack_hint(&mut self) {
        self.esc_backtrack_hint = true;
        self.composer.set_esc_backtrack_hint(/*show*/ true);
        self.request_redraw();
    }

    pub(crate) fn clear_esc_backtrack_hint(&mut self) {
        if self.esc_backtrack_hint {
            self.esc_backtrack_hint = false;
            self.composer.set_esc_backtrack_hint(/*show*/ false);
            self.request_redraw();
        }
    }

    // esc_backtrack_hint_visible removed; hints are controlled internally.

    pub fn set_task_running(&mut self, running: bool) {
        let was_running = self.is_task_running;
        self.is_task_running = running;
        self.composer.set_task_running(running);

        if running {
            if !was_running {
                self.status_timer.reset(Duration::ZERO);
                if self.status.is_none() {
                    self.status = Some(StatusIndicatorWidget::new(
                        self.app_event_tx.clone(),
                        self.frame_requester.clone(),
                        self.animations_enabled,
                    ));
                }
                if let Some(status) = self.status.as_mut() {
                    status.set_interrupt_hint_visible(/*visible*/ true);
                    status.set_interrupt_binding(
                        self.keymap
                            .primary_hint(KeymapContext::Chat, "interrupt_turn"),
                    );
                }
                self.sync_status_inline_message();
                self.request_redraw();
            }
        } else {
            // Hide the status indicator when a task completes, but keep other modal views.
            self.hide_status_indicator();
        }
    }

    pub(crate) fn set_queue_submissions(&mut self, queue_submissions: bool) {
        self.composer.set_queue_submissions(queue_submissions);
    }

    /// Hide the status indicator while leaving task-running state untouched.
    pub(crate) fn hide_status_indicator(&mut self) {
        if self.status.take().is_some() {
            self.request_redraw();
        }
    }

    pub(crate) fn ensure_status_indicator(&mut self) {
        if self.status.is_none() {
            self.status.get_or_insert_with(|| {
                StatusIndicatorWidget::new(
                    self.app_event_tx.clone(),
                    self.frame_requester.clone(),
                    self.animations_enabled,
                )
            });
            if let Some(status) = self.status.as_mut() {
                status.set_interrupt_binding(
                    self.keymap
                        .primary_hint(KeymapContext::Chat, "interrupt_turn"),
                );
            }
            self.sync_status_inline_message();
            self.request_redraw();
        }
    }

    pub(crate) fn set_interrupt_hint_visible(&mut self, visible: bool) {
        if let Some(status) = self.status.as_mut() {
            status.set_interrupt_hint_visible(visible);
            self.request_redraw();
        }
    }

    pub(crate) fn set_context_window(&mut self, percent: Option<i64>, used_tokens: Option<i64>) {
        if self.context_window_percent == percent && self.context_window_used_tokens == used_tokens
        {
            return;
        }

        self.context_window_percent = percent;
        self.context_window_used_tokens = used_tokens;
        self.composer
            .set_context_window(percent, self.context_window_used_tokens);
        self.request_redraw();
    }

    pub(crate) fn set_context_window_pending(&mut self, pending: bool) {
        self.composer.set_context_window_pending(pending);
        self.request_redraw();
    }

    /// Show a generic list selection view with the provided items.
    pub(crate) fn show_selection_view(
        &mut self,
        mut params: list_selection_view::SelectionViewParams,
    ) {
        self.apply_standard_popup_hint(&mut params);
        let view = list_selection_view::ListSelectionView::new(
            params,
            self.app_event_tx.clone(),
            self.keymap.list.clone(),
        );
        self.push_view(Box::new(view));
    }

    fn apply_standard_popup_hint(&self, params: &mut list_selection_view::SelectionViewParams) {
        // Configured list actions take precedence over optional row shortcuts.
        for item in &mut params.items {
            if item.secondary_action.as_ref().is_some_and(|secondary| {
                let (code, modifiers) = secondary.key.parts();
                self.keymap
                    .list
                    .action_for(KeyEvent::new(code, modifiers))
                    .is_some()
            }) {
                item.secondary_action = None;
            }
        }
        if !params.allow_cancel {
            if params.footer_hint.is_none()
                || params.footer_hint.as_ref() == Some(&popup_consts::standard_popup_hint_line())
            {
                params.footer_hint = None;
            }
            return;
        }
        if params.footer_hint.is_none()
            || params.footer_hint.as_ref() == Some(&popup_consts::standard_popup_hint_line())
        {
            params.footer_hint = Some(self.standard_popup_hint_line());
        }
    }

    /// Replace the active selection view when it matches `view_id`.
    pub(crate) fn replace_selection_view_if_active(
        &mut self,
        view_id: &'static str,
        mut params: list_selection_view::SelectionViewParams,
    ) -> bool {
        let is_match = self
            .view_stack
            .last()
            .is_some_and(|view| view.view_id() == Some(view_id));
        if !is_match {
            return false;
        }

        self.view_stack.pop();
        self.apply_standard_popup_hint(&mut params);
        let view = list_selection_view::ListSelectionView::new(
            params,
            self.app_event_tx.clone(),
            self.keymap.list.clone(),
        );
        self.push_view(Box::new(view));
        true
    }

    /// Replace the newest matching selection view without disturbing views stacked above it.
    /// Preserve pending parent cleanup when an already-open child is accepted.
    pub(crate) fn replace_selection_view_if_present(
        &mut self,
        view_id: &'static str,
        mut params: list_selection_view::SelectionViewParams,
    ) -> bool {
        let Some(index) = self
            .view_stack
            .iter()
            .rposition(|view| view.view_id() == Some(view_id))
        else {
            return false;
        };

        let replaces_active_view = index + 1 == self.view_stack.len();
        self.apply_standard_popup_hint(&mut params);
        let mut view = list_selection_view::ListSelectionView::new(
            params,
            self.app_event_tx.clone(),
            self.keymap.list.clone(),
        );
        view.dismiss_after_child_accept = self.view_stack[index].dismiss_after_child_accept();
        self.view_stack[index] = Box::new(view);
        if replaces_active_view {
            self.schedule_active_view_frame();
        }
        self.request_redraw();
        true
    }

    pub(crate) fn standard_popup_hint_line(&self) -> Line<'static> {
        popup_consts::standard_popup_hint_line_for_keymap(&self.keymap.list)
    }

    pub(crate) fn replace_view_if_present(
        &mut self,
        view_id: &'static str,
        view: Box<dyn BottomPaneView>,
    ) {
        if let Some(index) = self
            .view_stack
            .iter()
            .rposition(|existing| existing.view_id() == Some(view_id))
        {
            self.view_stack[index] = view;
            self.request_redraw();
        }
    }

    pub(crate) fn list_keymap(&self) -> crate::keymap::ListKeymap {
        self.keymap.list.clone()
    }

    /// Replace one or more active views whose IDs are in `view_ids` with a
    /// generic list selection view.
    pub(crate) fn replace_active_views_with_selection_view(
        &mut self,
        view_ids: &[&'static str],
        mut params: list_selection_view::SelectionViewParams,
    ) -> bool {
        let is_match = self
            .view_stack
            .last()
            .and_then(|view| view.view_id())
            .is_some_and(|view_id| view_ids.contains(&view_id));
        if !is_match {
            return false;
        }

        while self
            .view_stack
            .last()
            .and_then(|view| view.view_id())
            .is_some_and(|view_id| view_ids.contains(&view_id))
        {
            self.view_stack.pop();
        }
        self.apply_standard_popup_hint(&mut params);
        let view = list_selection_view::ListSelectionView::new(
            params,
            self.app_event_tx.clone(),
            self.keymap.list.clone(),
        );
        self.push_view(Box::new(view));
        true
    }

    pub(crate) fn selected_index_for_active_view(&self, view_id: &'static str) -> Option<usize> {
        self.view_stack
            .last()
            .filter(|view| view.view_id() == Some(view_id))
            .and_then(|view| view.selected_index())
    }

    pub(crate) fn selected_index_for_present_view(&self, view_id: &'static str) -> Option<usize> {
        self.view_stack
            .iter()
            .rfind(|view| view.view_id() == Some(view_id))
            .and_then(|view| view.selected_index())
    }

    pub(crate) fn active_tab_id_for_active_view(&self, view_id: &'static str) -> Option<&str> {
        self.view_stack
            .last()
            .filter(|view| view.view_id() == Some(view_id))
            .and_then(|view| view.active_tab_id())
    }

    /// Forward a suggestion to its matching prompt, even beneath another view.
    pub(crate) fn apply_text_suggestion(
        &mut self,
        request_id: uuid::Uuid,
        suggestion: Option<&str>,
    ) -> bool {
        let changed = self
            .view_stack
            .iter_mut()
            .rev()
            .any(|view| view.apply_text_suggestion(request_id, suggestion));

        if changed {
            self.request_redraw();
        }

        changed
    }

    pub(crate) fn dismiss_active_view_if_id(&mut self, view_id: &'static str) -> bool {
        let is_match = self
            .view_stack
            .last()
            .is_some_and(|view| view.view_id() == Some(view_id));
        if !is_match {
            return false;
        }

        self.view_stack.pop();
        self.request_redraw();
        true
    }

    /// Dismiss the newest matching view without disturbing views stacked above it.
    pub(crate) fn dismiss_view_by_id(&mut self, view_id: &'static str) -> bool {
        let Some(index) = self
            .view_stack
            .iter()
            .rposition(|view| view.view_id() == Some(view_id))
        else {
            return false;
        };

        let removed_active_view = index + 1 == self.view_stack.len();
        self.view_stack.remove(index);
        if removed_active_view {
            self.schedule_active_view_frame();
        }
        self.request_redraw();
        true
    }

    /// Update the pending-input preview shown above the composer.
    pub(crate) fn set_pending_input_preview(
        &mut self,
        queued: Vec<String>,
        pending_steers: Vec<String>,
        rejected_steers: Vec<String>,
    ) {
        self.pending_input_preview.pending_steers = pending_steers;
        self.pending_input_preview.rejected_steers = rejected_steers;
        self.pending_input_preview.queued_messages = queued;
        self.request_redraw();
    }

    /// Update the inactive-thread approval list shown above the composer.
    pub(crate) fn set_pending_thread_approvals(&mut self, threads: Vec<String>) {
        if self.pending_thread_approvals.set_threads(threads) {
            self.request_redraw();
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_thread_approvals(&self) -> &[String] {
        self.pending_thread_approvals.threads()
    }

    /// Update the unified-exec process set and refresh whichever summary surface is active.
    ///
    /// The summary may be displayed inline in the status row or as a dedicated
    /// footer row depending on whether a status indicator is currently visible.
    pub(crate) fn set_unified_exec_processes(&mut self, processes: Vec<String>) {
        if self.unified_exec_footer.set_processes(processes) {
            self.sync_status_inline_message();
            self.request_redraw();
        }
    }

    /// Update hook activity after the lifecycle reveal delay, even outside a turn.
    pub(crate) fn set_hook_status_message(&mut self, message: Option<String>) {
        if self.hook_status_message == message {
            return;
        }

        self.hook_status_message = message;
        if self.hook_status_message.is_some() && self.status.is_none() && self.is_task_running() {
            self.ensure_status_indicator();
        } else {
            self.sync_status_inline_message();
            self.request_redraw();
        }
    }

    /// Copy background activity and hook text into the active status row, if any.
    ///
    /// This keeps status-line inline text synchronized without forcing the
    /// standalone unified-exec footer row to be visible.
    fn sync_status_inline_message(&mut self) {
        if let Some(status) = self.status.as_mut() {
            status.update_inline_message(self.unified_exec_footer.summary_text());
            status.update_hook_status_message(self.hook_status_message.clone());
        }
    }

    pub(crate) fn composer_is_empty(&self) -> bool {
        self.composer.is_empty()
    }

    pub(crate) fn composer_is_vim_enabled(&self) -> bool {
        self.composer.is_vim_enabled()
    }

    pub(crate) fn composer_should_handle_vim_insert_escape(&self, key_event: KeyEvent) -> bool {
        self.composer.should_handle_vim_insert_escape(key_event)
    }

    pub(crate) fn is_task_running(&self) -> bool {
        self.is_task_running
    }

    pub(crate) fn should_interrupt_running_task(&self, key_event: KeyEvent) -> bool {
        let is_agent_command = self
            .composer_text()
            .lines()
            .next()
            .and_then(parse_slash_name)
            .is_some_and(|(name, _, _)| matches!(name, "agents" | "subagents"));

        self.keymap.chat.interrupt_turn.is_pressed(key_event)
            && self.is_task_running
            && !(is_agent_command && key_event.code == KeyCode::Esc)
            && self.no_modal_or_popup_active()
            && !self.composer_should_handle_vim_insert_escape(key_event)
    }

    pub(crate) fn terminal_title_requires_action(&self) -> bool {
        self.questions
            .as_ref()
            .is_some_and(|q| q.unanswered_count() > 0)
            || self
                .active_view()
                .is_some_and(bottom_pane_view::BottomPaneView::terminal_title_requires_action)
    }

    pub(crate) fn has_active_view(&self) -> bool {
        !self.view_stack.is_empty()
    }

    pub(crate) fn active_view_will_interrupt_turn_on_key_event(&self, key_event: KeyEvent) -> bool {
        self.is_task_running
            && self
                .active_view()
                .is_some_and(|view| view.will_interrupt_turn_on_key_event(key_event))
    }

    #[cfg(test)]
    pub(crate) fn active_view_id(&self) -> Option<&'static str> {
        self.view_stack.last().and_then(|view| view.view_id())
    }

    /// Return true when the pane is in the regular composer state without any
    /// overlays or popups and not running a task. This is the safe context to
    /// use Esc-Esc for backtracking from the main view.
    pub(crate) fn is_normal_backtrack_mode(&self) -> bool {
        !self.is_task_running
            && !self.questions.as_ref().is_some_and(|q| q.expanded)
            && self.view_stack.is_empty()
            && !self.composer.popup_active()
    }

    /// Return true when no popups or modal views are active, regardless of task state.
    pub(crate) fn can_launch_external_editor(&self) -> bool {
        self.view_stack.is_empty()
            && !self.composer.popup_active()
            && !self.questions.as_ref().is_some_and(|q| q.expanded)
    }

    /// Returns true when the bottom pane has no active modal view and no active composer popup.
    ///
    /// This is the UI-level definition of "no modal/popup is active" for key routing decisions.
    /// It intentionally does not include task state, since some actions are safe while a task is
    /// running and some are not.
    pub(crate) fn no_modal_or_popup_active(&self) -> bool {
        self.can_launch_external_editor()
    }

    pub(crate) fn show_view(&mut self, view: Box<dyn BottomPaneView>) {
        self.push_view(view);
    }

    /// Show a text prompt with the composer's current editing preferences.
    pub(crate) fn show_text_prompt(&mut self, mut view: custom_prompt_view::CustomPromptView) {
        view.set_keymap_bindings(&self.keymap);
        if self.composer_is_vim_enabled() {
            view.enable_vim_in_insert_mode();
        }
        self.push_view(Box::new(view));
    }

    /// Called when the agent requests user approval.
    pub fn push_approval_request(&mut self, request: ApprovalRequest, features: &Features) {
        let request = if let Some(view) = self.view_stack.last_mut() {
            match view.try_consume_approval_request(request) {
                Some(request) => request,
                None => {
                    self.request_redraw();
                    return;
                }
            }
        } else {
            request
        };

        let now = Instant::now();
        if !self.delayed_approval_requests.is_empty()
            || self.approval_prompt_delay_remaining(now).is_some()
        {
            self.delayed_approval_requests
                .push_back(DelayedApprovalRequest {
                    request,
                    features: features.clone(),
                });
            self.maybe_show_delayed_approval_requests_at(now);
        } else {
            // No recent composer activity, so show the approval modal immediately.
            let modal = ApprovalOverlay::new(
                request,
                self.app_event_tx.clone(),
                features.clone(),
                self.keymap.approval.clone(),
                self.keymap.list.clone(),
            );
            self.pause_status_timer_for_modal();
            self.push_view(Box::new(modal));
        }
    }

    /// Called when the agent requests user input.
    pub fn push_user_input_request(&mut self, request: ToolRequestUserInputParams) {
        let request = if let Some(view) = self.view_stack.last_mut() {
            match view.try_consume_user_input_request(request) {
                Some(request) => request,
                None => {
                    self.request_redraw();
                    return;
                }
            }
        } else {
            request
        };

        let modal = RequestUserInputOverlay::new_with_keymap(
            request,
            self.app_event_tx.clone(),
            self.has_input_focus,
            self.enhanced_keys_supported,
            self.disable_paste_burst,
            self.keymap.clone(),
        );
        self.pause_status_timer_for_modal();
        self.set_composer_input_enabled(
            /*enabled*/ false,
            Some("Answer the questions to continue.".to_string()),
        );
        self.push_view(Box::new(modal));
    }

    pub(crate) fn push_mcp_server_elicitation_request(
        &mut self,
        request: McpServerElicitationFormRequest,
    ) {
        let request = if let Some(view) = self.view_stack.last_mut() {
            match view.try_consume_mcp_server_elicitation_request(request) {
                Some(request) => request,
                None => {
                    self.request_redraw();
                    return;
                }
            }
        } else {
            request
        };

        let modal = McpServerElicitationOverlay::new_with_keymap(
            request,
            self.app_event_tx.clone(),
            self.has_input_focus,
            self.enhanced_keys_supported,
            self.disable_paste_burst,
            self.keymap.clone(),
        );
        self.pause_status_timer_for_modal();
        self.set_composer_input_enabled(
            /*enabled*/ false,
            Some("Respond to the MCP server request to continue.".to_string()),
        );
        self.push_view(Box::new(modal));
    }

    pub(crate) fn dismiss_app_server_request(
        &mut self,
        request: &ResolvedAppServerRequest,
    ) -> bool {
        let delayed_len = self.delayed_approval_requests.len();
        self.delayed_approval_requests
            .retain(|delayed| !delayed.request.matches_resolved_request(request));
        let delayed_changed = self.delayed_approval_requests.len() != delayed_len;

        if self.view_stack.is_empty() {
            if delayed_changed {
                self.request_redraw();
            }
            return delayed_changed;
        }

        let mut changed = delayed_changed;
        let mut completed_indices = Vec::new();
        for index in (0..self.view_stack.len()).rev() {
            let view = &mut self.view_stack[index];
            if !view.dismiss_app_server_request(request) {
                continue;
            }
            changed = true;
            if view.is_complete() {
                completed_indices.push(index);
            }
        }
        if !changed {
            return false;
        }
        for index in completed_indices {
            self.view_stack.remove(index);
        }
        self.on_view_stack_depth_decreased();
        self.request_redraw();
        true
    }

    fn on_active_view_complete(&mut self) {
        self.resume_status_timer_after_modal();
        self.set_composer_input_enabled(/*enabled*/ true, /*placeholder*/ None);
    }

    fn pause_status_timer_for_modal(&mut self) {
        self.status_timer.pause_at(Instant::now());
    }

    fn resume_status_timer_after_modal(&mut self) {
        self.status_timer.resume_at(Instant::now());
        self.request_redraw();
    }

    /// Height (terminal rows) required by the current bottom pane.
    pub(crate) fn request_redraw(&self) {
        self.frame_requester.schedule_frame();
    }

    pub(crate) fn request_redraw_in(&self, dur: Duration) {
        self.frame_requester.schedule_frame_in(dur);
    }

    // --- History helpers ---

    pub(crate) fn set_history_metadata(
        &mut self,
        thread_id: ThreadId,
        log_id: u64,
        entry_count: usize,
    ) {
        self.thread_id = Some(thread_id);
        self.composer
            .set_history_metadata(thread_id, log_id, entry_count);
    }

    pub(crate) fn flush_paste_burst_if_due(&mut self) -> bool {
        // Every editor can retain buffered input after losing focus. Flush all
        // of them, matching the buffers that can defer drawing below.
        let mut flushed = self.composer.flush_paste_burst_if_due();
        if let Some(view) = self.view_stack.last_mut() {
            flushed |= view.flush_paste_burst_if_due();
        }
        if let Some(questions) = &mut self.questions {
            flushed |= questions.flush_paste_burst_if_due();
        }
        flushed
    }

    pub(crate) fn is_in_paste_burst(&self) -> bool {
        self.view_stack
            .last()
            .is_some_and(|view| view.is_in_paste_burst())
            || self
                .questions
                .as_deref()
                .is_some_and(bottom_pane_view::BottomPaneView::is_in_paste_burst)
            || self.composer.is_in_paste_burst()
    }

    pub(crate) fn on_history_lookup_response(&mut self, response: HistoryLookupResponse) {
        let mut updated = false;
        for composer in std::iter::once(&mut self.composer)
            .chain(self.questions.iter_mut().map(|q| &mut q.composer))
        {
            updated |= match response.clone() {
                HistoryLookupResponse::Entry {
                    offset,
                    log_id,
                    entry,
                } => composer.on_history_entry_response(log_id, offset, entry),
                HistoryLookupResponse::Batch {
                    cursor,
                    log_id,
                    entries,
                    next_older_cursor,
                } => composer.on_history_batch_response(log_id, cursor, entries, next_older_cursor),
                HistoryLookupResponse::BatchError { cursor, log_id } => {
                    composer.on_history_batch_error(log_id, cursor)
                }
            };
        }
        if updated {
            self.request_redraw();
        }
    }

    pub(crate) fn record_replayed_user_message_history(&mut self, entry: HistoryEntry) {
        self.composer.record_replayed_user_message_history(entry);
    }

    pub(crate) fn on_file_search_result(&mut self, query: String, matches: Vec<FileMatch>) {
        self.composer.on_file_search_result(query, matches);
        self.request_redraw();
    }

    pub(crate) fn attach_image(&mut self, path: PathBuf) {
        if self.view_stack.is_empty() {
            self.composer.attach_image(path);
            self.request_redraw();
        }
    }

    #[cfg(test)]
    pub(crate) fn take_recent_submission_images(&mut self) -> Vec<PathBuf> {
        self.composer.take_recent_submission_images()
    }

    pub(crate) fn take_recent_submission_images_with_placeholders(
        &mut self,
    ) -> Vec<LocalImageAttachment> {
        self.composer
            .take_recent_submission_images_with_placeholders()
    }

    pub(crate) fn prepare_inline_args_submission(
        &mut self,
        record_history: bool,
    ) -> Option<(String, Vec<TextElement>)> {
        self.composer.prepare_inline_args_submission(record_history)
    }

    fn as_renderable(&'_ self) -> RenderableItem<'_> {
        self.as_renderable_with_composer_right_reserve(/*composer_right_reserve*/ 0)
    }

    pub(crate) fn as_renderable_with_composer_right_reserve(
        &'_ self,
        composer_right_reserve: u16,
    ) -> RenderableItem<'_> {
        if let Some(view) = self.active_view() {
            RenderableItem::Borrowed(view)
        } else {
            let mut flex = FlexRenderable::new();
            if let Some(status) = self.status_widget() {
                flex.push(
                    /*flex*/ 0,
                    RenderableItem::Owned(Box::new(status.with_timer(&self.status_timer))),
                );
            }
            if self.status.is_none()
                && let Some(message) = &self.hook_status_message
            {
                flex.push(
                    /*flex*/ 0,
                    RenderableItem::Owned(Box::new(hook_status::HookStatus {
                        message,
                        animations_enabled: self.animations_enabled,
                    })),
                );
            }
            // Avoid double-surfacing the same summary and avoid adding an extra
            // row while the status line is already visible.
            if self.status_widget().is_none() && !self.unified_exec_footer.is_empty() {
                flex.push(
                    /*flex*/ 0,
                    RenderableItem::Borrowed(&self.unified_exec_footer),
                );
            }
            let has_pending_thread_approvals = !self.pending_thread_approvals.is_empty();
            let has_questions = self
                .questions
                .as_ref()
                .is_some_and(|q| q.unanswered_count() > 0);
            let has_pending_input = has_questions
                || !self.pending_input_preview.queued_messages.is_empty()
                || !self.pending_input_preview.pending_steers.is_empty()
                || !self.pending_input_preview.rejected_steers.is_empty();
            let has_status_or_footer = self.status_widget().is_some()
                || self.hook_status_message.is_some()
                || !self.unified_exec_footer.is_empty();
            let has_inline_previews = has_pending_thread_approvals || has_pending_input;
            if has_inline_previews && has_status_or_footer {
                flex.push(/*flex*/ 0, RenderableItem::Owned("".into()));
            }
            flex.push(
                /*flex*/ 1,
                RenderableItem::Borrowed(&self.pending_thread_approvals),
            );
            if has_pending_thread_approvals && has_pending_input {
                flex.push(/*flex*/ 0, RenderableItem::Owned("".into()));
            }
            flex.push(
                /*flex*/ 1,
                if has_questions {
                    RenderableItem::Owned(Box::new(
                        pending_input_preview::PendingInputPreviewContent(
                            &self.pending_input_preview,
                        ),
                    ))
                } else {
                    RenderableItem::Borrowed(&self.pending_input_preview)
                },
            );
            let question_editor = self.questions.as_ref().filter(|q| q.expanded);
            if !has_inline_previews
                && has_status_or_footer
                && question_editor.is_none_or(|q| q.unanswered_count() > 1)
            {
                flex.push(/*flex*/ 0, RenderableItem::Owned("".into()));
            }
            if let Some(summary) = self.question_summary(Instant::now()) {
                flex.push(
                    /*flex*/ 0,
                    RenderableItem::Owned(Box::new(Paragraph::new(summary))),
                );
            }
            let mut flex2 = FlexRenderable::new();
            flex2.push(/*flex*/ 1, RenderableItem::Owned(flex.into()));
            let composer: RenderableItem<'_> = if let Some(questions) = question_editor {
                RenderableItem::Borrowed(questions.as_ref())
            } else if composer_right_reserve == 0 {
                RenderableItem::Borrowed(&self.composer)
            } else {
                RenderableItem::Owned(Box::new(ChatComposerRightReserveRenderable {
                    composer: &self.composer,
                    right_reserve: composer_right_reserve,
                }))
            };
            flex2.push(/*flex*/ 0, composer);
            RenderableItem::Owned(Box::new(flex2))
        }
    }

    pub(crate) fn set_status_line(&mut self, status_line: Option<Line<'static>>) {
        if self.composer.set_status_line(status_line) {
            self.request_redraw();
        }
    }

    pub(crate) fn set_luna_reserve_active(&mut self, active: bool) {
        if self.composer.set_luna_reserve_active(active) {
            self.request_redraw();
        }
    }

    pub(crate) fn set_status_line_hyperlink(&mut self, url: Option<String>) {
        if self.composer.set_status_line_hyperlink(url) {
            self.request_redraw();
        }
    }

    pub(crate) fn set_status_line_enabled(&mut self, enabled: bool) {
        if self.composer.set_status_line_enabled(enabled) {
            self.request_redraw();
        }
    }

    /// Updates the contextual footer label and requests a redraw only when it changed.
    ///
    /// This keeps the footer plumbing cheap during thread transitions where `App` may recompute
    /// the label several times while the visible thread settles.
    pub(crate) fn set_active_agent_label(&mut self, active_agent_label: Option<String>) {
        if self.composer.set_active_agent_label(active_agent_label) {
            self.request_redraw();
        }
    }

    pub(crate) fn set_side_conversation_context_label(&mut self, label: Option<String>) {
        if self.composer.set_side_conversation_context_label(label) {
            self.request_redraw();
        }
    }
}

struct ChatComposerRightReserveRenderable<'a> {
    composer: &'a chat_composer::ChatComposer,
    right_reserve: u16,
}

impl Renderable for ChatComposerRightReserveRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.composer.render_with_mask_and_textarea_right_reserve(
            area,
            buf,
            /*mask_char*/ None,
            self.right_reserve,
        );
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.composer
            .desired_height_with_textarea_right_reserve(width, self.right_reserve)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.composer
            .cursor_pos_with_textarea_right_reserve(area, self.right_reserve)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.composer.cursor_style(area)
    }
}

impl Renderable for BottomPane {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.as_renderable().render(area, buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.as_renderable().desired_height(width)
    }
    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.as_renderable().cursor_pos(area)
    }

    fn cursor_style(&self, area: Rect) -> crossterm::cursor::SetCursorStyle {
        self.as_renderable().cursor_style(area)
    }
}
