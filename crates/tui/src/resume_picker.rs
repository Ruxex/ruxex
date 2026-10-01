use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use crate::app_server_session::AppServerSession;
use crate::clipboard_paste::normalize_pasted_search_query;
use crate::color::blend;
use crate::color::is_light;
use crate::key_hint::KeyBindingListExt;
use crate::key_hint::is_plain_text_key_event;
use crate::keymap::ListAction;
use crate::keymap::ListKeymap;
use crate::keymap::PagerKeymap;
use crate::keymap::RuntimeChordKeymap;
use crate::keymap::RuntimeKeymap;
use crate::legacy_core::config::Config;
use crate::legacy_core::config::edit::ConfigEditsBuilder;
use crate::markdown_render::render_streaming_markdown_lines_with_width_and_cwd as render_assistant;
use crate::pager_overlay::Overlay;
use crate::status::format_directory_display;
use crate::style::footer_hint_key_style;
use crate::style::footer_hint_label_style;
use crate::terminal_palette::best_color;
use crate::terminal_palette::default_bg;
use crate::text_formatting::truncate_text;
use crate::thread_transcript::RawReasoningVisibility;
use crate::thread_transcript::TranscriptCells;
use crate::thread_transcript::load_session_transcript;
use crate::tui::FrameRequester;
use crate::tui::Tui;
use crate::tui::TuiEvent;
use crate::wrapping::RtOptions;
use crate::wrapping::adaptive_wrap_lines;
use chrono::DateTime;
use chrono::Utc;
use genex_app_server_client::AppServerRequestHandle;
use genex_app_server_protocol::ClientRequest;
use genex_app_server_protocol::RequestId;
use genex_app_server_protocol::Thread;
use genex_app_server_protocol::ThreadArchiveParams;
use genex_app_server_protocol::ThreadArchiveResponse;
use genex_app_server_protocol::ThreadHistoryMode;
use genex_app_server_protocol::ThreadListCwdFilter;
use genex_app_server_protocol::ThreadListParams;
use genex_app_server_protocol::ThreadSortKey;
use genex_app_server_protocol::ThreadUnarchiveParams;
use genex_app_server_protocol::ThreadUnarchiveResponse;
use genex_config::types::SessionPickerViewMode;
use genex_protocol::ThreadId;
use genex_utils_path as path_utils;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Styled as _;
use ratatui::style::Stylize as _;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::widgets::Widget;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::warn;
use unicode_width::UnicodeWidthStr;
use uuid::Uuid;

mod archive;
mod page_loading;

use page_loading::PageCwdFilter;
use page_loading::PageLoadMode;
use page_loading::PaginationState;

#[path = "resume_picker_transcript_preview.rs"]
mod transcript_preview;

pub(crate) use transcript_preview::load_transcript_preview;

const PAGE_SIZE: usize = 25;
const LOAD_NEAR_THRESHOLD: usize = 5;
const SESSION_META_INDENT_WIDTH: usize = 2;
const SESSION_META_DATE_WIDTH: usize = 12;
const SESSION_META_FIELD_GAP_WIDTH: usize = 2;
const SESSION_META_MIN_CWD_WIDTH: usize = 30;
const SESSION_META_MAX_CWD_WIDTH: usize = 72;
const SESSION_META_BRANCH_ICON: &str = "";
const SESSION_META_CWD_ICON: &str = "⌁";
const FOOTER_COMPACT_BREAKPOINT: u16 = 120;
const FOOTER_HINT_LEFT_PADDING: usize = 1;
const FOOTER_HINT_GAP: usize = 3;
const PICKER_CHROME_HEIGHT: u16 = 8;
const PICKER_LIST_HORIZONTAL_INSET: u16 = 4;

#[derive(Debug, Clone)]
pub struct SessionTarget {
    pub path: Option<PathBuf>,
    pub thread_id: ThreadId,
    /// Working directory reported by `thread/list` or `thread/read` at selection time.
    pub cwd: Option<PathBuf>,
    /// History mode observed during selection, if the server provided one.
    pub history_mode: Option<ThreadHistoryMode>,
}

impl SessionTarget {
    pub fn display_label(&self) -> String {
        self.path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| format!("thread {}", self.thread_id))
    }
}

#[derive(Debug, Clone)]
pub enum SessionSelection {
    StartFresh,
    AgentsOverview,
    Resume(SessionTarget),
    Fork(SessionTarget),
    Exit,
}

#[derive(Clone, Copy, Debug)]
pub enum SessionPickerAction {
    Resume,
    Fork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPickerLaunchContext {
    Startup,
    ExistingSession { current_thread_id: Option<ThreadId> },
}

impl SessionPickerAction {
    fn title(self) -> &'static str {
        match self {
            SessionPickerAction::Resume => "Resume a previous session",
            SessionPickerAction::Fork => "Fork a previous session",
        }
    }

    fn action_label(self) -> &'static str {
        match self {
            SessionPickerAction::Resume => "resume",
            SessionPickerAction::Fork => "fork",
        }
    }

    fn selection(self, target_session: SessionTarget) -> SessionSelection {
        match self {
            SessionPickerAction::Resume => SessionSelection::Resume(target_session),
            SessionPickerAction::Fork => SessionSelection::Fork(target_session),
        }
    }
}

#[derive(Clone)]
struct PageLoadRequest {
    cursor: Option<PageCursor>,
    request_token: usize,
    search_token: Option<usize>,
    mode: PageLoadMode,
    cwd_filter: Option<PathBuf>,
    status: SessionStatus,
    provider_filter: ProviderFilter,
    sort_key: ThreadSortKey,
}

enum PickerLoadRequest {
    Page(PageLoadRequest),
    Preview {
        thread_id: ThreadId,
    },
    Transcript {
        thread_id: ThreadId,
        cancellation: oneshot::Receiver<()>,
    },
    Archive {
        thread_id: ThreadId,
    },
    Unarchive {
        thread_id: ThreadId,
    },
}

#[derive(Clone)]
enum ProviderFilter {
    Any,
    MatchDefault(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionFilterMode {
    Cwd,
    All,
}

impl SessionFilterMode {
    fn from_show_all(show_all: bool, filter_cwd: Option<&Path>) -> Self {
        if show_all || filter_cwd.is_none() {
            Self::All
        } else {
            Self::Cwd
        }
    }

    fn toggle(self, filter_cwd: Option<&Path>) -> Self {
        match self {
            Self::Cwd => Self::All,
            Self::All if filter_cwd.is_some() => Self::Cwd,
            Self::All => Self::All,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionStatus {
    Active,
    Archived,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarControl {
    Filter,
    Status,
    Sort,
}

impl ToolbarControl {
    fn previous(self, action: SessionPickerAction) -> Self {
        match self {
            Self::Filter => Self::Sort,
            Self::Status => Self::Filter,
            Self::Sort if matches!(action, SessionPickerAction::Resume) => Self::Status,
            Self::Sort => Self::Filter,
        }
    }

    fn next(self, action: SessionPickerAction) -> Self {
        match self {
            Self::Filter if matches!(action, SessionPickerAction::Resume) => Self::Status,
            Self::Filter | Self::Status => Self::Sort,
            Self::Sort => Self::Filter,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionListDensity {
    Comfortable,
    Dense,
}

impl SessionListDensity {
    fn toggle(self) -> Self {
        match self {
            Self::Comfortable => Self::Dense,
            Self::Dense => Self::Comfortable,
        }
    }
}

impl From<SessionPickerViewMode> for SessionListDensity {
    fn from(mode: SessionPickerViewMode) -> Self {
        match mode {
            SessionPickerViewMode::Comfortable => Self::Comfortable,
            SessionPickerViewMode::Dense => Self::Dense,
        }
    }
}

impl From<SessionListDensity> for SessionPickerViewMode {
    fn from(density: SessionListDensity) -> Self {
        match density {
            SessionListDensity::Comfortable => Self::Comfortable,
            SessionListDensity::Dense => Self::Dense,
        }
    }
}

type PickerLoader = Arc<dyn Fn(PickerLoadRequest) + Send + Sync>;
enum BackgroundEvent {
    Page {
        request_token: usize,
        search_token: Option<usize>,
        page: std::io::Result<PickerPage>,
    },
    Preview {
        thread_id: ThreadId,
        preview: std::io::Result<Vec<TranscriptPreviewLine>>,
    },
    Transcript {
        thread_id: ThreadId,
        transcript: std::io::Result<TranscriptCells>,
    },
    Archive {
        thread_id: ThreadId,
        result: std::io::Result<()>,
    },
    Unarchive {
        thread_id: ThreadId,
        result: std::io::Result<SessionTarget>,
    },
}

#[derive(Clone)]
enum PageCursor {
    AppServer(String),
}

struct PickerPage {
    rows: Vec<Row>,
    history_modes: HashMap<ThreadId, ThreadHistoryMode>,
    next_cursor: Option<PageCursor>,
    num_scanned_files: usize,
    reached_scan_cap: bool,
}

#[derive(Clone)]
struct SessionPickerViewPersistence {
    codex_home: PathBuf,
}

struct SessionPickerRunOptions {
    use_theme_colors: bool,
    show_all: bool,
    filter_cwd: Option<PathBuf>,
    local_filter_cwd: Option<PathBuf>,
    worktrees_enabled: bool,
    action: SessionPickerAction,
    launch_context: SessionPickerLaunchContext,
    provider_filter: ProviderFilter,
    initial_density: SessionListDensity,
    view_persistence: Option<SessionPickerViewPersistence>,
    pager_keymap: PagerKeymap,
    list_keymap: ListKeymap,
    initial_page_mode: PageLoadMode,
    chord_keymap: Arc<RuntimeChordKeymap>,
}

/// Interactive session picker that lists app-server threads with simple search,
/// lazy transcript previews, and pagination.
///
/// Sessions render as compact multi-line records with stable metadata first and
/// the conversation preview last. Users can focus the toolbar controls with
/// Tab, change the focused control with the arrow keys, and expand the
/// selected session with Ctrl+E to load recent transcript context on demand.
///
/// Sessions are loaded on-demand via cursor-based pagination. The backend
/// `thread/list` API returns pages ordered by the selected sort key, and the
/// picker deduplicates across pages to handle overlapping windows when new
/// sessions appear during pagination.
///
/// Filtering happens in two layers:
/// 1. Provider, source, and eligible working-directory filtering at the backend.
/// 2. Typed search filtering over loaded rows in the picker.
pub async fn run_resume_picker_with_app_server(
    uses_remote_filesystem: bool,
    tui: &mut Tui,
    config: &Config,
    local_settings: &crate::local_settings::LocalSettings,
    show_all: bool,
    include_non_interactive: bool,
    app_server: AppServerSession,
) -> Result<SessionSelection> {
    let archive_request_handle = app_server.request_handle();
    run_resume_picker_with_launch_context(
        uses_remote_filesystem,
        tui,
        config,
        local_settings,
        show_all,
        include_non_interactive,
        app_server,
        archive_request_handle,
        SessionPickerLaunchContext::Startup,
    )
    .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "keep local preferences separate while the legacy Config parameter is still required"
)]
pub async fn run_resume_picker_from_existing_session_with_app_server(
    uses_remote_filesystem: bool,
    tui: &mut Tui,
    config: &Config,
    local_settings: &crate::local_settings::LocalSettings,
    show_all: bool,
    include_non_interactive: bool,
    app_server: AppServerSession,
    archive_request_handle: AppServerRequestHandle,
    current_thread_id: Option<ThreadId>,
) -> Result<SessionSelection> {
    run_resume_picker_with_launch_context(
        uses_remote_filesystem,
        tui,
        config,
        local_settings,
        show_all,
        include_non_interactive,
        app_server,
        archive_request_handle,
        SessionPickerLaunchContext::ExistingSession { current_thread_id },
    )
    .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "keep local preferences separate while the legacy Config parameter is still required"
)]
async fn run_resume_picker_with_launch_context(
    uses_remote_filesystem: bool,
    tui: &mut Tui,
    config: &Config,
    local_settings: &crate::local_settings::LocalSettings,
    show_all: bool,
    include_non_interactive: bool,
    app_server: AppServerSession,
    archive_request_handle: AppServerRequestHandle,
    launch_context: SessionPickerLaunchContext,
) -> Result<SessionSelection> {
    let (bg_tx, bg_rx) = mpsc::unbounded_channel();
    let uses_remote_workspace = app_server.uses_remote_workspace();
    let cwd_filter = picker_cwd_filter(
        config.cwd.as_path(),
        /*show_all*/ false,
        uses_remote_workspace,
        app_server.remote_cwd_override(),
    );
    let local_filter_cwd = local_picker_cwd_filter(&cwd_filter, uses_remote_filesystem);
    let provider_filter = picker_provider_filter(config, uses_remote_workspace);
    let runtime_keymap = picker_runtime_keymap(local_settings)?;
    let options = SessionPickerRunOptions {
        use_theme_colors: local_settings.tui.status_line_use_colors,
        show_all,
        filter_cwd: cwd_filter,
        local_filter_cwd,
        worktrees_enabled: config.features.enabled(genex_features::Feature::Worktrees),
        action: SessionPickerAction::Resume,
        launch_context,
        provider_filter,
        initial_density: SessionListDensity::from(
            local_settings.tui.session_picker_view.unwrap_or_default(),
        ),
        view_persistence: Some(SessionPickerViewPersistence {
            codex_home: local_settings.codex_home.to_path_buf(),
        }),
        pager_keymap: runtime_keymap.pager,
        list_keymap: runtime_keymap.list,
        initial_page_mode: if uses_remote_workspace {
            PageLoadMode::StoreDefault
        } else {
            PageLoadMode::StateDbOnly
        },
        chord_keymap: runtime_keymap.chords,
    };
    run_session_picker_with_loader(
        tui,
        options,
        spawn_app_server_page_loader(
            uses_remote_filesystem,
            app_server,
            archive_request_handle,
            include_non_interactive,
            raw_reasoning_visibility(config),
            (!uses_remote_workspace).then(|| config.clone()),
            bg_tx,
        ),
        bg_rx,
    )
    .await
}

pub async fn run_fork_picker_with_app_server(
    uses_remote_filesystem: bool,
    tui: &mut Tui,
    config: &Config,
    local_settings: &crate::local_settings::LocalSettings,
    show_all: bool,
    app_server: AppServerSession,
) -> Result<SessionSelection> {
    let archive_request_handle = app_server.request_handle();
    let (bg_tx, bg_rx) = mpsc::unbounded_channel();
    let uses_remote_workspace = app_server.uses_remote_workspace();
    let cwd_filter = picker_cwd_filter(
        config.cwd.as_path(),
        /*show_all*/ false,
        uses_remote_workspace,
        app_server.remote_cwd_override(),
    );
    let local_filter_cwd = local_picker_cwd_filter(&cwd_filter, uses_remote_filesystem);
    let provider_filter = picker_provider_filter(config, uses_remote_workspace);
    let runtime_keymap = picker_runtime_keymap(local_settings)?;
    let options = SessionPickerRunOptions {
        use_theme_colors: local_settings.tui.status_line_use_colors,
        show_all,
        filter_cwd: cwd_filter,
        local_filter_cwd,
        worktrees_enabled: config.features.enabled(genex_features::Feature::Worktrees),
        action: SessionPickerAction::Fork,
        launch_context: SessionPickerLaunchContext::Startup,
        provider_filter,
        initial_density: SessionListDensity::from(
            local_settings.tui.session_picker_view.unwrap_or_default(),
        ),
        view_persistence: Some(SessionPickerViewPersistence {
            codex_home: local_settings.codex_home.to_path_buf(),
        }),
        pager_keymap: runtime_keymap.pager,
        list_keymap: runtime_keymap.list,
        initial_page_mode: if uses_remote_workspace {
            PageLoadMode::StoreDefault
        } else {
            PageLoadMode::StateDbOnly
        },
        chord_keymap: runtime_keymap.chords,
    };
    run_session_picker_with_loader(
        tui,
        options,
        spawn_app_server_page_loader(
            uses_remote_filesystem,
            app_server,
            archive_request_handle,
            /*include_non_interactive*/ false,
            raw_reasoning_visibility(config),
            (!uses_remote_workspace).then(|| config.clone()),
            bg_tx,
        ),
        bg_rx,
    )
    .await
}

async fn run_session_picker_with_loader(
    tui: &mut Tui,
    options: SessionPickerRunOptions,
    picker_loader: PickerLoader,
    bg_rx: mpsc::UnboundedReceiver<BackgroundEvent>,
) -> Result<SessionSelection> {
    let alt = AltScreenGuard::enter(tui);
    let mut state = PickerState::new(
        alt.tui.frame_requester(),
        picker_loader,
        options.provider_filter,
        options.show_all,
        options.filter_cwd,
        options.action,
    );
    state.local_filter_cwd = options.local_filter_cwd;
    state.use_theme_colors = options.use_theme_colors;
    state.worktrees_enabled = options.worktrees_enabled;
    state.density = options.initial_density;
    state.view_persistence = options.view_persistence;
    state.pager_keymap = options.pager_keymap;
    state.list_keymap = options.list_keymap;
    state.chord_keymap = options.chord_keymap;
    state.launch_context = options.launch_context;
    state.initial_page_mode = options.initial_page_mode;
    state.start_initial_load();
    state.request_frame();

    if let Ok(size) = alt.tui.terminal.size() {
        let list_height = size.height.saturating_sub(PICKER_CHROME_HEIGHT) as usize;
        state.update_viewport(list_height, list_viewport_width(size.width));
        state.ensure_minimum_rows_for_view(list_height);
    }
    draw_picker(alt.tui, &state, alt.tui.terminal.last_known_screen_size)?;
    if state.launch_context == SessionPickerLaunchContext::Startup {
        alt.tui.discard_pending_input_before_interactive_screen()?;
    }
    let mut tui_events = alt.tui.event_stream().fuse();
    let mut background_events = UnboundedReceiverStream::new(bg_rx).fuse();

    loop {
        tokio::select! {
            Some(ev) = tui_events.next() => {
                let screen_size = alt.tui.screen_size_for_event(&ev)?;
                let ev = if let TuiEvent::Key(key) = ev {
                    let Some(key) = state.route_key_chord(key) else {
                        continue;
                    };
                    TuiEvent::Key(key)
                } else {
                    ev
                };
                if state.overlay.is_some() {
                    state.handle_overlay_event(alt.tui, ev)?;
                    continue;
                }
                match ev {
                    TuiEvent::Key(key) => {
                        if matches!(key.kind, KeyEventKind::Release) {
                            continue;
                        }
                        if let Some(sel) = state.handle_key(key).await? {
                            return Ok(sel);
                        }
                    }
                    TuiEvent::Paste(pasted) => {
                        state.handle_paste(pasted);
                    }
                    TuiEvent::Draw | TuiEvent::Resume | TuiEvent::Resize(_) | TuiEvent::FocusGained => {
                        let list_width = list_viewport_width(screen_size.width);
                        let list_height =
                            usize::from(screen_size.height.saturating_sub(PICKER_CHROME_HEIGHT));
                        state.update_viewport(list_height, list_width);
                        state.ensure_minimum_rows_for_view(list_height);
                        draw_picker(alt.tui, &state, screen_size)?;
                        if state.note_transcript_loading_frame_drawn() {
                            state.open_pending_transcript_if_ready();
                        }
                    }
                    TuiEvent::FocusLost => {}
                }
            }
            Some(event) = background_events.next() => {
                if let Some(selection) = state.handle_background_event(event).await? {
                    return Ok(selection);
                }
            }
            else => break,
        }
    }

    // Fallback – treat as cancel/new
    Ok(SessionSelection::StartFresh)
}

fn raw_reasoning_visibility(config: &Config) -> RawReasoningVisibility {
    if config.show_raw_agent_reasoning {
        RawReasoningVisibility::Visible
    } else {
        RawReasoningVisibility::Hidden
    }
}

fn local_picker_cwd_filter(
    cwd_filter: &Option<PathBuf>,
    uses_remote_filesystem: bool,
) -> Option<PathBuf> {
    if uses_remote_filesystem {
        None
    } else {
        cwd_filter.clone()
    }
}

fn picker_provider_filter(config: &Config, uses_remote_workspace: bool) -> ProviderFilter {
    if uses_remote_workspace {
        ProviderFilter::Any
    } else {
        ProviderFilter::MatchDefault(config.model_provider_id.to_string())
    }
}

fn picker_runtime_keymap(config: &crate::local_settings::LocalSettings) -> Result<RuntimeKeymap> {
    RuntimeKeymap::from_config(&config.tui.keymap)
        .map_err(|err| color_eyre::eyre::eyre!("invalid keymap configuration: {err}"))
}

fn picker_cwd_filter(
    config_cwd: &Path,
    show_all: bool,
    uses_remote_workspace: bool,
    remote_cwd_override: Option<&Path>,
) -> Option<PathBuf> {
    if show_all {
        None
    } else if uses_remote_workspace {
        remote_cwd_override.map(Path::to_path_buf)
    } else {
        Some(config_cwd.to_path_buf())
    }
}

fn spawn_app_server_page_loader(
    uses_remote_filesystem: bool,
    app_server: AppServerSession,
    archive_request_handle: AppServerRequestHandle,
    include_non_interactive: bool,
    raw_reasoning_visibility: RawReasoningVisibility,
    config: Option<Config>,
    bg_tx: mpsc::UnboundedSender<BackgroundEvent>,
) -> PickerLoader {
    let (request_tx, mut request_rx) = mpsc::unbounded_channel::<PickerLoadRequest>();

    tokio::spawn(async move {
        let mut app_server = app_server;
        let mut page_cwd_filter = PageCwdFilter::default();
        while let Some(request) = request_rx.recv().await {
            match request {
                PickerLoadRequest::Page(request) => {
                    let cwd_filter = page_cwd_filter.for_request(
                        request.cursor.as_ref(),
                        request.cwd_filter.as_deref(),
                        uses_remote_filesystem,
                        config.as_ref().is_some_and(|config| {
                            config.features.enabled(genex_features::Feature::Worktrees)
                        }),
                    );
                    let cursor = request.cursor.map(|PageCursor::AppServer(cursor)| cursor);
                    let params = thread_list_params(
                        cursor,
                        cwd_filter,
                        request.status,
                        request.provider_filter,
                        request.sort_key,
                        include_non_interactive,
                        matches!(request.mode, PageLoadMode::StateDbOnly),
                    );
                    let page = load_app_server_page(&mut app_server, params).await;
                    let _ = bg_tx.send(BackgroundEvent::Page {
                        request_token: request.request_token,
                        search_token: request.search_token,
                        page,
                    });
                }
                PickerLoadRequest::Preview { thread_id } => {
                    let preview =
                        load_transcript_preview(&mut app_server, thread_id, config.as_ref()).await;
                    let _ = bg_tx.send(BackgroundEvent::Preview { thread_id, preview });
                }
                PickerLoadRequest::Transcript {
                    thread_id,
                    cancellation,
                } => {
                    tokio::select! {
                        transcript = load_session_transcript(
                            &mut app_server,
                            thread_id,
                            raw_reasoning_visibility,
                            config.as_ref(),
                        ) => {
                            let _ = bg_tx.send(BackgroundEvent::Transcript {
                                thread_id,
                                transcript,
                            });
                        }
                        _ = cancellation => {}
                    }
                }
                PickerLoadRequest::Archive { thread_id } => {
                    let result = archive_request_handle
                        .request_typed::<ThreadArchiveResponse>(ClientRequest::ThreadArchive {
                            request_id: RequestId::String(format!(
                                "resume-picker-archive-{}",
                                Uuid::new_v4()
                            )),
                            params: ThreadArchiveParams {
                                thread_id: thread_id.to_string(),
                            },
                        })
                        .await
                        .map(|_| ())
                        .map_err(std::io::Error::other);
                    let _ = bg_tx.send(BackgroundEvent::Archive { thread_id, result });
                }
                PickerLoadRequest::Unarchive { thread_id } => {
                    let result = archive_request_handle
                        .request_typed::<ThreadUnarchiveResponse>(ClientRequest::ThreadUnarchive {
                            request_id: RequestId::String(format!(
                                "resume-picker-unarchive-{}",
                                Uuid::new_v4()
                            )),
                            params: ThreadUnarchiveParams {
                                thread_id: thread_id.to_string(),
                            },
                        })
                        .await
                        .map(|response| SessionTarget {
                            path: response.thread.path,
                            thread_id,
                            cwd: Some(response.thread.cwd.to_path_buf()),
                            history_mode: Some(response.thread.history_mode),
                        })
                        .map_err(std::io::Error::other);
                    let _ = bg_tx.send(BackgroundEvent::Unarchive { thread_id, result });
                }
            }
        }
        if let Err(err) = app_server.shutdown().await {
            warn!(%err, "Failed to shut down app-server picker session");
        }
    });

    Arc::new(move |request: PickerLoadRequest| {
        let _ = request_tx.send(request);
    })
}

/// Returns the human-readable column header for the given sort key.
fn sort_key_label(sort_key: ThreadSortKey) -> &'static str {
    match sort_key {
        ThreadSortKey::CreatedAt => "Created",
        ThreadSortKey::UpdatedAt | ThreadSortKey::RecencyAt | ThreadSortKey::SectionPosition => {
            "Updated"
        }
    }
}

/// RAII guard that ensures we leave the alt-screen on scope exit.
struct AltScreenGuard<'a> {
    tui: &'a mut Tui,
}

impl<'a> AltScreenGuard<'a> {
    fn enter(tui: &'a mut Tui) -> Self {
        let _ = tui.enter_alt_screen();
        Self { tui }
    }
}

impl Drop for AltScreenGuard<'_> {
    fn drop(&mut self) {
        let _ = self.tui.leave_alt_screen();
    }
}

struct PickerState {
    use_theme_colors: bool,
    // Resolve local filesystem membership once per cwd for each page-loading cycle.
    local_cwd_matches: HashMap<PathBuf, bool>,
    requester: FrameRequester,
    relative_time_reference: Option<DateTime<Utc>>,
    pagination: PaginationState,
    all_rows: Vec<Row>,
    filtered_rows: Vec<Row>,
    thread_history_modes: HashMap<ThreadId, ThreadHistoryMode>,
    seen_rows: HashSet<SeenRowKey>,
    selected: usize,
    scroll_top: usize,
    pending_page_down_target: Option<usize>,
    frozen_footer_percent: Option<u8>,
    query: String,
    search_state: SearchState,
    next_request_token: usize,
    next_search_token: usize,
    picker_loader: PickerLoader,
    view_rows: Option<usize>,
    view_width: Option<u16>,
    provider_filter: ProviderFilter,
    filter_mode: SessionFilterMode,
    status: SessionStatus,
    filter_cwd: Option<PathBuf>,
    local_filter_cwd: Option<PathBuf>,
    worktrees_enabled: bool,
    toolbar_focus: ToolbarControl,
    density: SessionListDensity,
    launch_context: SessionPickerLaunchContext,
    view_persistence: Option<SessionPickerViewPersistence>,
    action: SessionPickerAction,
    sort_key: ThreadSortKey,
    inline_error: Option<String>,
    archive_state: archive::ArchiveState,
    expanded_thread_id: Option<ThreadId>,
    transcript_previews: HashMap<ThreadId, TranscriptPreviewState>,
    transcript_cells: HashMap<ThreadId, SessionTranscriptState>,
    pending_transcript_open: Option<ThreadId>,
    pending_transcript_cancellation: Option<oneshot::Sender<()>>,
    transcript_loading_frame_shown: bool,
    overlay: Option<Overlay>,
    pager_keymap: PagerKeymap,
    list_keymap: ListKeymap,
    initial_page_mode: PageLoadMode,
    chord_keymap: Arc<RuntimeChordKeymap>,
    chord_matcher: crate::keymap::KeyChordMatcher,
}

#[derive(Clone, Copy, Debug)]
enum SearchState {
    Idle,
    Active { token: usize },
}

#[derive(Clone)]
enum TranscriptPreviewState {
    Loading,
    Loaded(Vec<TranscriptPreviewLine>),
    Failed,
}

enum SessionTranscriptState {
    Loading,
    Loaded(TranscriptCells),
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptPreviewLine {
    speaker: TranscriptPreviewSpeaker,
    text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TranscriptPreviewSpeaker {
    User,
    Assistant,
}

enum LoadTrigger {
    Scroll,
    Search { token: usize },
}

async fn load_app_server_page(
    app_server: &mut AppServerSession,
    params: ThreadListParams,
) -> std::io::Result<PickerPage> {
    let response = app_server
        .thread_list(params)
        .await
        .map_err(std::io::Error::other)?;
    let num_scanned_files = response.data.len();
    let (rows, history_modes): (Vec<_>, HashMap<_, _>) = response
        .data
        .into_iter()
        .filter_map(|thread| {
            let history_mode = thread.history_mode;
            let row = row_from_app_server_thread(thread)?;
            let thread_id = row.thread_id?;
            Some((row, (thread_id, history_mode)))
        })
        .unzip();

    Ok(PickerPage {
        rows,
        history_modes,
        next_cursor: response.next_cursor.map(PageCursor::AppServer),
        num_scanned_files,
        reached_scan_cap: false,
    })
}

impl SearchState {
    fn active_token(&self) -> Option<usize> {
        match self {
            SearchState::Idle => None,
            SearchState::Active { token } => Some(*token),
        }
    }

    fn is_active(&self) -> bool {
        self.active_token().is_some()
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
struct Row {
    path: Option<PathBuf>,
    preview: String,
    thread_id: Option<ThreadId>,
    thread_name: Option<String>,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    cwd: Option<PathBuf>,
    git_branch: Option<String>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum SeenRowKey {
    Path(PathBuf),
    Thread(ThreadId),
}

impl Row {
    fn seen_key(&self) -> Option<SeenRowKey> {
        if let Some(path) = self.path.clone() {
            return Some(SeenRowKey::Path(path));
        }
        self.thread_id.map(SeenRowKey::Thread)
    }

    fn display_preview(&self) -> &str {
        self.thread_name.as_deref().unwrap_or(&self.preview)
    }

    fn matches_query(&self, query: &str) -> bool {
        if self.preview.to_lowercase().contains(query) {
            return true;
        }
        if let Some(thread_name) = self.thread_name.as_ref()
            && thread_name.to_lowercase().contains(query)
        {
            return true;
        }
        if self
            .thread_id
            .is_some_and(|thread_id| thread_id.to_string().to_lowercase().contains(query))
        {
            return true;
        }
        if self
            .git_branch
            .as_ref()
            .is_some_and(|branch| branch.to_lowercase().contains(query))
        {
            return true;
        }
        if self
            .cwd
            .as_ref()
            .is_some_and(|cwd| cwd.to_string_lossy().to_lowercase().contains(query))
        {
            return true;
        }
        false
    }
}

impl PickerState {
    fn new(
        requester: FrameRequester,
        picker_loader: PickerLoader,
        provider_filter: ProviderFilter,
        show_all: bool,
        filter_cwd: Option<PathBuf>,
        action: SessionPickerAction,
    ) -> Self {
        Self {
            use_theme_colors: true,
            requester,
            relative_time_reference: None,
            pagination: PaginationState::new(),
            all_rows: Vec::new(),
            filtered_rows: Vec::new(),
            thread_history_modes: HashMap::new(),
            seen_rows: HashSet::new(),
            selected: 0,
            scroll_top: 0,
            pending_page_down_target: None,
            frozen_footer_percent: None,
            query: String::new(),
            search_state: SearchState::Idle,
            next_request_token: 0,
            next_search_token: 0,
            picker_loader,
            view_rows: None,
            view_width: None,
            provider_filter,
            filter_mode: SessionFilterMode::from_show_all(show_all, filter_cwd.as_deref()),
            status: SessionStatus::Active,
            local_filter_cwd: filter_cwd.clone(),
            local_cwd_matches: HashMap::new(),
            worktrees_enabled: false,
            filter_cwd,
            toolbar_focus: ToolbarControl::Filter,
            density: SessionListDensity::Comfortable,
            launch_context: SessionPickerLaunchContext::Startup,
            view_persistence: None,
            action,
            sort_key: ThreadSortKey::UpdatedAt,
            inline_error: None,
            archive_state: archive::ArchiveState::default(),
            expanded_thread_id: None,
            transcript_previews: HashMap::new(),
            transcript_cells: HashMap::new(),
            pending_transcript_open: None,
            pending_transcript_cancellation: None,
            transcript_loading_frame_shown: false,
            overlay: None,
            pager_keymap: RuntimeKeymap::defaults().pager,
            list_keymap: RuntimeKeymap::defaults().list,
            initial_page_mode: PageLoadMode::StoreDefault,
            chord_keymap: Arc::default(),
            chord_matcher: crate::keymap::KeyChordMatcher::default(),
        }
    }

    fn route_key_chord(&mut self, key: KeyEvent) -> Option<KeyEvent> {
        let context = if self.overlay.is_some() {
            crate::keymap::KeymapContext::Pager
        } else {
            crate::keymap::KeymapContext::List
        };
        match self.chord_matcher.advance(
            key,
            &self.chord_keymap,
            crate::keymap::KeymapContextSet::new(context),
            tokio::time::Instant::now(),
        ) {
            crate::keymap::KeyChordMatch::PassThrough => Some(key),
            crate::keymap::KeyChordMatch::Completed(dispatch_event) => Some(dispatch_event),
            crate::keymap::KeyChordMatch::Pending(_)
            | crate::keymap::KeyChordMatch::Cancelled
            | crate::keymap::KeyChordMatch::Ignored => None,
        }
    }

    fn request_frame(&self) {
        self.requester.schedule_frame();
    }

    fn is_transcript_loading(&self) -> bool {
        self.pending_transcript_open.is_some()
    }

    fn note_transcript_loading_frame_drawn(&mut self) -> bool {
        if self.pending_transcript_open.is_some() {
            self.transcript_loading_frame_shown = true;
            true
        } else {
            false
        }
    }

    fn open_pending_transcript_if_ready(&mut self) {
        if !self.transcript_loading_frame_shown {
            return;
        }
        let Some(thread_id) = self.pending_transcript_open else {
            return;
        };
        let Some(SessionTranscriptState::Loaded(cells)) = self.transcript_cells.get(&thread_id)
        else {
            return;
        };
        self.overlay = Some(Overlay::new_transcript(
            cells.clone(),
            self.pager_keymap.clone(),
        ));
        self.pending_transcript_open = None;
        self.transcript_loading_frame_shown = false;
        self.request_frame();
    }

    fn begin_transcript_loading(&mut self, thread_id: ThreadId) {
        self.pending_transcript_open = Some(thread_id);
        self.transcript_loading_frame_shown = false;
        self.request_frame();
    }

    fn handle_overlay_event(&mut self, tui: &mut Tui, event: TuiEvent) -> Result<()> {
        let Some(overlay) = &mut self.overlay else {
            return Ok(());
        };
        overlay.handle_event(tui, event)?;
        if overlay.is_done() {
            self.overlay = None;
            self.request_frame();
        }
        Ok(())
    }

    fn open_selected_transcript(&mut self) {
        let Some(row) = self.filtered_rows.get(self.selected) else {
            return;
        };
        let Some(thread_id) = row.thread_id else {
            self.inline_error = Some("No transcript available for this session".to_string());
            self.request_frame();
            return;
        };

        match self.transcript_cells.get(&thread_id) {
            Some(SessionTranscriptState::Loaded(_)) => {
                self.begin_transcript_loading(thread_id);
            }
            Some(SessionTranscriptState::Loading) => {
                self.begin_transcript_loading(thread_id);
            }
            Some(SessionTranscriptState::Failed) | None => {
                self.transcript_cells
                    .insert(thread_id, SessionTranscriptState::Loading);
                self.begin_transcript_loading(thread_id);
                let (cancellation_tx, cancellation) = oneshot::channel();
                self.pending_transcript_cancellation = Some(cancellation_tx);
                (self.picker_loader)(PickerLoadRequest::Transcript {
                    thread_id,
                    cancellation,
                });
            }
        }
    }

    fn handle_transcript_loading_key(&mut self, key: KeyEvent) -> Option<SessionSelection> {
        match key {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) => Some(SessionSelection::Exit),
            key if self.list_keymap.cancel.is_pressed(key) => {
                if let Some(thread_id) = self.pending_transcript_open.take()
                    && matches!(
                        self.transcript_cells.get(&thread_id),
                        Some(SessionTranscriptState::Loading)
                    )
                {
                    self.transcript_cells.remove(&thread_id);
                }
                if let Some(cancellation) = self.pending_transcript_cancellation.take() {
                    let _ = cancellation.send(());
                }
                self.transcript_loading_frame_shown = false;
                self.request_frame();
                None
            }
            _ => None,
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<Option<SessionSelection>> {
        self.inline_error = None;
        if self.is_transcript_loading() {
            return Ok(self.handle_transcript_loading_key(key));
        }
        if !self.list_keymap.page_down.is_pressed(key) {
            self.pending_page_down_target = None;
        }
        // The session picker is always searchable, so plain text belongs to
        // the query first. Modified list bindings still route through the
        // runtime keymap below.
        let allow_plain_char_navigation = !is_plain_text_key_event(key);
        match key {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(Some(SessionSelection::Exit));
            }
            _ if self.list_keymap.cancel.is_pressed(key) => {
                if self.query.is_empty() {
                    return Ok(Some(SessionSelection::StartFresh));
                }
                self.clear_query_preserving_selection();
            }
            KeyEvent {
                code: KeyCode::Char('t'),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_selected_transcript();
            }
            KeyEvent {
                code: KeyCode::Char('e'),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_selected_expansion();
            }
            KeyEvent {
                code: KeyCode::Char('\u{0014}'),
                modifiers: KeyModifiers::NONE,
                ..
            } /* ^T */ => {
                self.open_selected_transcript();
            }
            KeyEvent {
                code: KeyCode::Char('\u{0005}'),
                modifiers: KeyModifiers::NONE,
                ..
            } /* ^E */ => {
                self.toggle_selected_expansion();
            }
            KeyEvent {
                code: KeyCode::Char('o'),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_density().await;
            }
            KeyEvent {
                code: KeyCode::Char('\u{000f}'),
                modifiers: KeyModifiers::NONE,
                ..
            } /* ^O */ => {
                self.toggle_density().await;
            }
            _ if self.list_keymap.accept.is_pressed(key)
                && !matches!(self.archive_state, archive::ArchiveState::Idle) => {}
            _ if self.list_keymap.accept.is_pressed(key) => {
                if let Some(row) = self.filtered_rows.get(self.selected) {
                    let path = row.path.clone();
                    if let Some(thread_id) = row.thread_id {
                        if self.status == SessionStatus::Archived {
                            self.request_unarchive(thread_id);
                            return Ok(None);
                        }
                        return Ok(Some(self.action.selection(SessionTarget {
                            path,
                            thread_id,
                            cwd: row.cwd.clone(),
                            history_mode: self.thread_history_modes.get(&thread_id).copied(),
                        })));
                    }
                    self.inline_error = Some(match path {
                        Some(path) => {
                            format!("Failed to read session metadata from {}", path.display())
                        }
                        None => {
                            String::from("Failed to read session metadata from selected session")
                        }
                    });
                    self.request_frame();
                }
            }
            _ if allow_plain_char_navigation && self.list_keymap.move_up.is_pressed(key) => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.ensure_selected_visible();
                }
                self.request_frame();
            }
            _ if allow_plain_char_navigation && self.list_keymap.move_down.is_pressed(key) => {
                if self.selected + 1 < self.filtered_rows.len() {
                    self.selected += 1;
                    self.ensure_selected_visible();
                }
                self.maybe_load_more_for_scroll();
                self.request_frame();
            }
            _ if allow_plain_char_navigation && self.list_keymap.page_up.is_pressed(key) => {
                let step = self.view_rows.unwrap_or(10).max(1);
                if self.selected > 0 {
                    self.selected = self.selected.saturating_sub(step);
                    self.ensure_selected_visible();
                    self.request_frame();
                }
            }
            _ if allow_plain_char_navigation && self.list_keymap.jump_top.is_pressed(key)
                && !self.filtered_rows.is_empty() => {
                    self.selected = 0;
                    self.ensure_selected_visible();
                    self.request_frame();
                }
            _ if allow_plain_char_navigation && self.list_keymap.jump_bottom.is_pressed(key)
                && !self.filtered_rows.is_empty() => {
                    self.selected = self.filtered_rows.len().saturating_sub(1);
                    self.ensure_selected_visible();
                    self.maybe_load_more_for_scroll();
                    self.request_frame();
                }
            _ if allow_plain_char_navigation && self.list_keymap.page_down.is_pressed(key)
                && !self.filtered_rows.is_empty() => {
                    let step = self.view_rows.unwrap_or(10).max(1);
                    let target = self.selected.saturating_add(step);
                    let max_index = self.filtered_rows.len().saturating_sub(1);
                    if target > max_index && self.pagination.next_cursor.is_some() {
                        self.pending_page_down_target = Some(target);
                        self.load_more_if_needed(LoadTrigger::Scroll);
                    } else {
                        self.selected = target.min(max_index);
                        self.ensure_selected_visible();
                        self.maybe_load_more_for_scroll();
                    }
                    self.request_frame();
                }
            KeyEvent {
                code: KeyCode::Tab, ..
            } => {
                self.focus_next_toolbar_control();
                self.request_frame();
            }
            KeyEvent {
                code: KeyCode::BackTab,
                ..
            } => {
                self.focus_previous_toolbar_control();
                self.request_frame();
            }
            _ if allow_plain_char_navigation
                && (self.list_keymap.move_left.is_pressed(key)
                    || self.list_keymap.move_right.is_pressed(key)) =>
            {
                self.change_focused_toolbar_value();
                self.request_frame();
            }
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                let mut new_query = self.query.clone();
                new_query.pop();
                self.set_query(new_query);
            }
            _ if self.archive_shortcut_available()
                && crate::key_hint::ctrl(KeyCode::Char('a')).is_press(key) =>
            {
                self.request_archive_for_selected_session();
            }
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            }
                // basic text input for search
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT)
                => {
                    let mut new_query = self.query.clone();
                    new_query.push(c);
                    self.set_query(new_query);
                }
            _ => {}
        }
        Ok(None)
    }

    fn handle_paste(&mut self, pasted: String) {
        if self.is_transcript_loading() {
            return;
        }
        let Some(pasted) = normalize_pasted_search_query(&pasted) else {
            return;
        };
        let mut new_query = self.query.clone();
        if !new_query.is_empty() && !new_query.ends_with(char::is_whitespace) {
            new_query.push(' ');
        }
        new_query.push_str(&pasted);
        self.set_query(new_query);
    }

    fn start_initial_load(&mut self) {
        self.relative_time_reference = Some(Utc::now());
        self.reset_pagination();
        self.all_rows.clear();
        self.filtered_rows.clear();
        self.thread_history_modes.clear();
        self.seen_rows.clear();
        self.local_cwd_matches.clear();
        self.selected = 0;
        self.pending_page_down_target = None;
        self.frozen_footer_percent = None;

        let search_token = if self.query.is_empty() {
            self.search_state = SearchState::Idle;
            None
        } else {
            let token = self.allocate_search_token();
            self.search_state = SearchState::Active { token };
            Some(token)
        };

        let request_token = self.allocate_request_token();
        let mode = self.initial_page_mode;
        self.pagination
            .start_load(request_token, search_token, mode);
        self.request_frame();

        (self.picker_loader)(PickerLoadRequest::Page(PageLoadRequest {
            cursor: None,
            request_token,
            search_token,
            mode,
            cwd_filter: self.active_cwd_filter(),
            status: self.status,
            provider_filter: self.provider_filter.clone(),
            sort_key: self.sort_key,
        }));
    }

    async fn handle_background_event(
        &mut self,
        event: BackgroundEvent,
    ) -> Result<Option<SessionSelection>> {
        match event {
            BackgroundEvent::Page {
                request_token,
                search_token,
                page,
            } => {
                let Some(pending) = self.pagination.finish_load(request_token) else {
                    return Ok(None);
                };
                let page_has_rows = matches!(&page, Ok(page) if !page.rows.is_empty());
                // Fall back only when the initial DB listing is unusable. Once SQLite returns
                // rows, its pagination is authoritative and an empty later page ends the list.
                let should_restart_from_store = pending.mode == PageLoadMode::StateDbOnly
                    && self.all_rows.is_empty()
                    && !page_has_rows;
                if should_restart_from_store {
                    let request_token = self.allocate_request_token();
                    let search_token = pending.search_token.or(search_token);
                    self.pagination.reset();
                    self.pagination.start_load(
                        request_token,
                        search_token,
                        PageLoadMode::StoreDefault,
                    );
                    (self.picker_loader)(PickerLoadRequest::Page(PageLoadRequest {
                        cursor: None,
                        request_token,
                        search_token,
                        mode: PageLoadMode::StoreDefault,
                        cwd_filter: self.active_cwd_filter(),
                        status: self.status,
                        provider_filter: self.provider_filter.clone(),
                        sort_key: self.sort_key,
                    }));
                    return Ok(None);
                }
                let page = page.map_err(color_eyre::Report::from)?;
                self.ingest_page(page);
                self.complete_pending_page_down();
                let completed_token = pending.search_token.or(search_token);
                self.continue_search_if_token_matches(completed_token);
            }
            BackgroundEvent::Preview { thread_id, preview } => {
                self.transcript_previews.insert(
                    thread_id,
                    match preview {
                        Ok(lines) => TranscriptPreviewState::Loaded(lines),
                        Err(_) => TranscriptPreviewState::Failed,
                    },
                );
                self.request_frame();
            }
            BackgroundEvent::Transcript {
                thread_id,
                transcript,
            } => match transcript {
                Ok(cells) => {
                    let should_open = self.pending_transcript_open == Some(thread_id);
                    self.transcript_cells
                        .insert(thread_id, SessionTranscriptState::Loaded(cells.clone()));
                    if should_open {
                        self.pending_transcript_cancellation = None;
                        self.open_pending_transcript_if_ready();
                    }
                    self.request_frame();
                }
                Err(_) => {
                    self.transcript_cells
                        .insert(thread_id, SessionTranscriptState::Failed);
                    if self.pending_transcript_open == Some(thread_id) {
                        self.pending_transcript_cancellation = None;
                        self.pending_transcript_open = None;
                        self.transcript_loading_frame_shown = false;
                        self.inline_error = Some("Could not load transcript preview".to_string());
                    }
                    self.request_frame();
                }
            },
            BackgroundEvent::Archive { thread_id, result } => {
                self.handle_archive_result(thread_id, result);
            }
            BackgroundEvent::Unarchive { thread_id, result } => {
                return Ok(self.handle_unarchive_result(thread_id, result));
            }
        }
        Ok(None)
    }

    fn reset_pagination(&mut self) {
        self.pagination.reset();
        self.frozen_footer_percent = None;
    }

    fn ingest_page(&mut self, page: PickerPage) {
        let PickerPage {
            rows,
            history_modes,
            next_cursor,
            num_scanned_files,
            reached_scan_cap,
        } = page;
        self.pagination
            .complete_page(next_cursor, num_scanned_files, reached_scan_cap);
        self.thread_history_modes.extend(history_modes);

        for row in rows {
            if let Some(cwd) = row.cwd.as_ref()
                && !self.local_cwd_matches.contains_key(cwd)
            {
                let matches = self.row_matches_local_cwd(&row);
                self.local_cwd_matches.insert(cwd.clone(), matches);
            }
            if let Some(seen_key) = row.seen_key() {
                if self.seen_rows.insert(seen_key) {
                    self.all_rows.push(row);
                }
            } else {
                self.all_rows.push(row);
            }
        }

        self.apply_filter();
    }

    fn complete_pending_page_down(&mut self) {
        let Some(target) = self.pending_page_down_target else {
            return;
        };
        if self.filtered_rows.is_empty() {
            return;
        }

        let max_index = self.filtered_rows.len().saturating_sub(1);
        if target > max_index && self.pagination.next_cursor.is_some() {
            self.load_more_if_needed(LoadTrigger::Scroll);
            return;
        }

        self.pending_page_down_target = None;
        self.selected = target.min(max_index);
        self.ensure_selected_visible();
        self.maybe_load_more_for_scroll();
        self.request_frame();
    }

    fn apply_filter(&mut self) {
        let base_iter = self
            .all_rows
            .iter()
            .filter(|row| self.row_matches_filter(row));
        if self.query.is_empty() {
            self.filtered_rows = base_iter.cloned().collect();
        } else {
            let q = self.query.to_lowercase();
            self.filtered_rows = base_iter.filter(|r| r.matches_query(&q)).cloned().collect();
        }
        if self.selected >= self.filtered_rows.len() {
            self.selected = self.filtered_rows.len().saturating_sub(1);
        }
        if self.filtered_rows.is_empty() {
            self.scroll_top = 0;
        }
        self.ensure_selected_visible();
        self.request_frame();
    }

    fn row_matches_filter(&self, row: &Row) -> bool {
        self.filter_mode == SessionFilterMode::All
            || self.local_filter_cwd.is_none()
            || row
                .cwd
                .as_ref()
                .and_then(|cwd| self.local_cwd_matches.get(cwd))
                .copied()
                .unwrap_or(false)
    }

    fn row_matches_local_cwd(&self, row: &Row) -> bool {
        let Some(filter_cwd) = self.local_filter_cwd.as_ref() else {
            return true;
        };
        let Some(row_cwd) = row.cwd.as_ref() else {
            return false;
        };
        paths_match(row_cwd, filter_cwd)
            || (self.worktrees_enabled
                && genex_git_utils::repository_identity(row_cwd)
                    .zip(genex_git_utils::repository_identity(filter_cwd))
                    .is_some_and(|(row, filter)| {
                        row.common_dir == filter.common_dir
                            && row.relative_cwd == filter.relative_cwd
                    }))
    }

    fn set_query(&mut self, new_query: String) {
        if self.query == new_query {
            return;
        }
        self.query = new_query;
        self.selected = 0;
        self.apply_filter();
        if self.query.is_empty() {
            self.search_state = SearchState::Idle;
            return;
        }
        if !self.filtered_rows.is_empty() {
            self.search_state = SearchState::Idle;
            return;
        }
        if self.pagination.reached_scan_cap || self.pagination.next_cursor.is_none() {
            self.search_state = SearchState::Idle;
            return;
        }
        let token = self.allocate_search_token();
        self.search_state = SearchState::Active { token };
        self.load_more_if_needed(LoadTrigger::Search { token });
    }

    fn clear_query_preserving_selection(&mut self) {
        let selected_key = self
            .filtered_rows
            .get(self.selected)
            .and_then(Row::seen_key);
        self.query.clear();
        self.search_state = SearchState::Idle;
        self.apply_filter();
        if let Some(selected_key) = selected_key
            && let Some(index) = self
                .filtered_rows
                .iter()
                .position(|row| row.seen_key().as_ref() == Some(&selected_key))
        {
            self.selected = index;
            self.ensure_selected_visible();
            self.request_frame();
        }
    }

    fn continue_search_if_needed(&mut self) {
        let Some(token) = self.search_state.active_token() else {
            return;
        };
        if !self.filtered_rows.is_empty() {
            self.search_state = SearchState::Idle;
            return;
        }
        if self.pagination.reached_scan_cap || self.pagination.next_cursor.is_none() {
            self.search_state = SearchState::Idle;
            return;
        }
        self.load_more_if_needed(LoadTrigger::Search { token });
    }

    fn continue_search_if_token_matches(&mut self, completed_token: Option<usize>) {
        let Some(active) = self.search_state.active_token() else {
            return;
        };
        if let Some(token) = completed_token
            && token != active
        {
            return;
        }
        self.continue_search_if_needed();
    }

    fn ensure_selected_visible(&mut self) {
        if self.filtered_rows.is_empty() {
            self.scroll_top = 0;
            return;
        }
        let viewport_rows = self.view_rows.unwrap_or(usize::MAX).max(1);
        if self.selected < self.scroll_top {
            self.scroll_top = self.selected;
        }
        while self.rendered_height_between(self.scroll_top, self.selected)
            > self.available_content_rows(viewport_rows)
            && self.scroll_top < self.selected
        {
            self.scroll_top += 1;
        }
    }

    fn ensure_minimum_rows_for_view(&mut self, minimum_rows: usize) {
        if minimum_rows == 0 {
            return;
        }
        if self.pagination.is_loading() || self.pagination.next_cursor.is_none() {
            return;
        }
        let rendered_rows = if self.filtered_rows.is_empty() {
            0
        } else {
            self.rendered_height_between(/*start*/ 0, self.filtered_rows.len() - 1)
        };
        if rendered_rows >= self.available_content_rows(minimum_rows) {
            return;
        }
        if let Some(token) = self.search_state.active_token() {
            self.load_more_if_needed(LoadTrigger::Search { token });
        } else {
            self.load_more_if_needed(LoadTrigger::Scroll);
        }
    }

    fn update_viewport(&mut self, rows: usize, width: u16) {
        self.view_rows = if rows == 0 { None } else { Some(rows) };
        self.view_width = Some(width);
        self.ensure_selected_visible();
    }

    fn maybe_load_more_for_scroll(&mut self) {
        if self.pagination.is_loading() || self.pagination.next_cursor.is_none() {
            return;
        }
        if self.filtered_rows.is_empty() {
            return;
        }
        let remaining = self.filtered_rows.len().saturating_sub(self.selected + 1);
        if remaining <= LOAD_NEAR_THRESHOLD {
            self.load_more_if_needed(LoadTrigger::Scroll);
        }
    }

    fn load_more_if_needed(&mut self, trigger: LoadTrigger) {
        let Some((cursor, mode)) = self.pagination.next_page() else {
            return;
        };
        self.freeze_footer_percent();
        let request_token = self.allocate_request_token();
        let search_token = match trigger {
            LoadTrigger::Scroll => None,
            LoadTrigger::Search { token } => Some(token),
        };
        self.pagination
            .start_load(request_token, search_token, mode);
        self.request_frame();

        (self.picker_loader)(PickerLoadRequest::Page(PageLoadRequest {
            cursor: Some(cursor),
            request_token,
            search_token,
            mode,
            cwd_filter: self.active_cwd_filter(),
            status: self.status,
            provider_filter: self.provider_filter.clone(),
            sort_key: self.sort_key,
        }));
    }

    fn freeze_footer_percent(&mut self) {
        let list_height = self.view_rows.unwrap_or_default().min(u16::MAX as usize) as u16;
        self.frozen_footer_percent = Some(picker_footer_scroll_percent(self, list_height));
    }

    fn allocate_request_token(&mut self) -> usize {
        let token = self.next_request_token;
        self.next_request_token = self.next_request_token.wrapping_add(1);
        token
    }

    fn allocate_search_token(&mut self) -> usize {
        let token = self.next_search_token;
        self.next_search_token = self.next_search_token.wrapping_add(1);
        token
    }

    /// Cycles the sort order between creation time and last-updated time.
    ///
    /// Triggers a full reload because the backend must re-sort all sessions.
    /// The existing `all_rows` are cleared and pagination restarts from the
    /// beginning with the new sort key.
    fn toggle_sort_key(&mut self) {
        self.sort_key = match self.sort_key {
            ThreadSortKey::CreatedAt => ThreadSortKey::UpdatedAt,
            ThreadSortKey::UpdatedAt
            | ThreadSortKey::RecencyAt
            | ThreadSortKey::SectionPosition => ThreadSortKey::CreatedAt,
        };
        self.start_initial_load();
    }

    fn toggle_filter_mode(&mut self) {
        let next_filter_mode = self.filter_mode.toggle(self.filter_cwd.as_deref());
        if self.filter_mode == next_filter_mode {
            return;
        }
        self.filter_mode = next_filter_mode;
        self.start_initial_load();
    }

    fn toggle_status(&mut self) {
        self.status = match self.status {
            SessionStatus::Active => SessionStatus::Archived,
            SessionStatus::Archived => SessionStatus::Active,
        };
        self.start_initial_load();
    }

    fn active_cwd_filter(&self) -> Option<PathBuf> {
        match self.filter_mode {
            SessionFilterMode::Cwd => self.filter_cwd.clone(),
            SessionFilterMode::All => None,
        }
    }

    fn focus_previous_toolbar_control(&mut self) {
        self.toolbar_focus = self.toolbar_focus.previous(self.action);
    }

    fn focus_next_toolbar_control(&mut self) {
        self.toolbar_focus = self.toolbar_focus.next(self.action);
    }

    fn change_focused_toolbar_value(&mut self) {
        match self.toolbar_focus {
            ToolbarControl::Sort => self.toggle_sort_key(),
            ToolbarControl::Filter => self.toggle_filter_mode(),
            ToolbarControl::Status => self.toggle_status(),
        }
    }

    async fn toggle_density(&mut self) {
        self.density = self.density.toggle();
        self.ensure_selected_visible();
        if let Err(err) = self.persist_density().await {
            warn!(error = %err, "failed to persist session picker view mode");
            self.inline_error = Some(format!("Failed to save view mode: {err}"));
        }
        self.request_frame();
    }

    async fn persist_density(&self) -> Result<()> {
        let Some(persistence) = &self.view_persistence else {
            return Ok(());
        };

        ConfigEditsBuilder::new(&persistence.codex_home)
            .set_session_picker_view(SessionPickerViewMode::from(self.density))
            .apply()
            .await
            .map_err(|err| color_eyre::eyre::eyre!("failed to write config.toml: {err}"))?;

        Ok(())
    }

    fn toggle_selected_expansion(&mut self) {
        let Some(row) = self.filtered_rows.get(self.selected) else {
            return;
        };
        let Some(thread_id) = row.thread_id else {
            return;
        };
        if self.expanded_thread_id == Some(thread_id) {
            self.expanded_thread_id = None;
            self.request_frame();
            return;
        }
        self.expanded_thread_id = Some(thread_id);
        if let std::collections::hash_map::Entry::Vacant(e) =
            self.transcript_previews.entry(thread_id)
        {
            e.insert(TranscriptPreviewState::Loading);
            (self.picker_loader)(PickerLoadRequest::Preview { thread_id });
        }
        self.request_frame();
    }

    fn rendered_height_between(&self, start: usize, end_inclusive: usize) -> usize {
        self.filtered_rows
            .get(start..=end_inclusive)
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(offset, row)| {
                let row_idx = start + offset;
                let is_selected = row_idx == self.selected;
                let is_expanded = is_selected
                    && row.thread_id.is_some()
                    && self.expanded_thread_id == row.thread_id;
                render_session_lines(
                    row,
                    self,
                    is_selected,
                    is_expanded,
                    /*is_zebra*/ false,
                    self.view_width.unwrap_or(u16::MAX),
                )
                .len()
            })
            .sum::<usize>()
            + self.row_separator_height() * end_inclusive.saturating_sub(start)
    }

    fn has_more_above(&self) -> bool {
        self.scroll_top > 0
    }

    fn has_more_below(&self, viewport_height: usize) -> bool {
        if self.filtered_rows.is_empty() {
            return false;
        }
        if self.pagination.next_cursor.is_some() {
            return true;
        }
        let capacity = self.available_content_rows(viewport_height);
        let mut used = 0usize;
        for (offset, row) in self.filtered_rows[self.scroll_top..].iter().enumerate() {
            let row_idx = self.scroll_top + offset;
            let is_selected = row_idx == self.selected;
            let is_expanded =
                is_selected && row.thread_id.is_some() && self.expanded_thread_id == row.thread_id;
            let row_height = render_session_lines(
                row,
                self,
                is_selected,
                is_expanded,
                /*is_zebra*/ false,
                self.view_width.unwrap_or(u16::MAX),
            )
            .len();
            let separator_height = usize::from(offset > 0) * self.row_separator_height();
            if used + separator_height + row_height > capacity {
                return true;
            }
            used += separator_height + row_height;
        }
        false
    }

    fn available_content_rows(&self, viewport_height: usize) -> usize {
        viewport_height
            .saturating_sub(usize::from(self.has_more_above()))
            .saturating_sub(usize::from(
                self.pagination.next_cursor.is_some()
                    || self.selected + 1 < self.filtered_rows.len(),
            ))
            .max(1)
    }

    fn row_separator_height(&self) -> usize {
        match self.density {
            SessionListDensity::Comfortable => 1,
            SessionListDensity::Dense => 0,
        }
    }
}

fn row_from_app_server_thread(thread: Thread) -> Option<Row> {
    let thread_id = match ThreadId::from_string(&thread.id) {
        Ok(thread_id) => thread_id,
        Err(err) => {
            warn!(thread_id = thread.id, %err, "Skipping app-server picker row with invalid id");
            return None;
        }
    };
    let preview = thread.preview.trim();
    Some(Row {
        path: thread.path,
        preview: if preview.is_empty() {
            String::from("(no message yet)")
        } else {
            preview.to_string()
        },
        thread_id: Some(thread_id),
        thread_name: thread.name,
        created_at: chrono::DateTime::from_timestamp(thread.created_at, 0)
            .map(|dt| dt.with_timezone(&Utc)),
        updated_at: chrono::DateTime::from_timestamp(thread.updated_at, 0)
            .map(|dt| dt.with_timezone(&Utc)),
        cwd: Some(thread.cwd.to_path_buf()),
        git_branch: thread.git_info.and_then(|git_info| git_info.branch),
    })
}

fn thread_list_params(
    cursor: Option<String>,
    cwd_filter: Option<ThreadListCwdFilter>,
    status: SessionStatus,
    provider_filter: ProviderFilter,
    sort_key: ThreadSortKey,
    include_non_interactive: bool,
    use_state_db_only: bool,
) -> ThreadListParams {
    ThreadListParams {
        originators: None,
        cursor,
        limit: Some(PAGE_SIZE as u32),
        sort_key: Some(sort_key),
        sort_direction: None,
        model_providers: match provider_filter {
            ProviderFilter::Any => None,
            ProviderFilter::MatchDefault(default_provider) => Some(vec![default_provider]),
        },
        source_kinds: Some(crate::resume_source_kinds(include_non_interactive)),
        archived: Some(status == SessionStatus::Archived),
        section_id: None,
        project_id: None,
        parent_thread_id: None,
        ancestor_thread_id: None,
        cwd: cwd_filter,
        use_state_db_only,
        search_term: None,
    }
}

pub(crate) fn repository_cwd_filter(
    cwd: &Path,
    uses_remote_filesystem: bool,
    worktrees_enabled: bool,
) -> ThreadListCwdFilter {
    if worktrees_enabled
        && !uses_remote_filesystem
        && let Some(cwds) = genex_git_utils::linked_worktree_cwds(cwd)
        && cwds.len() > 1
    {
        return ThreadListCwdFilter::Many(
            cwds.into_iter()
                .map(|cwd| cwd.to_string_lossy().into_owned())
                .collect(),
        );
    }
    ThreadListCwdFilter::One(cwd.to_string_lossy().into_owned())
}

fn paths_match(a: &Path, b: &Path) -> bool {
    path_utils::paths_match_after_normalization(a, b)
}

#[cfg_attr(not(test), allow(dead_code))]
fn parse_timestamp_str(ts: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

fn draw_picker(tui: &mut Tui, state: &PickerState, screen_size: Size) -> std::io::Result<()> {
    // Render full-screen overlay
    tui.draw(screen_size.height, |frame| {
        let area = frame.area();
        let [header, _header_gap, search, _search_gap, list, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(area.height.saturating_sub(PICKER_CHROME_HEIGHT)),
            Constraint::Length(4),
        ])
        .areas(area);

        let chrome = |area: Rect| {
            Rect::new(
                area.x.saturating_add(1),
                area.y,
                area.width.saturating_sub(2),
                area.height,
            )
        };

        // Header
        let header_title = if default_bg().is_some_and(is_light) {
            state.action.title().bold().fg(best_color((0, 100, 0)))
        } else {
            state.action.title().bold().cyan()
        };
        let header_line: Line = vec![header_title].into();
        frame.render_widget_ref(&header_line, chrome(header));

        // Search line
        let search = chrome(search);
        frame.render_widget_ref(&search_line(state, search.width), search);

        let list = Rect::new(
            list.x.saturating_add(2),
            list.y,
            list_viewport_width(list.width),
            list.height,
        );
        render_list(frame, list, state);
        if state.is_transcript_loading() {
            render_transcript_loading_overlay(frame, list);
        }

        render_picker_footer(frame, footer, state, list.height);
    })
}

fn list_viewport_width(width: u16) -> u16 {
    width.saturating_sub(PICKER_LIST_HORIZONTAL_INSET)
}

fn search_line(state: &PickerState, width: u16) -> Line<'_> {
    if let Some(error) = state.inline_error.as_deref() {
        return Line::from(error.red());
    }
    let search = if state.query.is_empty() {
        "Type to search".dim()
    } else {
        format!("Search: {}", state.query).into()
    };
    let search_width = UnicodeWidthStr::width(search.content.as_ref());
    let mut toolbar = toolbar_line(state, /*compact*/ false);
    if search_width.saturating_add(toolbar.width()) > usize::from(width.saturating_sub(2)) {
        toolbar = toolbar_line(state, /*compact*/ true);
    }
    let toolbar_width = toolbar.width();
    let spacer_width = width
        .saturating_sub((search_width + toolbar_width) as u16)
        .max(2) as usize;
    let available_search_width = width
        .saturating_sub(toolbar_width as u16)
        .saturating_sub(spacer_width as u16) as usize;
    let search = if search_width > available_search_width {
        let truncated = truncate_text(search.content.as_ref(), available_search_width);
        if state.query.is_empty() {
            truncated.dim()
        } else {
            truncated.into()
        }
    } else {
        search
    };

    let mut spans = vec![search, " ".repeat(spacer_width).into()];
    spans.extend(toolbar.spans);
    spans.into()
}

fn toolbar_line(state: &PickerState, compact: bool) -> Line<'static> {
    let mut spans = Vec::new();
    let separator = if compact && matches!(state.action, SessionPickerAction::Resume) {
        " "
    } else {
        "   "
    };
    spans.extend(filter_control_spans(state, compact));
    spans.push(separator.dim());
    if matches!(state.action, SessionPickerAction::Resume) {
        let status_focused = state.toolbar_focus == ToolbarControl::Status;
        if compact {
            let active_status = match state.status {
                SessionStatus::Active => "Active",
                SessionStatus::Archived => "Archived",
            };
            spans.push(toolbar_value(
                active_status,
                /*active*/ true,
                status_focused,
            ));
        } else {
            spans.push("Status: ".dim());
            spans.push(toolbar_value(
                "Active",
                state.status == SessionStatus::Active,
                status_focused,
            ));
            spans.push(toolbar_value(
                "Archived",
                state.status == SessionStatus::Archived,
                status_focused,
            ));
        }
        spans.push(separator.dim());
    }
    spans.extend(sort_control_spans(state, compact));
    spans.into()
}

fn sort_control_spans(state: &PickerState, compact: bool) -> Vec<Span<'static>> {
    let sort_focused = state.toolbar_focus == ToolbarControl::Sort;
    if compact {
        return vec![
            "Sort:".dim(),
            toolbar_value(
                sort_key_label(state.sort_key),
                /*active*/ true,
                sort_focused,
            ),
        ];
    }
    vec![
        "Sort: ".dim(),
        toolbar_value(
            sort_key_label(ThreadSortKey::UpdatedAt),
            state.sort_key == ThreadSortKey::UpdatedAt,
            sort_focused,
        ),
        toolbar_value(
            sort_key_label(ThreadSortKey::CreatedAt),
            state.sort_key == ThreadSortKey::CreatedAt,
            sort_focused,
        ),
    ]
}

fn filter_control_spans(state: &PickerState, compact: bool) -> Vec<Span<'static>> {
    let filter_focused = state.toolbar_focus == ToolbarControl::Filter;
    if compact || state.filter_cwd.is_none() {
        return vec![
            "Filter:".dim(),
            toolbar_value(
                filter_mode_label(state.filter_mode),
                /*active*/ true,
                filter_focused,
            ),
        ];
    }
    vec![
        "Filter: ".dim(),
        toolbar_value(
            filter_mode_label(SessionFilterMode::Cwd),
            state.filter_mode == SessionFilterMode::Cwd,
            filter_focused,
        ),
        toolbar_value(
            filter_mode_label(SessionFilterMode::All),
            state.filter_mode == SessionFilterMode::All,
            filter_focused,
        ),
    ]
}

fn toolbar_value(label: &'static str, active: bool, focused: bool) -> Span<'static> {
    if active {
        let value = format!("[{label}]");
        if focused {
            value.magenta()
        } else {
            value.into()
        }
    } else {
        format!(" {label} ").dim()
    }
}

fn filter_mode_label(filter_mode: SessionFilterMode) -> &'static str {
    match filter_mode {
        SessionFilterMode::Cwd => "Cwd",
        SessionFilterMode::All => "All",
    }
}

struct PickerFooterHint {
    key: String,
    wide_label: String,
    compact_label: String,
    priority: u8,
}

fn render_picker_footer(
    frame: &mut crate::custom_terminal::Frame,
    area: Rect,
    state: &PickerState,
    list_height: u16,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let separator = Rect::new(area.x, area.y, area.width, 1);
    render_picker_footer_separator(
        frame,
        separator,
        picker_footer_progress_label(state, list_height, area.width),
    );

    let lines = footer_hint_lines(state, area.width);
    for (idx, line) in lines.into_iter().enumerate() {
        let y = area.y.saturating_add(1 + idx as u16);
        if y >= area.bottom() {
            break;
        }
        frame.render_widget_ref(&line, Rect::new(area.x, y, area.width, 1));
    }
}

fn render_picker_footer_separator(
    frame: &mut crate::custom_terminal::Frame,
    area: Rect,
    progress_label: String,
) {
    if area.width == 0 {
        return;
    }

    let separator = "─".repeat(area.width as usize);
    frame.render_widget_ref(&Line::from(separator.dim()), area);

    let progress_width = UnicodeWidthStr::width(progress_label.as_str()) as u16;
    if progress_width < area.width {
        let percent_area = Rect::new(
            area.x + area.width - progress_width - 1,
            area.y,
            progress_width,
            1,
        );
        frame.render_widget_ref(&Line::from(progress_label.dim()), percent_area);
    }
}

fn picker_footer_progress_label(state: &PickerState, list_height: u16, width: u16) -> String {
    let position = if state.filtered_rows.is_empty() {
        0
    } else {
        state.selected.saturating_add(1)
    };
    let total = if state.pagination.is_loading() {
        format!("{}…", state.filtered_rows.len())
    } else {
        state.filtered_rows.len().to_string()
    };
    let percent = picker_footer_percent(state, list_height);
    let labels = [
        format!(" {position} / {total} · {percent}% "),
        format!(" {position}/{total} · {percent}% "),
        format!(" {percent}% "),
    ];
    labels
        .into_iter()
        .find(|label| UnicodeWidthStr::width(label.as_str()) < width as usize)
        .unwrap_or_default()
}

fn picker_footer_percent(state: &PickerState, list_height: u16) -> u8 {
    if state.pagination.is_loading() {
        return state.frozen_footer_percent.unwrap_or_else(|| {
            if state.filtered_rows.is_empty() {
                0
            } else {
                picker_footer_scroll_percent(state, list_height)
            }
        });
    }

    picker_footer_scroll_percent(state, list_height)
}

fn picker_footer_scroll_percent(state: &PickerState, list_height: u16) -> u8 {
    if state.filtered_rows.is_empty() {
        return 100;
    }

    let content_rows = state.available_content_rows(list_height as usize);
    let total_height =
        state.rendered_height_between(/*start*/ 0, state.filtered_rows.len() - 1);
    let max_scroll = total_height.saturating_sub(content_rows);
    if max_scroll == 0 {
        return 100;
    }
    let remaining_height =
        state.rendered_height_between(state.scroll_top, state.filtered_rows.len() - 1);
    if remaining_height <= content_rows {
        return 100;
    }

    let skipped_height = if state.scroll_top == 0 {
        0
    } else {
        state.rendered_height_between(/*start*/ 0, state.scroll_top - 1)
    };
    (((skipped_height.min(max_scroll)) as f32 / max_scroll as f32) * 100.0).round() as u8
}

fn footer_hint_lines(state: &PickerState, width: u16) -> Vec<Line<'static>> {
    if state.is_transcript_loading() {
        let hints = [
            PickerFooterHint {
                key: "loading".to_string(),
                wide_label: String::from("transcript"),
                compact_label: String::from("transcript"),
                priority: 0,
            },
            PickerFooterHint {
                key: "ctrl+c".to_string(),
                wide_label: String::from("quit"),
                compact_label: String::from("quit"),
                priority: 1,
            },
        ];
        let line = fit_footer_hints(&hints, FooterHintLabelMode::Wide, width)
            .or_else(|| fit_footer_hints(&hints, FooterHintLabelMode::Compact, width))
            .or_else(|| fit_footer_hints(&hints, FooterHintLabelMode::KeyOnly, width))
            .unwrap_or_default();
        return vec![line, Line::default()];
    }

    let action_label = if state.status == SessionStatus::Archived {
        "restore"
    } else {
        state.action.action_label()
    };
    let (esc_label, esc_compact_label) = if state.query.is_empty() {
        match state.launch_context {
            SessionPickerLaunchContext::Startup => ("start new", "new"),
            SessionPickerLaunchContext::ExistingSession { .. } => ("exit", "exit"),
        }
    } else {
        ("clear search", "clear")
    };
    let ctrl_c_label = match state.launch_context {
        SessionPickerLaunchContext::Startup => "quit",
        SessionPickerLaunchContext::ExistingSession { .. } => "exit",
    };
    let density_label = match state.density {
        SessionListDensity::Comfortable => "dense view",
        SessionListDensity::Dense => "comfortable view",
    };
    let density_compact_label = match state.density {
        SessionListDensity::Comfortable => "dense",
        SessionListDensity::Dense => "comfy",
    };
    let mut first_row_hints = Vec::new();
    if let Some(accept) = state.list_keymap.primary_hint(ListAction::Accept) {
        first_row_hints.push(PickerFooterHint {
            key: accept.display_label(),
            wide_label: action_label.to_string(),
            compact_label: action_label.to_string(),
            priority: 0,
        });
    }
    if !state.filtered_rows.is_empty() && state.archive_shortcut_available() {
        first_row_hints.push(PickerFooterHint {
            key: "ctrl+a".to_string(),
            wide_label: String::from("archive"),
            compact_label: String::from("archive"),
            priority: 2,
        });
    }
    if let Some(cancel) = state.list_keymap.primary_hint(ListAction::Cancel) {
        first_row_hints.push(PickerFooterHint {
            key: cancel.display_label(),
            wide_label: esc_label.to_string(),
            compact_label: esc_compact_label.to_string(),
            priority: 1,
        });
    }
    first_row_hints.extend([
        PickerFooterHint {
            key: "ctrl+c".to_string(),
            wide_label: ctrl_c_label.to_string(),
            compact_label: ctrl_c_label.to_string(),
            priority: 2,
        },
        PickerFooterHint {
            key: "tab".to_string(),
            wide_label: String::from("focus sort/filter"),
            compact_label: String::from("focus"),
            priority: 7,
        },
    ]);
    let option_keys = [ListAction::MoveLeft, ListAction::MoveRight]
        .into_iter()
        .filter_map(|action| state.list_keymap.primary_hint(action))
        .map(super::key_hint::ShortcutHint::display_label)
        .collect::<Vec<_>>()
        .join("/");
    if !option_keys.is_empty() {
        first_row_hints.push(PickerFooterHint {
            key: option_keys,
            wide_label: String::from("change option"),
            compact_label: String::from("option"),
            priority: 8,
        });
    }
    let mut second_row_hints = vec![
        PickerFooterHint {
            key: "ctrl+o".to_string(),
            wide_label: density_label.to_string(),
            compact_label: density_compact_label.to_string(),
            priority: 3,
        },
        PickerFooterHint {
            key: "ctrl+t".to_string(),
            wide_label: String::from("transcript"),
            compact_label: String::from("preview"),
            priority: 4,
        },
        PickerFooterHint {
            key: "ctrl+e".to_string(),
            wide_label: String::from("expand"),
            compact_label: String::from("exp"),
            priority: 6,
        },
    ];
    let browse_keys = [ListAction::MoveUp, ListAction::MoveDown]
        .into_iter()
        .filter_map(|action| state.list_keymap.primary_hint(action))
        .map(super::key_hint::ShortcutHint::display_label)
        .collect::<Vec<_>>()
        .join("/");
    if !browse_keys.is_empty() {
        second_row_hints.push(PickerFooterHint {
            key: browse_keys,
            wide_label: String::from("browse"),
            compact_label: String::from("browse"),
            priority: 5,
        });
    }

    vec![
        hint_line_for_row(&first_row_hints, width),
        hint_line_for_row(&second_row_hints, width),
    ]
}

fn hint_line_for_row(hints: &[PickerFooterHint], width: u16) -> Line<'static> {
    if width >= FOOTER_COMPACT_BREAKPOINT
        && let Some(line) = fit_footer_hints(hints, FooterHintLabelMode::Wide, width)
    {
        return line;
    }
    if let Some(line) = fit_footer_hints(hints, FooterHintLabelMode::Compact, width) {
        return line;
    }
    if let Some(line) = fit_footer_hints(hints, FooterHintLabelMode::KeyOnly, width) {
        return line;
    }

    let mut retained = (0..hints.len()).collect::<Vec<_>>();
    retained.sort_by_key(|idx| hints[*idx].priority);
    for retain_count in (1..=retained.len()).rev() {
        let mut candidate_indices = retained[..retain_count].to_vec();
        candidate_indices.sort_unstable();
        let candidate = candidate_indices
            .iter()
            .map(|idx| &hints[*idx])
            .collect::<Vec<_>>();
        if let Some(line) = fit_footer_hint_refs(&candidate, FooterHintLabelMode::KeyOnly, width) {
            return line;
        }
    }
    Line::default()
}

fn render_transcript_loading_overlay(frame: &mut crate::custom_terminal::Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let message = "Loading transcript…";
    let message_width = UnicodeWidthStr::width(message) as u16;
    let overlay_width = if area.width >= message_width.saturating_add(10) {
        message_width + 10
    } else {
        area.width
    };
    let overlay_height = if area.height >= 3 { 3 } else { 1 };
    let overlay = Rect::new(
        area.x + area.width.saturating_sub(overlay_width) / 2,
        area.y + area.height.saturating_sub(overlay_height) / 2,
        overlay_width,
        overlay_height,
    );
    let style = transcript_loading_overlay_style();
    for y in overlay.y..overlay.bottom() {
        for x in overlay.x..overlay.right() {
            frame.buffer[(x, y)].set_symbol(" ").set_style(style);
        }
    }

    let message = truncate_text(message, overlay.width as usize);
    let message_width = UnicodeWidthStr::width(message.as_str()) as u16;
    let line = Rect::new(
        overlay.x + overlay.width.saturating_sub(message_width) / 2,
        overlay.y + overlay.height / 2,
        message_width.min(overlay.width),
        1,
    );
    frame.render_widget_ref(&Line::from(message.bold()), line);
}

fn transcript_loading_overlay_style() -> Style {
    let Some(bg) = default_bg() else {
        return Style::default().bg(Color::DarkGray);
    };
    let (overlay, alpha) = if is_light(bg) {
        ((0, 0, 0), 0.08)
    } else {
        ((255, 255, 255), 0.14)
    };
    Style::default().bg(best_color(blend(overlay, bg, alpha)))
}

#[derive(Clone, Copy)]
enum FooterHintLabelMode {
    Wide,
    Compact,
    KeyOnly,
}

fn fit_footer_hints(
    hints: &[PickerFooterHint],
    mode: FooterHintLabelMode,
    width: u16,
) -> Option<Line<'static>> {
    let hint_refs = hints.iter().collect::<Vec<_>>();
    fit_footer_hint_refs(&hint_refs, mode, width)
}

fn fit_footer_hint_refs(
    hints: &[&PickerFooterHint],
    mode: FooterHintLabelMode,
    width: u16,
) -> Option<Line<'static>> {
    let gap_width = FOOTER_HINT_GAP;
    if footer_hints_width(hints, mode, gap_width) > width as usize {
        return None;
    }

    let mut spans = vec![
        " ".repeat(FOOTER_HINT_LEFT_PADDING)
            .set_style(footer_hint_label_style()),
    ];
    for (idx, hint) in hints.iter().enumerate() {
        if idx > 0 {
            spans.push(" ".repeat(gap_width).set_style(footer_hint_label_style()));
        }
        spans.push(hint.key.clone().set_style(footer_hint_key_style()));
        let label = match mode {
            FooterHintLabelMode::Wide => Some(hint.wide_label.as_str()),
            FooterHintLabelMode::Compact => Some(hint.compact_label.as_str()),
            FooterHintLabelMode::KeyOnly => None,
        };
        if let Some(label) = label {
            spans.push(" ".set_style(footer_hint_label_style()));
            spans.push(label.to_string().set_style(footer_hint_label_style()));
        }
    }
    Some(spans.into())
}

fn footer_hints_width(
    hints: &[&PickerFooterHint],
    mode: FooterHintLabelMode,
    gap_width: usize,
) -> usize {
    FOOTER_HINT_LEFT_PADDING
        + hints
            .iter()
            .enumerate()
            .map(|(idx, hint)| {
                let label_width = match mode {
                    FooterHintLabelMode::Wide => {
                        1 + UnicodeWidthStr::width(hint.wide_label.as_str())
                    }
                    FooterHintLabelMode::Compact => {
                        1 + UnicodeWidthStr::width(hint.compact_label.as_str())
                    }
                    FooterHintLabelMode::KeyOnly => 0,
                };
                let hint_width = UnicodeWidthStr::width(hint.key.as_str()) + label_width;
                if idx == 0 {
                    hint_width
                } else {
                    hint_width + gap_width
                }
            })
            .sum::<usize>()
}

fn render_list(frame: &mut crate::custom_terminal::Frame, area: Rect, state: &PickerState) {
    if area.height == 0 {
        return;
    }
    Clear.render(area, frame.buffer);

    let rows = &state.filtered_rows;
    if rows.is_empty() {
        let message = render_empty_state_line(state);
        frame.render_widget_ref(&message, area);
        return;
    }

    let show_more_above = state.has_more_above();
    let show_more_below = state.has_more_below(area.height as usize);
    let content_area = Rect::new(
        area.x,
        area.y.saturating_add(u16::from(show_more_above)),
        area.width,
        area.height
            .saturating_sub(u16::from(show_more_above))
            .saturating_sub(u16::from(show_more_below)),
    );
    if show_more_above {
        frame.render_widget_ref(
            &more_line("↑ more"),
            Rect::new(area.x, area.y, area.width, 1),
        );
    }

    let start = state.scroll_top.min(rows.len().saturating_sub(1));
    let mut y = content_area.y;
    for (idx, row) in rows[start..].iter().enumerate() {
        if y >= content_area.y.saturating_add(content_area.height) {
            break;
        }
        let row_idx = start + idx;
        let is_selected = row_idx == state.selected;
        let is_expanded =
            is_selected && row.thread_id.is_some() && state.expanded_thread_id == row.thread_id;
        let is_zebra = row_idx.is_multiple_of(2);
        for line in render_session_lines(row, state, is_selected, is_expanded, is_zebra, area.width)
        {
            if y >= content_area.y.saturating_add(content_area.height) {
                break;
            }
            frame.render_widget_ref(&line, Rect::new(area.x, y, area.width, 1));
            y = y.saturating_add(1);
        }
        if state.density == SessionListDensity::Comfortable
            && y < content_area.y.saturating_add(content_area.height)
            && start + idx + 1 < rows.len()
        {
            y = y.saturating_add(1);
        }
    }

    if state.pagination.is_loading() && y < content_area.y.saturating_add(content_area.height) {
        let loading_line: Line = vec!["  ".into(), "Loading older sessions…".italic().dim()].into();
        let rect = Rect::new(area.x, y, area.width, 1);
        frame.render_widget_ref(&loading_line, rect);
    }
    if show_more_below {
        let label = if state.pagination.is_loading() {
            "↓ loading more"
        } else {
            "↓ more"
        };
        frame.render_widget_ref(
            &more_line(label),
            Rect::new(
                area.x,
                area.y.saturating_add(area.height.saturating_sub(1)),
                area.width,
                1,
            ),
        );
    }
}

fn more_line(label: &'static str) -> Line<'static> {
    vec![label.dim()].into()
}

fn render_session_lines(
    row: &Row,
    state: &PickerState,
    is_selected: bool,
    is_expanded: bool,
    is_zebra: bool,
    width: u16,
) -> Vec<Line<'static>> {
    match state.density {
        SessionListDensity::Comfortable => {
            render_comfortable_session_lines(row, state, is_selected, is_expanded, is_zebra, width)
        }
        SessionListDensity::Dense => {
            render_dense_session_lines(row, state, is_selected, is_expanded, is_zebra, width)
        }
    }
}

fn render_comfortable_session_lines(
    row: &Row,
    state: &PickerState,
    is_selected: bool,
    is_expanded: bool,
    is_zebra: bool,
    width: u16,
) -> Vec<Line<'static>> {
    let marker = selection_marker(is_selected, is_expanded);
    let title = truncate_text(row.display_preview(), width.saturating_sub(2) as usize);
    let title = session_title_span(title, row.thread_id, state.use_theme_colors, is_selected);
    let title_line = Line::from(vec![marker, title]);
    let mut lines = vec![title_line];
    let row_style = if is_selected {
        Some(dense_selected_style())
    } else if is_zebra {
        Some(dense_zebra_style())
    } else {
        None
    };
    if let Some(style) = row_style {
        lines = apply_session_row_background(lines, style, width);
    }
    if is_expanded {
        lines.extend(render_transcript_preview_lines(row, state, width));
        return lines;
    }

    let reference = state.relative_time_reference.unwrap_or_else(Utc::now);
    let created = format_relative_time(reference, row.created_at);
    let updated = format_relative_time(reference, row.updated_at.or(row.created_at));
    let branch = row.git_branch.as_deref();
    let cwd = row
        .cwd
        .as_ref()
        .map(|path| format_directory_display(path, /*max_width*/ None));
    let show_cwd = state.filter_mode == SessionFilterMode::All
        || (state.worktrees_enabled
            && row
                .cwd
                .as_deref()
                .zip(state.local_filter_cwd.as_deref())
                .is_some_and(|(row_cwd, filter_cwd)| !paths_match(row_cwd, filter_cwd)));
    let footer_lines = render_footer_lines(
        state.sort_key,
        &created,
        &updated,
        branch,
        cwd.as_deref(),
        show_cwd,
        width,
    );
    if let Some(style) = row_style {
        lines.extend(apply_session_row_background(footer_lines, style, width));
    } else {
        lines.extend(footer_lines);
    }
    lines
}

fn apply_session_row_background(
    lines: Vec<Line<'static>>,
    style: Style,
    width: u16,
) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| apply_line_background(line, style, width))
        .collect()
}

fn apply_line_background(mut line: Line<'static>, style: Style, width: u16) -> Line<'static> {
    let padding = (width as usize).saturating_sub(line.width());
    if padding > 0 {
        line.spans.push(" ".repeat(padding).set_style(style));
    }
    line.style = line.style.patch(style);
    for span in &mut line.spans {
        // Keep identity accents while inheriting the selected row's background.
        span.style = style.patch(span.style);
    }
    line
}

fn render_dense_session_lines(
    row: &Row,
    state: &PickerState,
    is_selected: bool,
    is_expanded: bool,
    is_zebra: bool,
    width: u16,
) -> Vec<Line<'static>> {
    let marker = selection_marker(is_selected, is_expanded);
    let reference = state.relative_time_reference.unwrap_or_else(Utc::now);
    let created = format_relative_time(reference, row.created_at);
    let updated = format_relative_time(reference, row.updated_at.or(row.created_at));
    let date = match state.sort_key {
        ThreadSortKey::CreatedAt => created,
        ThreadSortKey::UpdatedAt | ThreadSortKey::RecencyAt | ThreadSortKey::SectionPosition => {
            updated
        }
    };
    let mut lines = vec![dense_summary_line(DenseSummaryInput {
        marker,
        date: &date,
        title: row.display_preview(),
        thread_id: row.thread_id,
        use_theme_colors: state.use_theme_colors,
        is_selected,
        is_zebra,
        width,
    })];
    if is_expanded {
        lines.extend(render_transcript_preview_lines(row, state, width));
    }
    lines
}

struct DenseSummaryInput<'a> {
    thread_id: Option<ThreadId>,
    use_theme_colors: bool,
    marker: Span<'static>,
    date: &'a str,
    title: &'a str,
    is_selected: bool,
    is_zebra: bool,
    width: u16,
}

fn dense_summary_line(input: DenseSummaryInput<'_>) -> Line<'static> {
    let marker_width = input.marker.width();
    let available = (input.width as usize).saturating_sub(marker_width);
    let columns = dense_columns(available);
    let title = session_title_span(
        dense_column_text(input.title, columns.title_width),
        input.thread_id,
        input.use_theme_colors,
        input.is_selected,
    );

    let spans = vec![
        input.marker,
        dense_column_text(input.date, columns.date_width).dim(),
        title,
    ];
    let mut line = Line::from(spans);
    if input.is_selected {
        let padding = (input.width as usize).saturating_sub(line.width());
        if padding > 0 {
            line.spans
                .push(" ".repeat(padding).set_style(dense_selected_style()));
        }
        line = line.style(dense_selected_style());
    } else if input.is_zebra {
        let padding = (input.width as usize).saturating_sub(line.width());
        if padding > 0 {
            line.spans
                .push(" ".repeat(padding).set_style(dense_zebra_style()));
        }
        line = line.style(dense_zebra_style());
    }
    line
}

struct DenseColumns {
    date_width: usize,
    title_width: usize,
}

fn dense_columns(width: usize) -> DenseColumns {
    let date_width = SESSION_META_DATE_WIDTH;
    DenseColumns {
        date_width,
        title_width: width.saturating_sub(date_width),
    }
}

fn dense_zebra_style() -> Style {
    dense_row_background_style(/*selected*/ false)
}

fn dense_selected_style() -> Style {
    selected_session_style().patch(dense_row_background_style(/*selected*/ true))
}

fn dense_row_background_style(selected: bool) -> Style {
    let Some(bg) = default_bg() else {
        return Style::default();
    };
    let (overlay, alpha) = if is_light(bg) {
        ((0, 0, 0), if selected { 0.12 } else { 0.04 })
    } else {
        ((255, 255, 255), if selected { 0.12 } else { 0.055 })
    };
    Style::default().bg(best_color(blend(overlay, bg, alpha)))
}

fn dense_column_text(text: &str, width: usize) -> String {
    let text = truncate_text(text, width.saturating_sub(1));
    let padding = width.saturating_sub(UnicodeWidthStr::width(text.as_str()));
    format!("{text}{}", " ".repeat(padding))
}

fn selection_marker(is_selected: bool, is_expanded: bool) -> Span<'static> {
    match (is_selected, is_expanded) {
        (true, true) => "⌄ ".set_style(selected_session_style().bold()),
        (true, false) => "❯ ".set_style(selected_session_style().bold()),
        (false, _) => "  ".into(),
    }
}

fn selected_session_style() -> Style {
    if default_bg().is_some_and(is_light) {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::Yellow)
    }
}

fn session_title_span(
    title: String,
    thread_id: Option<ThreadId>,
    use_theme_colors: bool,
    is_selected: bool,
) -> Span<'static> {
    if use_theme_colors && let Some(thread_id) = thread_id {
        title.fg(crate::thread_color::thread_color(thread_id))
    } else if is_selected {
        title.set_style(selected_session_style())
    } else {
        title.into()
    }
}

fn render_footer_lines(
    sort_key: ThreadSortKey,
    created: &str,
    updated: &str,
    branch: Option<&str>,
    cwd: Option<&str>,
    show_cwd: bool,
    width: u16,
) -> Vec<Line<'static>> {
    let date = match sort_key {
        ThreadSortKey::CreatedAt => created,
        ThreadSortKey::UpdatedAt | ThreadSortKey::RecencyAt | ThreadSortKey::SectionPosition => {
            updated
        }
    };
    let mut parts = vec![FooterPart::Date(date.to_string())];
    if show_cwd {
        parts.push(FooterPart::Cwd(cwd.map(str::to_string)));
    }
    parts.push(FooterPart::Branch(branch.map(str::to_string)));
    pack_footer_parts(parts, width)
}

enum FooterPart {
    Date(String),
    Branch(Option<String>),
    Cwd(Option<String>),
}

impl FooterPart {
    fn text(&self) -> &str {
        match self {
            FooterPart::Date(text) => text,
            FooterPart::Branch(Some(text)) | FooterPart::Cwd(Some(text)) => text,
            FooterPart::Branch(None) => "no branch",
            FooterPart::Cwd(None) => "no cwd",
        }
    }

    fn prefix(&self) -> Option<&'static str> {
        match self {
            FooterPart::Date(_) => None,
            FooterPart::Branch(_) => Some(SESSION_META_BRANCH_ICON),
            FooterPart::Cwd(_) => Some(SESSION_META_CWD_ICON),
        }
    }
}

fn pack_footer_parts(parts: Vec<FooterPart>, width: u16) -> Vec<Line<'static>> {
    let available_width = width as usize;
    if available_width <= SESSION_META_INDENT_WIDTH {
        return Vec::new();
    }
    let cwd_width = cwd_column_width(available_width);
    let all_parts_width = footer_parts_width(&parts, cwd_width);
    if all_parts_width <= available_width {
        return vec![footer_line(parts, available_width, cwd_width)];
    }

    let mut lines = Vec::with_capacity(parts.len());
    let mut current_parts = Vec::new();
    for part in parts {
        let mut candidate_parts = std::mem::take(&mut current_parts);
        candidate_parts.push(part);
        if candidate_parts.len() > 1
            && footer_parts_width(&candidate_parts, cwd_width) > available_width
        {
            let previous_parts = candidate_parts
                .drain(..candidate_parts.len().saturating_sub(1))
                .collect();
            lines.push(footer_line(previous_parts, available_width, cwd_width));
        }
        current_parts = candidate_parts;
    }
    if !current_parts.is_empty() {
        lines.push(footer_line(current_parts, available_width, cwd_width));
    }
    lines
}

fn cwd_column_width(width: usize) -> usize {
    let available = width.saturating_sub(
        SESSION_META_INDENT_WIDTH + SESSION_META_DATE_WIDTH + 2 * SESSION_META_FIELD_GAP_WIDTH,
    );
    (available / 2).clamp(SESSION_META_MIN_CWD_WIDTH, SESSION_META_MAX_CWD_WIDTH)
}

fn footer_parts_width(parts: &[FooterPart], cwd_width: usize) -> usize {
    let content_width: usize = parts
        .iter()
        .enumerate()
        .map(|(idx, part)| footer_part_width(part, idx + 1 < parts.len(), cwd_width))
        .sum();
    SESSION_META_INDENT_WIDTH + content_width
}

fn footer_part_width(part: &FooterPart, padded: bool, cwd_width: usize) -> usize {
    let prefix_width = part.prefix().map_or(0, UnicodeWidthStr::width);
    let prefix_gap_width = usize::from(part.prefix().is_some() && !part.text().is_empty());
    let text_width = UnicodeWidthStr::width(part.text());
    let actual_width = prefix_width + prefix_gap_width + text_width;
    match part {
        FooterPart::Date(_) if padded => SESSION_META_DATE_WIDTH.max(actual_width),
        FooterPart::Cwd(_) if padded => cwd_width,
        _ => actual_width,
    }
}

fn footer_line(parts: Vec<FooterPart>, width: usize, cwd_width: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec!["  ".into()];
    let mut remaining_width = width.saturating_sub(SESSION_META_INDENT_WIDTH);
    let part_count = parts.len();
    for (idx, part) in parts.into_iter().enumerate() {
        if idx > 0 {
            let gap_width = SESSION_META_FIELD_GAP_WIDTH.min(remaining_width);
            if gap_width > 0 {
                spans.push(" ".repeat(gap_width).dim());
                remaining_width = remaining_width.saturating_sub(gap_width);
            }
        }
        let padded = idx + 1 < part_count;
        let target_width = match part {
            FooterPart::Date(_) if padded => Some(SESSION_META_DATE_WIDTH),
            FooterPart::Cwd(_) if padded => Some(cwd_width),
            FooterPart::Date(_) | FooterPart::Branch(_) | FooterPart::Cwd(_) => None,
        };
        let used_width = push_footer_part(&mut spans, part, target_width, remaining_width);
        remaining_width = remaining_width.saturating_sub(used_width);
        if let Some(target_width) = target_width {
            let padding = target_width.saturating_sub(used_width);
            if padding > 0 {
                spans.push(" ".repeat(padding).dim());
                remaining_width = remaining_width.saturating_sub(padding);
            }
        }
    }
    spans.into()
}

fn push_footer_part(
    spans: &mut Vec<Span<'static>>,
    part: FooterPart,
    target_width: Option<usize>,
    available_width: usize,
) -> usize {
    let text = part.text().to_string();
    let Some(prefix) = part.prefix() else {
        let text = truncate_text(&text, available_width);
        let width = UnicodeWidthStr::width(text.as_str());
        spans.push(text.dim());
        return width;
    };

    let prefix_width = UnicodeWidthStr::width(prefix);
    if available_width <= prefix_width {
        let prefix = truncate_text(prefix, available_width);
        let width = UnicodeWidthStr::width(prefix.as_str());
        spans.push(prefix.dim());
        return width;
    }

    spans.push(prefix.dim());
    let mut used_width = prefix_width;
    if !text.is_empty() && used_width < available_width {
        spans.push(" ".dim());
        used_width += 1;
    }
    let text_width = target_width
        .unwrap_or(available_width)
        .saturating_sub(used_width)
        .min(available_width.saturating_sub(used_width));
    let text = truncate_text(&text, text_width);
    let rendered_text_width = UnicodeWidthStr::width(text.as_str());
    match part {
        FooterPart::Branch(None) | FooterPart::Cwd(None) => spans.push(text.dim().italic()),
        _ => spans.push(text.dim()),
    }
    used_width + rendered_text_width
}

fn render_transcript_preview_lines(
    row: &Row,
    state: &PickerState,
    width: u16,
) -> Vec<Line<'static>> {
    let mut details = render_expanded_session_details(row, state, width);
    let Some(thread_id) = row.thread_id else {
        return details;
    };
    let preview_lines = match state.transcript_previews.get(&thread_id) {
        Some(TranscriptPreviewState::Loading) => {
            vec![vec!["  │ ".dim(), "Loading recent transcript...".italic().dim()].into()]
        }
        Some(TranscriptPreviewState::Failed) => vec![
            vec![
                "  │ ".dim(),
                "Could not load transcript preview".italic().red(),
            ]
            .into(),
        ],
        Some(TranscriptPreviewState::Loaded(lines)) => {
            render_conversation_preview_lines(lines, width, row.cwd.as_deref())
        }
        None => Vec::new(),
    };
    details.extend(preview_lines);
    details
}

fn render_expanded_session_details(
    row: &Row,
    state: &PickerState,
    width: u16,
) -> Vec<Line<'static>> {
    let reference = state.relative_time_reference.unwrap_or_else(Utc::now);
    let session = match (row.thread_name.as_deref(), row.thread_id) {
        (Some(thread_name), Some(thread_id)) => format!("{thread_name} ({thread_id})"),
        (Some(thread_name), None) => thread_name.to_string(),
        (None, Some(thread_id)) => thread_id.to_string(),
        (None, None) => "-".to_string(),
    };
    let directory = row
        .cwd
        .as_ref()
        .map(|path| format_directory_display(path, /*max_width*/ None))
        .unwrap_or_else(|| "-".to_string());
    let branch = row
        .git_branch
        .as_ref()
        .map(|branch| format!("{SESSION_META_BRANCH_ICON} {branch}"))
        .unwrap_or_else(|| format!("{SESSION_META_BRANCH_ICON} no branch"));

    vec![
        expanded_detail_line("Session:", &session, width),
        expanded_time_detail_line("Created:", reference, row.created_at, width),
        expanded_time_detail_line(
            "Updated:",
            reference,
            row.updated_at.or(row.created_at),
            width,
        ),
        expanded_detail_line("Directory:", &directory, width),
        expanded_detail_line("Branch:", &branch, width),
        vec!["  │".dim()].into(),
        vec!["  │ ".dim(), "Conversation:".dim()].into(),
    ]
}

fn render_conversation_preview_lines(
    lines: &[TranscriptPreviewLine],
    width: u16,
    cwd: Option<&Path>,
) -> Vec<Line<'static>> {
    if lines.is_empty() {
        return vec![
            vec![
                "  └ ".dim(),
                "No transcript preview available".italic().dim(),
            ]
            .into(),
        ];
    }

    let mut rendered = Vec::new();
    for line in lines {
        rendered.extend(render_transcript_content_lines(line, width, cwd));
    }
    let rendered_len = rendered.len();
    rendered
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            let prefix = if idx + 1 == rendered_len {
                "  └ "
            } else {
                "  │ "
            };
            prefix_transcript_line(prefix, line)
        })
        .collect()
}

fn render_transcript_content_lines(
    line: &TranscriptPreviewLine,
    width: u16,
    cwd: Option<&Path>,
) -> Vec<Line<'static>> {
    let content_width = width.saturating_sub(4) as usize;
    let lines = match line.speaker {
        TranscriptPreviewSpeaker::User => vec![conversation_content_line(
            Line::from(line.text.clone()),
            conversation_user_style(),
        )],
        TranscriptPreviewSpeaker::Assistant => {
            let mut lines = render_assistant(&line.text, /*width*/ None, cwd, &|_| false)
                .lines
                .into_iter()
                .map(|line| line.line)
                .collect::<Vec<_>>();
            for line in &mut lines {
                *line = conversation_content_line(line.clone(), conversation_assistant_style());
            }
            lines
        }
    };
    adaptive_wrap_lines(lines, RtOptions::new(content_width.max(/*other*/ 1)))
}

fn conversation_content_line(mut line: Line<'static>, style: Style) -> Line<'static> {
    line.style = line.style.patch(style);
    for span in &mut line.spans {
        span.style = span.style.patch(style);
    }
    line
}

fn prefix_transcript_line(prefix: &'static str, line: Line<'static>) -> Line<'static> {
    let mut spans = vec![prefix.set_style(transcript_prefix_style(&line))];
    spans.extend(line.spans);
    Line::from(spans).style(line.style)
}

fn transcript_prefix_style(line: &Line<'_>) -> Style {
    let style = line
        .spans
        .iter()
        .find(|span| !span.content.trim().is_empty())
        .map(|span| line.style.patch(span.style))
        .unwrap_or(line.style);
    connector_style_from_content(style)
}

fn connector_style_from_content(style: Style) -> Style {
    Style {
        fg: style.fg,
        bg: style.bg,
        ..Style::default()
    }
}

fn conversation_assistant_style() -> Style {
    if default_bg().is_some_and(is_light) {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn conversation_user_style() -> Style {
    if default_bg().is_some_and(is_light) {
        Style::default().fg(Color::DarkGray).italic()
    } else {
        Style::default().fg(Color::Gray).italic()
    }
}

fn expanded_detail_line(label: &'static str, value: &str, width: u16) -> Line<'static> {
    const LABEL_WIDTH: usize = 10;
    let prefix_width = 4;
    let gap_width = 2;
    let value_width = (width as usize)
        .saturating_sub(prefix_width + LABEL_WIDTH + gap_width)
        .max(1);
    vec![
        "  │ ".dim(),
        format!("{label:<LABEL_WIDTH$}").dim(),
        "  ".dim(),
        truncate_text(value, value_width).into(),
    ]
    .into()
}

fn expanded_time_detail_line(
    label: &'static str,
    reference: DateTime<Utc>,
    ts: Option<DateTime<Utc>>,
    width: u16,
) -> Line<'static> {
    let Some(ts) = ts else {
        return expanded_detail_line(label, "-", width);
    };
    let value = format!(
        "{} · {}",
        format_relative_time_long(reference, ts),
        format_timestamp(ts)
    );
    expanded_detail_line(label, &value, width)
}

fn format_relative_time(reference: DateTime<Utc>, ts: Option<DateTime<Utc>>) -> String {
    let Some(ts) = ts else {
        return "-".to_string();
    };
    let seconds = (reference - ts).num_seconds().max(0);
    if seconds == 0 {
        return "now".to_string();
    }
    if seconds < 60 {
        return format!("{seconds}s ago");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    format!("{days}d ago")
}

fn format_relative_time_long(reference: DateTime<Utc>, ts: DateTime<Utc>) -> String {
    let seconds = (reference - ts).num_seconds().max(0);
    if seconds == 0 {
        return "now".to_string();
    }
    if seconds < 60 {
        return plural_time(seconds, "second");
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return plural_time(minutes, "minute");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return plural_time(hours, "hour");
    }
    plural_time(hours / 24, "day")
}

fn plural_time(value: i64, unit: &str) -> String {
    if value == 1 {
        format!("1 {unit} ago")
    } else {
        format!("{value} {unit}s ago")
    }
}

fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn render_empty_state_line(state: &PickerState) -> Line<'static> {
    if !state.query.is_empty() {
        if state.search_state.is_active()
            || (state.pagination.is_loading() && state.pagination.next_cursor.is_some())
        {
            return vec!["Searching…".italic().dim()].into();
        }
        if state.pagination.reached_scan_cap {
            let msg = format!(
                "Search scanned first {} sessions; more may exist",
                state.pagination.num_scanned_files
            );
            return vec![Span::from(msg).italic().dim()].into();
        }
        return vec!["No results for your search".italic().dim()].into();
    }

    if state.pagination.is_loading() {
        if state.all_rows.is_empty() && state.pagination.num_scanned_files == 0 {
            return vec!["Loading sessions…".italic().dim()].into();
        }
        return vec!["Loading older sessions…".italic().dim()].into();
    }

    vec!["No sessions yet".italic().dim()].into()
}
