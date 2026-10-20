//! Status line configuration view for customizing the TUI status bar.
//!
//! This module provides an interactive picker for selecting which items appear
//! in the status line at the bottom of the terminal. Users can:
//!
//! - **Select items**: Toggle which information is displayed
//! - **Reorder items**: Use left/right arrows to change display order
//! - **Preview changes**: See a live preview of the configured status line
//!
//! # Available Status Line Items
//!
//! - Model information (name, reasoning level)
//! - Directory paths (current dir, project root)
//! - Machine hostname
//! - Git information (branch name)
//! - Permissions profile
//! - Approval mode
//! - Context usage (remaining %, used %, window size)
//! - Usage limits (primary, secondary)
//! - Session info (thread title, thread ID, tokens used)
//! - Application version

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::collections::HashSet;
use strum::IntoEnumIterator;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::CancellationEvent;
use crate::bottom_pane::bottom_pane_view::BottomPaneView;
use crate::bottom_pane::multi_select_picker::MultiSelectItem;
use crate::bottom_pane::multi_select_picker::MultiSelectPicker;
use crate::bottom_pane::status_surface_preview::StatusSurfacePreviewData;
use crate::bottom_pane::status_surface_preview::StatusSurfacePreviewItem;
use crate::keymap::ListKeymap;
use crate::render::renderable::Renderable;

const STATUS_LINE_USE_THEME_COLORS_ITEM_ID: &str = "status-line-use-theme-colors";

/// Available items that can be displayed in the status line.
///
/// Each variant represents a piece of information that can be shown at the
/// bottom of the TUI. Items are serialized to kebab-case for configuration
/// storage (e.g., `ModelWithReasoning` becomes `model-with-reasoning`).
///
/// Some items are conditionally displayed based on availability:
/// - Git-related items only show when in a git repository
/// - Context/limit items only show when data is available from the API
/// - Thread ID only shows after a session has started
#[derive(EnumIter, EnumString, Display, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[strum(serialize_all = "kebab_case")]
pub(crate) enum StatusLineItem {
    /// The current model name.
    #[strum(to_string = "model", serialize = "model-name")]
    ModelName,

    /// Model name with reasoning level suffix.
    ModelWithReasoning,

    /// Current reasoning level.
    Reasoning,

    /// Current working directory path.
    CurrentDir,

    /// Project root directory (if detected).
    #[strum(
        to_string = "project-name",
        serialize = "project",
        serialize = "project-root"
    )]
    ProjectRoot,

    /// Hostname of the machine running Ruxex.
    Hostname,

    /// Current git branch name (if in a repository).
    GitBranch,

    /// Open pull request number for the current branch.
    PullRequestNumber,

    /// Committed branch diff stats relative to the default branch.
    BranchChanges,

    /// Compact runtime run-state text.
    #[strum(to_string = "run-state", serialize = "status")]
    Status,

    /// Active permission profile or sandbox summary.
    Permissions,

    /// Active command approval mode.
    #[strum(to_string = "approval-mode", serialize = "approval")]
    ApprovalMode,

    /// Percentage of context window remaining.
    ContextRemaining,

    /// Percentage of context window used.
    ///
    /// Also accepts the legacy `context-usage` config value.
    #[strum(to_string = "context-used", serialize = "context-usage")]
    ContextUsed,

    /// Remaining usage on the primary rate limit.
    FiveHourLimit,

    /// Remaining usage on the secondary rate limit.
    WeeklyLimit,

    /// Ruxex application version.
    RuxexVersion,

    /// Total context window size in tokens.
    ContextWindowSize,

    /// Total tokens used in the current session.
    UsedTokens,

    /// Total input tokens consumed.
    TotalInputTokens,

    /// Total output tokens generated.
    TotalOutputTokens,

    /// Estimated credits attributed directly to the current enterprise thread.
    ThreadCredits,

    /// Estimated dollar cost attributed directly to the current enterprise thread.
    EstimatedThreadCost,

    /// Full thread UUID.
    #[strum(to_string = "thread-id", serialize = "session-id")]
    SessionId,

    /// Whether raw scrollback mode is currently active.
    RawOutput,

    /// Current thread name, omitted when unnamed.
    ThreadName,

    /// Current thread title, falling back to its identifier when unnamed.
    ThreadTitle,

    /// Latest checklist task progress from `update_plan` (if available).
    TaskProgress,
}

impl StatusLineItem {
    /// User-visible description shown in the popup.
    pub(crate) fn description(self) -> &'static str {
        match self {
            StatusLineItem::ModelName => "Current model name",
            StatusLineItem::ModelWithReasoning => "Current model name with reasoning level",
            StatusLineItem::Reasoning => "Current reasoning level",
            StatusLineItem::CurrentDir => "Current working directory",
            StatusLineItem::ProjectRoot => "Project name (omitted when unavailable)",
            StatusLineItem::Hostname => "Current machine hostname (omitted when unavailable)",
            StatusLineItem::GitBranch => "Current Git branch (omitted when unavailable)",
            StatusLineItem::PullRequestNumber => {
                "Open pull request number for the current branch (omitted when unavailable)"
            }
            StatusLineItem::BranchChanges => {
                "Committed branch changes against the default branch (omitted when unavailable)"
            }
            StatusLineItem::Status => "Compact session run-state text (Ready, Working, Thinking)",
            StatusLineItem::Permissions => "Active permission profile or sandbox mode",
            StatusLineItem::ApprovalMode => "Active command approval mode",
            StatusLineItem::ContextRemaining => {
                "Percentage of context window remaining (omitted when unknown)"
            }
            StatusLineItem::ContextUsed => {
                "Percentage of context window used (omitted when unknown)"
            }
            StatusLineItem::FiveHourLimit => {
                "Remaining usage on the primary usage limit (omitted when unavailable)"
            }
            StatusLineItem::WeeklyLimit => {
                "Remaining usage on the secondary usage limit (omitted when unavailable)"
            }
            StatusLineItem::RuxexVersion => "Ruxex application version",
            StatusLineItem::ContextWindowSize => {
                "Total context window size in tokens (omitted when unknown)"
            }
            StatusLineItem::UsedTokens => "Total tokens used in session (omitted when zero)",
            StatusLineItem::TotalInputTokens => "Total input tokens used in session",
            StatusLineItem::TotalOutputTokens => "Total output tokens used in session",
            StatusLineItem::ThreadCredits => {
                "Estimated current-thread credits (Enterprise workspaces only; omitted when unavailable)"
            }
            StatusLineItem::EstimatedThreadCost => {
                "Estimated current-thread cost in USD (Enterprise workspaces only; omitted when unavailable)"
            }
            StatusLineItem::SessionId => "Current thread identifier (omitted until thread starts)",
            StatusLineItem::RawOutput => "Whether raw scrollback mode is active",
            StatusLineItem::ThreadName => "Current thread name (omitted when unnamed)",
            StatusLineItem::ThreadTitle => {
                "Current thread title, or thread identifier when unnamed"
            }
            StatusLineItem::TaskProgress => {
                "Latest task progress from update_plan (omitted until available)"
            }
        }
    }

    pub(crate) fn preview_item(self) -> StatusSurfacePreviewItem {
        match self {
            StatusLineItem::ModelName => StatusSurfacePreviewItem::Model,
            StatusLineItem::ModelWithReasoning => StatusSurfacePreviewItem::ModelWithReasoning,
            StatusLineItem::Reasoning => StatusSurfacePreviewItem::Reasoning,
            StatusLineItem::CurrentDir => StatusSurfacePreviewItem::CurrentDir,
            StatusLineItem::ProjectRoot => StatusSurfacePreviewItem::ProjectRoot,
            StatusLineItem::Hostname => StatusSurfacePreviewItem::Hostname,
            StatusLineItem::GitBranch => StatusSurfacePreviewItem::GitBranch,
            StatusLineItem::PullRequestNumber => StatusSurfacePreviewItem::PullRequestNumber,
            StatusLineItem::BranchChanges => StatusSurfacePreviewItem::BranchChanges,
            StatusLineItem::Status => StatusSurfacePreviewItem::Status,
            StatusLineItem::Permissions => StatusSurfacePreviewItem::Permissions,
            StatusLineItem::ApprovalMode => StatusSurfacePreviewItem::ApprovalMode,
            StatusLineItem::ContextRemaining => StatusSurfacePreviewItem::ContextRemaining,
            StatusLineItem::ContextUsed => StatusSurfacePreviewItem::ContextUsed,
            StatusLineItem::FiveHourLimit => StatusSurfacePreviewItem::FiveHourLimit,
            StatusLineItem::WeeklyLimit => StatusSurfacePreviewItem::WeeklyLimit,
            StatusLineItem::RuxexVersion => StatusSurfacePreviewItem::RuxexVersion,
            StatusLineItem::ContextWindowSize => StatusSurfacePreviewItem::ContextWindowSize,
            StatusLineItem::UsedTokens => StatusSurfacePreviewItem::UsedTokens,
            StatusLineItem::TotalInputTokens => StatusSurfacePreviewItem::TotalInputTokens,
            StatusLineItem::TotalOutputTokens => StatusSurfacePreviewItem::TotalOutputTokens,
            StatusLineItem::ThreadCredits => StatusSurfacePreviewItem::ThreadCredits,
            StatusLineItem::EstimatedThreadCost => StatusSurfacePreviewItem::EstimatedThreadCost,
            StatusLineItem::SessionId => StatusSurfacePreviewItem::SessionId,
            StatusLineItem::RawOutput => StatusSurfacePreviewItem::RawOutput,
            StatusLineItem::ThreadName => StatusSurfacePreviewItem::ThreadName,
            StatusLineItem::ThreadTitle => StatusSurfacePreviewItem::ThreadTitle,
            StatusLineItem::TaskProgress => StatusSurfacePreviewItem::TaskProgress,
        }
    }
}

/// Interactive view for configuring which items appear in the status line.
///
/// Wraps a [`MultiSelectPicker`] with status-line-specific behavior:
/// - Pre-populates items from current configuration
/// - Shows a live preview of the configured status line
/// - Emits [`AppEvent::StatusLineSetup`] on confirmation
/// - Emits [`AppEvent::StatusLineSetupCancelled`] on cancellation
pub(crate) struct StatusLineSetupView {
    /// The underlying multi-select picker widget.
    picker: MultiSelectPicker,
}

impl StatusLineSetupView {
    /// Creates a new status line setup view.
    ///
    /// # Arguments
    ///
    /// * `status_line_items` - Currently configured item IDs (in display order),
    ///   or `None` to start with all items disabled
    /// * `use_theme_colors` - Whether the preview and saved status line use colors from
    ///   the active theme
    /// * `app_event_tx` - Event sender for dispatching configuration changes
    ///
    /// Items from `status_line_items` are shown first (in order) and marked as
    /// enabled. Remaining items are appended and marked as disabled.
    pub(crate) fn new(
        status_line_items: Option<&[String]>,
        use_theme_colors: bool,
        preview_data: StatusSurfacePreviewData,
        app_event_tx: AppEventSender,
        list_keymap: ListKeymap,
    ) -> Self {
        let mut used_ids = HashSet::new();
        let mut items = vec![MultiSelectItem {
            id: STATUS_LINE_USE_THEME_COLORS_ITEM_ID.to_string(),
            name: "Use theme colors".to_string(),
            description: Some("Apply colors from the active /theme".to_string()),
            enabled: use_theme_colors,
            orderable: false,
            section_break_after: true,
        }];

        if let Some(selected_items) = status_line_items.as_ref() {
            for id in *selected_items {
                let Ok(item) = id.parse::<StatusLineItem>() else {
                    continue;
                };
                let item_id = item.to_string();
                if !used_ids.insert(item_id.clone()) {
                    continue;
                }
                items.push(Self::status_line_select_item(
                    item,
                    /*enabled*/ true,
                    &preview_data,
                ));
            }
        }

        for item in StatusLineItem::iter() {
            let item_id = item.to_string();
            if used_ids.contains(&item_id) {
                continue;
            }
            items.push(Self::status_line_select_item(
                item,
                /*enabled*/ false,
                &preview_data,
            ));
        }

        Self {
            picker: MultiSelectPicker::builder(
                "Configure Status Line".to_string(),
                Some("Select which items to display in the status line.".to_string()),
                app_event_tx,
            )
            .list_keymap(list_keymap)
            .items(items)
            .enable_ordering()
            .on_preview(move |items| {
                let use_theme_colors = items
                    .iter()
                    .find(|item| item.id == STATUS_LINE_USE_THEME_COLORS_ITEM_ID)
                    .map(|item| item.enabled)
                    .unwrap_or(true);
                preview_data.status_line_for_items(
                    items
                        .iter()
                        .filter(|item| item.enabled)
                        .filter_map(|item| item.id.parse::<StatusLineItem>().ok()),
                    use_theme_colors,
                )
            })
            .on_confirm(|ids, app_event| {
                let use_theme_colors = ids
                    .iter()
                    .any(|id| id == STATUS_LINE_USE_THEME_COLORS_ITEM_ID);
                let items = ids
                    .iter()
                    .filter_map(|id| id.parse::<StatusLineItem>().ok())
                    .collect::<Vec<_>>();
                app_event.send(AppEvent::StatusLineSetup {
                    items,
                    use_theme_colors,
                });
            })
            .on_cancel(|app_event| {
                app_event.send(AppEvent::StatusLineSetupCancelled);
            })
            .build(),
        }
    }

    /// Converts a [`StatusLineItem`] into a [`MultiSelectItem`] for the picker.
    fn status_line_select_item(
        item: StatusLineItem,
        enabled: bool,
        preview_data: &StatusSurfacePreviewData,
    ) -> MultiSelectItem {
        let default_name = item.to_string();
        let default_description = item.description();
        let (name, description) = match item {
            StatusLineItem::FiveHourLimit | StatusLineItem::WeeklyLimit => (
                preview_data.rate_limit_item_name(item.preview_item(), &default_name),
                preview_data.rate_limit_item_description(item.preview_item(), default_description),
            ),
            _ => (default_name, default_description.to_string()),
        };

        MultiSelectItem {
            id: item.to_string(),
            name,
            description: Some(description),
            enabled,
            orderable: true,
            section_break_after: false,
        }
    }
}

impl BottomPaneView for StatusLineSetupView {
    fn keymap_contexts(&self) -> crate::keymap::KeymapContextSet {
        crate::keymap::KeymapContextSet::new(crate::keymap::KeymapContext::List)
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {
        self.picker.handle_key_event(key_event);
    }

    fn is_complete(&self) -> bool {
        self.picker.complete
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.picker.close();
        CancellationEvent::Handled
    }
}

impl Renderable for StatusLineSetupView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.picker.render(area, buf)
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.picker.desired_height(width)
    }
}
