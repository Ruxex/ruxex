// Forbid accidental stdout/stderr writes in the *library* portion of the TUI.
// The standalone `genex-tui` binary prints a short help message before the
// alternate‑screen mode starts; that file opts‑out locally via `allow`.
#![recursion_limit = "256"]
#![deny(clippy::print_stdout, clippy::print_stderr)]
#![deny(clippy::disallowed_methods)]
use crate::legacy_core::config::Config;
use crate::legacy_core::config::ConfigBuilder;
use crate::legacy_core::config::ConfigOverrides;
use crate::legacy_core::config::ConfigTomlLoadResult;
use crate::legacy_core::config::load_config_toml_with_layer_stack;
#[cfg(test)]
use crate::legacy_core::config::resolve_bootstrap_http_client_factory;
use crate::legacy_core::config::resolve_profile_v2_config_path;
use crate::session_resume::ResolveCwdOutcome;
use crate::session_resume::ResumeCwdContext;
use crate::session_resume::effective_resume_cwd_mode;
use crate::session_resume::read_session_cwd;
use crate::session_resume::resolve_cwd_for_resume_or_fork;
pub use crate::startup_error::LocalStateDbStartupError;
use additional_dirs::add_dir_warning_message;
use app::App;
pub use app::AppExitInfo;
pub use app::DisconnectInfo;
pub use app::ExitReason;
pub use app::ResumableThread;
use app_server_session::AppServerSession;
use app_server_session::ThreadParamsMode;
use color_eyre::eyre::WrapErr;
use cwd_prompt::CwdPromptAction;
use genex_app_server_client::AppServerClient;
use genex_app_server_client::AppServerRuntime;
use genex_app_server_client::AppServerRuntimeBuilder;
use genex_app_server_client::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY;
use genex_app_server_client::InProcessAppServerClient;
use genex_app_server_client::InProcessClientStartArgs;
use genex_app_server_client::ProviderInit;
use genex_app_server_client::RemoteAppServerClient;
use genex_app_server_client::RemoteAppServerConnectArgs;
pub use genex_app_server_client::RemoteAppServerEndpoint;
use genex_app_server_client::configured_app_server_runtime_from_config;
use genex_app_server_client::provider_init_from_config_with_descriptor;
use genex_app_server_protocol::AskForApproval;
use genex_app_server_protocol::ConfigWarningNotification;
use genex_app_server_protocol::GetAccountResponse;
use genex_app_server_protocol::ProviderAuthStatus;
use genex_app_server_protocol::Thread as AppServerThread;
#[cfg(test)]
use genex_app_server_protocol::ThreadListCwdFilter;
use genex_app_server_protocol::ThreadListParams;
use genex_app_server_protocol::ThreadSortKey as AppServerThreadSortKey;
use genex_app_server_protocol::ThreadSourceKind;
use genex_config::ConfigLoadError;
use genex_config::LoaderOverrides;
use genex_config::format_config_error_with_source;
use genex_config::types::ResumeCwdMode;
use genex_exec_server::EnvironmentManager;
use genex_exec_server::ExecServerRuntimePaths;
use genex_features::Feature;
use genex_login::default_client::originator;
use genex_login::default_client::set_default_client_residency_requirement;
use genex_login::enforce_login_restrictions;
use genex_model_provider::ModelProviderFactory;
use genex_model_provider::ProviderDescriptor;
use genex_protocol::ThreadId;
use genex_protocol::auth::AuthMode;
use genex_protocol::config_types::AltScreenMode;
use genex_protocol::config_types::SandboxMode;
use genex_rollout::StateDbHandle;
use genex_rollout::state_db;
use genex_state::log_db;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_absolute_path::canonicalize_existing_preserving_symlinks;
use genex_utils_home_dir::find_genex_home;
pub use session_archive_commands::DeleteConfirmation;
pub use session_archive_commands::SessionArchiveAction;
pub use session_archive_commands::SessionArchiveCommandOptions;
pub use session_archive_commands::run_session_archive_command;
pub use session_queue_commands::run_session_queue_command;
use std::fs::OpenOptions;
use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
pub use token_usage::TokenUsage;
use tracing::error;
use tracing::warn;
use tracing_appender::non_blocking;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use url::Url;
use uuid::Uuid;

pub(crate) use genex_app_server_client::legacy_core;

pub(crate) use worktree_startup::ManagedTuiWorktree;

mod additional_dirs;
mod app;
mod app_backtrack;
mod app_command;
mod app_event;
mod app_event_sender;
mod app_server_approval_conversions;
mod app_server_connection;
mod app_server_session;
mod approval_events;
mod bottom_pane;
mod branch_summary;
mod chatwidget;
mod cli;
mod clipboard_copy;
mod clipboard_html;
mod clipboard_paste;
mod collaboration_modes;
mod color;
mod config_update;
pub(crate) mod custom_terminal;
mod daybreak;
mod permission_discovery;
mod worktree_browser;
pub use custom_terminal::Terminal;
mod assistant_directives;
mod cwd_prompt;
mod diff_model;
mod diff_render;
mod dynamic_tools;
mod dynamic_tools_mcp;
mod exec_cell;
mod exec_command;
mod external_editor;
mod file_search;
mod get_git_diff;
mod git_action_directives;
mod goal_display;
mod goal_files;
mod history_cell;
mod hooks_rpc;
mod ide_context;
mod inline_visualization;
pub(crate) mod insert_history;
pub use insert_history::insert_history_lines;
mod footer_hint;
mod key_hint;
mod keymap;
mod keymap_setup;
mod line_truncation;
pub(crate) mod live_wrap;
mod local_settings;
pub use live_wrap::RowBuilder;
mod managed_new_thread_defaults;
mod markdown;
mod markdown_render;
mod markdown_stream;
mod markdown_text_merge;
mod mention_codec;
mod model_catalog;
mod model_migration;
mod motion;
mod multi_agents;
mod named_session_lookup;
mod notifications;
#[cfg(any(not(debug_assertions), test))]
mod npm_registry;
pub(crate) mod onboarding;
mod pager_overlay;
pub(crate) mod public_widgets;
mod render;
mod resize_reflow_cap;
mod resume_picker;
mod screen_reader;
mod selection_list;
mod service_tier_resolution;
mod session_archive_commands;
mod session_log;
mod session_queue_commands;
mod session_resume;
mod session_start;
mod session_state;
mod skills_helpers;
mod slash_command;
mod startup_draft;
mod startup_error;
mod startup_hooks_review;
mod startup_orchestration;
mod startup_preflight;
mod status;
mod status_indicator_widget;
mod streaming;
mod style;
mod system_motion;
mod task_mentions;
mod temporary_structured_request;
mod terminal_hyperlinks;
mod terminal_palette;
mod terminal_probe;
mod terminal_title;
mod terminal_visualization_instructions;
mod text_formatting;
mod theme_picker;
mod thread_color;
mod thread_transcript;
mod token_usage;
mod tooltips;
mod transcript_reflow;
mod tui;
mod ui_consts;
mod unarchive_prompt;
mod update_versions;
mod version;
mod vim_search;
mod width;
mod workspace_command;
mod worktree_startup;

mod wrapping;

mod table_detect;
#[cfg(test)]
pub(crate) mod test_backend;
#[cfg(test)]
pub(crate) mod test_support;

use crate::onboarding::onboarding_screen::OnboardingScreenArgs;
use crate::onboarding::onboarding_screen::run_onboarding_app;
use crate::startup_hooks_review::StartupHooksReviewOutcome;
use crate::startup_hooks_review::load_startup_hooks_review_entry;
use crate::startup_hooks_review::maybe_run_startup_hooks_review;
use crate::tui::Tui;
pub use cli::Cli;
use genex_arg0::Arg0DispatchPaths;
pub use markdown_render::render_markdown_text;
pub use public_widgets::composer_input::ComposerAction;
pub use public_widgets::composer_input::ComposerInput;
// (tests access modules directly within the crate)

const TUI_LOG_FILE_NAME: &str = "genex-tui.log";
const INTERACTIVE_OTEL_SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(/*millis*/ 500);

const AUTO_CONNECT_DAEMON_CONNECT_TIMEOUT: std::time::Duration =
    std::time::Duration::from_millis(50);

/// Hidden argument used when a provider-owned host re-enters itself as the
/// AppServer daemon process.
pub use genex_app_server_daemon::HOST_APP_SERVER_DAEMON_MODE_ARG as APP_SERVER_DAEMON_MODE_ARG;

type ProviderRuntimeFactory = (fn(ProviderInit) -> AppServerRuntime, ProviderDescriptor);

#[allow(clippy::too_many_arguments)]
async fn start_embedded_app_server(
    arg0_paths: Arg0DispatchPaths,
    config: Config,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    log_db: Option<log_db::LogDbLayer>,
    state_db: Option<StateDbHandle>,
    environment_manager: Arc<EnvironmentManager>,
    provider_factory: Option<ProviderRuntimeFactory>,
) -> color_eyre::Result<InProcessAppServerClient> {
    start_embedded_app_server_with(
        arg0_paths,
        config,
        cli_kv_overrides,
        loader_overrides,
        strict_config,
        log_db,
        state_db,
        environment_manager,
        provider_factory,
        InProcessAppServerClient::start,
    )
    .await
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AppServerTarget {
    Embedded,
    LocalDaemon {
        endpoint: RemoteAppServerEndpoint,
        allow_embedded_fallback: bool,
    },
    Remote {
        endpoint: RemoteAppServerEndpoint,
    },
}

impl AppServerTarget {
    pub(crate) fn uses_remote_workspace(&self) -> bool {
        matches!(self, Self::Remote { .. })
    }

    fn thread_params_mode(&self) -> ThreadParamsMode {
        if self.uses_remote_workspace() {
            ThreadParamsMode::Remote
        } else {
            ThreadParamsMode::Embedded
        }
    }
}

async fn init_state_db_for_app_server_target(
    config: &Config,
    app_server_target: &AppServerTarget,
) -> std::io::Result<Option<StateDbHandle>> {
    match app_server_target {
        AppServerTarget::Embedded => {
            genex_app_server_client::mount_provider_home_for_config(config).map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("failed to mount provider data root: {err}"),
                )
            })?;
            state_db::try_init(config).await.map(Some).map_err(|err| {
                let database_path = genex_state::runtime_db_path_for_corruption_error(&err)
                    .unwrap_or_else(|| config.sqlite_config().state_db_path());
                std::io::Error::other(LocalStateDbStartupError::new(
                    database_path,
                    format!("{err:#}"),
                ))
            })
        }
        AppServerTarget::LocalDaemon { .. } => {
            genex_app_server_client::mount_provider_home_for_config(config).map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("failed to mount provider data root: {err}"),
                )
            })?;
            Ok(state_db::get_state_db(config).await)
        }
        // A remote AppServer owns the provider-scoped home and validates its
        // marker before opening credentials or provider-owned SQLite. The TUI
        // must not open the local rollout/state database in remote mode.
        AppServerTarget::Remote { .. } => Ok(None),
    }
}

// TODO(jif) delete after 22/11/2026.
fn remove_legacy_tui_log_file(genex_home: &Path) {
    // Shared append-only TUI logs could grow without bound. Existing processes
    // may still hold the file open, so startup cleanup is best effort.
    let _ = std::fs::remove_file(genex_home.join("log").join(TUI_LOG_FILE_NAME));
}

fn remote_addr_has_explicit_port(addr: &str, parsed: &Url) -> bool {
    let Some(host) = parsed.host_str() else {
        return false;
    };
    if parsed.port().is_some() {
        return true;
    }

    let Some((_, rest)) = addr.split_once("://") else {
        return false;
    };
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let host_and_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host_and_port)| host_and_port);
    let explicit_default_port = match parsed.scheme() {
        "ws" => 80,
        "wss" => 443,
        _ => return false,
    };
    let expected_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    host_and_port == format!("{expected_host}:{explicit_default_port}")
}

fn websocket_url_supports_auth_token(parsed: &Url) -> bool {
    match (parsed.scheme(), parsed.host()) {
        ("wss", Some(_)) => true,
        ("ws", Some(url::Host::Domain(domain))) => domain.eq_ignore_ascii_case("localhost"),
        ("ws", Some(url::Host::Ipv4(addr))) => addr.is_loopback(),
        ("ws", Some(url::Host::Ipv6(addr))) => addr.is_loopback(),
        _ => false,
    }
}

pub fn resolve_remote_addr(addr: &str) -> color_eyre::Result<RemoteAppServerEndpoint> {
    if let Some(socket_path) = addr.strip_prefix("unix://") {
        let socket_path = if socket_path.is_empty() {
            let genex_home = find_genex_home().wrap_err("failed to resolve GENEX_HOME")?;
            genex_app_server_client::app_server_control_socket_path(&genex_home)
                .map_err(color_eyre::Report::new)?
        } else {
            AbsolutePathBuf::relative_to_current_dir(socket_path)
                .map_err(color_eyre::Report::new)?
        };
        return Ok(RemoteAppServerEndpoint::UnixSocket { socket_path });
    }

    let parsed = match Url::parse(addr) {
        Ok(parsed) => parsed,
        Err(_) => {
            color_eyre::eyre::bail!(
                "invalid remote address `{addr}`; expected `ws://host:port`, `wss://host:port`, `unix://`, or `unix://PATH`"
            );
        }
    };
    if matches!(parsed.scheme(), "ws" | "wss")
        && parsed.host_str().is_some()
        && remote_addr_has_explicit_port(addr, &parsed)
        && parsed.path() == "/"
        && parsed.query().is_none()
        && parsed.fragment().is_none()
    {
        return Ok(RemoteAppServerEndpoint::WebSocket {
            websocket_url: parsed.to_string(),
            auth_token: None,
        });
    }

    color_eyre::eyre::bail!(
        "invalid remote address `{addr}`; expected `ws://host:port`, `wss://host:port`, `unix://`, or `unix://PATH`"
    );
}

pub fn remote_addr_supports_auth_token(endpoint: &RemoteAppServerEndpoint) -> bool {
    match endpoint {
        RemoteAppServerEndpoint::WebSocket { websocket_url, .. } => {
            Url::parse(websocket_url).is_ok_and(|parsed| websocket_url_supports_auth_token(&parsed))
        }
        RemoteAppServerEndpoint::UnixSocket { .. } => false,
    }
}

async fn connect_remote_app_server(
    endpoint: RemoteAppServerEndpoint,
) -> color_eyre::Result<AppServerClient> {
    let app_server = RemoteAppServerClient::connect(RemoteAppServerConnectArgs {
        endpoint,
        client_name: "genex-tui".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await
    .wrap_err("failed to connect to remote app server")?;
    Ok(AppServerClient::Remote(app_server))
}

async fn maybe_probe_default_daemon_socket(genex_home: &Path) -> Option<AbsolutePathBuf> {
    let socket_path = genex_app_server_client::app_server_control_socket_path(genex_home).ok()?;
    match tokio::time::timeout(
        AUTO_CONNECT_DAEMON_CONNECT_TIMEOUT,
        genex_uds::UnixStream::connect(socket_path.as_path()),
    )
    .await
    {
        Ok(Ok(_stream)) => Some(socket_path),
        Ok(Err(err)) => {
            tracing::debug!(%err, socket_path = %socket_path.display(), "skipping default app-server daemon socket");
            None
        }
        Err(_) => {
            tracing::debug!(
                socket_path = %socket_path.display(),
                timeout_ms = AUTO_CONNECT_DAEMON_CONNECT_TIMEOUT.as_millis(),
                "timed out probing default app-server daemon socket"
            );
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn start_app_server(
    target: &mut AppServerTarget,
    arg0_paths: Arg0DispatchPaths,
    config: Config,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    log_db: Option<log_db::LogDbLayer>,
    state_db: &mut Option<StateDbHandle>,
    environment_manager: Arc<EnvironmentManager>,
    provider_factory: Option<ProviderRuntimeFactory>,
) -> color_eyre::Result<AppServerClient> {
    let connection = if matches!(target, AppServerTarget::Embedded) {
        None
    } else {
        Some(app_server_connection::connect(target).await)
    };
    if let Some(connection) = connection {
        match connection {
            Ok(app_server) => return Ok(app_server),
            Err(err)
                if matches!(
                    target,
                    AppServerTarget::LocalDaemon {
                        allow_embedded_fallback: true,
                        ..
                    }
                ) =>
            {
                tracing::debug!(%err, "local daemon connection failed; starting embedded app server");
                *target = AppServerTarget::Embedded;
                *state_db = init_state_db_for_app_server_target(&config, target).await?;
            }
            Err(err) if matches!(target, AppServerTarget::LocalDaemon { .. }) => {
                return Err(color_eyre::eyre::eyre!(
                    "{err:#}\n{}",
                    daemon_startup::FAILURE_HINT
                ));
            }
            Err(err) => return Err(err),
        }
    }
    start_embedded_app_server(
        arg0_paths,
        config,
        cli_kv_overrides,
        loader_overrides,
        strict_config,
        log_db,
        state_db.clone(),
        environment_manager,
        provider_factory,
    )
    .await
    .map(AppServerClient::InProcess)
}

pub(crate) async fn start_app_server_for_picker(
    config: &Config,
    target: &AppServerTarget,
    state_db: Option<StateDbHandle>,
    environment_manager: Arc<EnvironmentManager>,
) -> color_eyre::Result<AppServerSession> {
    let mut target = target.clone();
    let mut state_db = state_db;
    let app_server = start_app_server(
        &mut target,
        Arg0DispatchPaths::default(),
        config.clone(),
        Vec::new(),
        LoaderOverrides::default(),
        /*strict_config*/ false,
        /*log_db*/ None,
        &mut state_db,
        environment_manager,
        None,
    )
    .await?;
    Ok(
        AppServerSession::new(app_server, target.thread_params_mode())
            .with_local_genex_home(&config.genex_home),
    )
}

#[cfg(test)]
pub(crate) async fn start_embedded_app_server_for_picker(
    config: &Config,
) -> color_eyre::Result<AppServerSession> {
    let state_db = init_state_db_for_app_server_target(config, &AppServerTarget::Embedded).await?;
    start_app_server_for_picker(
        config,
        &AppServerTarget::Embedded,
        state_db,
        Arc::new(EnvironmentManager::default_for_tests()),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn start_embedded_app_server_with<F, Fut>(
    arg0_paths: Arg0DispatchPaths,
    config: Config,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    log_db: Option<log_db::LogDbLayer>,
    state_db: Option<StateDbHandle>,
    environment_manager: Arc<EnvironmentManager>,
    provider_factory: Option<ProviderRuntimeFactory>,
    start_client: F,
) -> color_eyre::Result<InProcessAppServerClient>
where
    F: FnOnce(InProcessClientStartArgs) -> Fut,
    Fut: Future<Output = std::io::Result<InProcessAppServerClient>>,
{
    let config_warnings = config
        .startup_warnings
        .iter()
        .map(|warning| ConfigWarningNotification {
            summary: warning.clone(),
            details: None,
            path: None,
            range: None,
        })
        .collect();
    let runtime = if let Some(provider_factory) = provider_factory {
        let init = provider_init_from_config_with_descriptor(&config, false, &provider_factory.1)
            .await
            .map_err(|err| {
                color_eyre::eyre::eyre!("failed to prepare embedded app-server runtime: {err}")
            })?;
        (provider_factory.0)(init)
    } else {
        configured_app_server_runtime_from_config(&config, false)
            .await
            .map_err(|err| {
                color_eyre::eyre::eyre!("failed to prepare embedded app-server runtime: {err}")
            })?
    };
    let client = start_client(InProcessClientStartArgs {
        arg0_paths,
        config: Arc::new(config),
        cli_overrides: cli_kv_overrides,
        loader_overrides,
        strict_config,
        log_db,
        state_db,
        environment_manager,
        config_warnings,
        session_source: serde_json::from_value(serde_json::json!("cli"))
            .unwrap_or_else(|err| panic!("cli session source should deserialize: {err}")),
        enable_api_key_env: false,
        runtime: Some(runtime),
        client_name: "genex-tui".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await
    .wrap_err("failed to start embedded app server")?;
    Ok(client)
}

async fn shutdown_app_server_if_present(app_server: Option<AppServerSession>) {
    if let Some(app_server) = app_server
        && let Err(err) = app_server.shutdown().await
    {
        warn!(%err, "Failed to shut down temporary embedded app server");
    }
}

/// Shut down the startup app server before restoring its terminal and ending its session.
async fn shutdown_startup_session(
    app_server: Option<AppServerSession>,
    terminal_restore_guard: &mut TerminalRestoreGuard,
) {
    shutdown_app_server_if_present(app_server).await;
    terminal_restore_guard.restore_silently();
    session_log::log_session_end();
}

fn session_target_from_app_server_thread(
    thread: AppServerThread,
) -> Option<resume_picker::SessionTarget> {
    match ThreadId::from_string(&thread.id) {
        Ok(thread_id) => Some(resume_picker::SessionTarget {
            path: thread.path,
            thread_id,
            cwd: Some(thread.cwd.to_path_buf()),
            history_mode: Some(thread.history_mode),
        }),
        Err(err) => {
            warn!(
                thread_id = thread.id,
                %err,
                "Ignoring app-server thread with invalid thread id during TUI session lookup"
            );
            None
        }
    }
}

pub(crate) fn resume_source_kinds(include_non_interactive: bool) -> Vec<ThreadSourceKind> {
    let mut source_kinds = vec![ThreadSourceKind::Cli, ThreadSourceKind::VsCode];
    if include_non_interactive {
        // `thread/list` treats omitted and empty `sourceKinds` as interactive-only,
        // so include-non-interactive has to name the user-resumable non-interactive
        // sources explicitly until the API grows an unfiltered request.
        source_kinds.extend([ThreadSourceKind::Exec, ThreadSourceKind::AppServer]);
    }
    source_kinds
}

async fn lookup_session_target_with_app_server(
    app_server: &mut AppServerSession,
    config: &Config,
    id_or_name: &str,
) -> color_eyre::Result<Option<resume_picker::SessionTarget>> {
    if Uuid::parse_str(id_or_name).is_ok() {
        let thread_id = match ThreadId::from_string(id_or_name) {
            Ok(thread_id) => thread_id,
            Err(err) => {
                warn!(
                    session = id_or_name,
                    %err,
                    "Failed to parse session id during TUI lookup"
                );
                return Ok(None);
            }
        };
        return match app_server
            .thread_read(thread_id, /*include_turns*/ false)
            .await
        {
            Ok(thread) => Ok(session_target_from_app_server_thread(thread)),
            Err(err) => {
                warn!(
                    session = id_or_name,
                    %err,
                    "thread/read failed during TUI session lookup"
                );
                Ok(None)
            }
        };
    }

    Ok(named_session_lookup::lookup(
        app_server,
        config.genex_home.as_path(),
        id_or_name,
        &[named_session_lookup::SessionCollection::Active],
        &[resume_source_kinds(/*include_non_interactive*/ false)],
    )
    .await?
    .and_then(session_target_from_app_server_thread))
}

async fn lookup_latest_session_target_with_app_server(
    uses_remote_filesystem: bool,
    app_server: &mut AppServerSession,
    config: &Config,
    cwd_filter: Option<&Path>,
    include_non_interactive: bool,
) -> color_eyre::Result<Option<resume_picker::SessionTarget>> {
    let uses_remote_workspace = app_server.uses_remote_workspace();
    for lookup_mode in [
        LatestSessionLookupMode::StateDbOnly,
        LatestSessionLookupMode::ScanAndRepair,
    ] {
        let response = app_server
            .thread_list(latest_session_lookup_params(
                uses_remote_filesystem,
                config,
                cwd_filter,
                include_non_interactive,
                lookup_mode,
            ))
            .await?;
        let target = response
            .data
            .into_iter()
            .find_map(session_target_from_app_server_thread);
        if target.as_ref().is_some_and(|target| {
            uses_remote_workspace || target.path.as_deref().is_some_and(std::path::Path::exists)
        }) {
            return Ok(target);
        }
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LatestSessionLookupMode {
    StateDbOnly,
    ScanAndRepair,
}

fn latest_session_lookup_params(
    uses_remote_filesystem: bool,
    config: &Config,
    cwd_filter: Option<&Path>,
    include_non_interactive: bool,
    lookup_mode: LatestSessionLookupMode,
) -> ThreadListParams {
    ThreadListParams {
        originators: None,
        cursor: None,
        limit: Some(1),
        sort_key: Some(AppServerThreadSortKey::UpdatedAt),
        sort_direction: None,
        source_kinds: Some(resume_source_kinds(include_non_interactive)),
        archived: Some(false),
        section_id: None,
        project_id: None,
        parent_thread_id: None,
        ancestor_thread_id: None,
        cwd: cwd_filter.map(|cwd| {
            resume_picker::repository_cwd_filter(
                cwd,
                uses_remote_filesystem,
                config.features.enabled(genex_features::Feature::Worktrees),
            )
        }),
        use_state_db_only: match lookup_mode {
            LatestSessionLookupMode::StateDbOnly => true,
            LatestSessionLookupMode::ScanAndRepair => false,
        },
        search_term: None,
    }
}

fn config_cwd_for_app_server_target(
    cwd: Option<&Path>,
    app_server_target: &AppServerTarget,
    default_environment_is_remote: bool,
) -> std::io::Result<Option<AbsolutePathBuf>> {
    if app_server_target.uses_remote_workspace() || default_environment_is_remote {
        return Ok(None);
    }

    let cwd = match cwd {
        Some(path) => {
            AbsolutePathBuf::from_absolute_path(canonicalize_existing_preserving_symlinks(path)?)
        }
        None => AbsolutePathBuf::current_dir(),
    }?;
    Ok(Some(cwd))
}

fn uses_remote_workspace_or_environment(
    app_server_target: &AppServerTarget,
    environment_manager: &EnvironmentManager,
) -> bool {
    app_server_target.uses_remote_workspace()
        || environment_manager
            .default_environment()
            .is_some_and(|environment| environment.is_remote())
}

async fn resolve_startup_resume_or_fork_cwd(
    tui: &mut Tui,
    config: &Config,
    app_server: Option<&mut AppServerSession>,
    session_selection: &resume_picker::SessionSelection,
    cwd_override: Option<&Path>,
    uses_remote_workspace: bool,
    uses_remote_workspace_or_environment: bool,
) -> color_eyre::Result<ResolveCwdOutcome> {
    let Some((action, target_session)) = (match session_selection {
        resume_picker::SessionSelection::Resume(target_session) => {
            Some((CwdPromptAction::Resume, target_session))
        }
        resume_picker::SessionSelection::Fork(target_session) => {
            Some((CwdPromptAction::Fork, target_session))
        }
        _ => None,
    }) else {
        return Ok(ResolveCwdOutcome::Continue(None));
    };
    let local_settings = crate::local_settings::LocalSettings::from(config);
    let resume_cwd_mode = effective_resume_cwd_mode(local_settings.tui.resume_cwd, cwd_override);
    if uses_remote_workspace_or_environment
        && cwd_override.is_none()
        && matches!(resume_cwd_mode, Some(ResumeCwdMode::Current))
    {
        color_eyre::eyre::bail!(
            "`tui.resume_cwd = \"current\"` requires `--cd` when using a remote workspace"
        );
    }
    if uses_remote_workspace {
        return Ok(ResolveCwdOutcome::Continue(Some(config.cwd.to_path_buf())));
    }

    let history_cwd = if matches!(resume_cwd_mode, Some(ResumeCwdMode::Current)) {
        None
    } else if let Some(cwd) = &target_session.cwd {
        Some(cwd.clone())
    } else if let Some(app_server) = app_server {
        read_session_cwd(app_server, target_session.thread_id).await
    } else {
        None
    };
    resolve_cwd_for_resume_or_fork(
        tui,
        config,
        history_cwd,
        action,
        ResumeCwdContext {
            current_cwd: config.cwd.as_path(),
            remembered_current_cwd: config.cwd.as_path(),
            allow_remember_current: !uses_remote_workspace_or_environment || cwd_override.is_some(),
            mode: resume_cwd_mode,
        },
    )
    .await
}

fn should_load_configured_environments(
    loader_overrides: &LoaderOverrides,
    app_server_target: &AppServerTarget,
) -> bool {
    !loader_overrides.ignore_user_config && !app_server_target.uses_remote_workspace()
}

fn latest_session_cwd_filter<'a>(
    uses_remote_workspace: bool,
    remote_cwd_override: Option<&'a Path>,
    config: &'a Config,
    show_all: bool,
) -> Option<&'a Path> {
    if show_all {
        return None;
    }

    if uses_remote_workspace {
        remote_cwd_override
    } else {
        Some(config.cwd.as_path())
    }
}

fn app_server_target_for_launch(
    explicit_remote_endpoint: Option<RemoteAppServerEndpoint>,
    default_daemon_socket: Option<AbsolutePathBuf>,
    can_reuse_implicit_local_daemon: bool,
) -> std::io::Result<AppServerTarget> {
    Ok(match explicit_remote_endpoint {
        Some(endpoint) => AppServerTarget::Remote { endpoint },
        None if can_reuse_implicit_local_daemon => {
            default_daemon_socket.map_or(AppServerTarget::Embedded, |socket_path| {
                AppServerTarget::LocalDaemon {
                    allow_embedded_fallback: true,
                    endpoint: RemoteAppServerEndpoint::UnixSocket { socket_path },
                }
            })
        }
        None => AppServerTarget::Embedded,
    })
}

fn loader_overrides_are_default(loader_overrides: &LoaderOverrides) -> bool {
    let loader_overrides_are_default = loader_overrides.user_config_path.is_none()
        && loader_overrides.user_config_profile.is_none()
        && loader_overrides.system_config_path.is_none()
        && loader_overrides.system_requirements_path.is_none()
        && !loader_overrides.ignore_managed_requirements
        && !loader_overrides.ignore_user_config
        && !loader_overrides.ignore_user_and_project_exec_policy_rules
        && loader_overrides
            .macos_managed_config_requirements_base64
            .is_none();
    #[cfg(target_os = "macos")]
    let loader_overrides_are_default =
        loader_overrides_are_default && loader_overrides.managed_preferences_base64.is_none();
    loader_overrides_are_default
}

/// Restore terminal modes before a fatal startup exit bypasses destructor cleanup.
fn restore_terminal_before_fatal_exit() {
    if crossterm::terminal::is_raw_mode_enabled().unwrap_or(false) {
        let _ = tui::restore_after_exit();
    }
}

pub async fn run_main(
    cli: Cli,
    arg0_paths: Arg0DispatchPaths,
    loader_overrides: LoaderOverrides,
    explicit_remote_endpoint: Option<RemoteAppServerEndpoint>,
) -> std::io::Result<AppExitInfo> {
    system_motion::initialize().await;
    // Keep the startup future out of the CLI caller's frame while the TUI is running.
    match Box::pin(startup_orchestration::run_main_inner(
        cli,
        arg0_paths,
        loader_overrides,
        explicit_remote_endpoint,
        None,
        None,
    ))
    .await
    {
        Err(err) if startup_draft::StartupCancelled::matches(&err) => Ok(AppExitInfo {
            token_usage: TokenUsage::default(),
            thread_id: None,
            resume_hint: None,
            disconnect_info: None,
            exit_reason: ExitReason::UserRequested,
        }),
        result => result,
    }
}

/// Runs the TUI with a provider type owned by the embedding host.
///
/// The same host executable is used for the default local daemon. The daemon
/// constructs `P` in its own process; embedded mode constructs it in-process.
pub async fn run_main_with_provider<P: ModelProviderFactory>(
    cli: Cli,
    arg0_paths: Arg0DispatchPaths,
    loader_overrides: LoaderOverrides,
    explicit_remote_endpoint: Option<RemoteAppServerEndpoint>,
) -> std::io::Result<AppExitInfo> {
    system_motion::initialize().await;
    match Box::pin(startup_orchestration::run_main_inner(
        cli,
        arg0_paths,
        loader_overrides,
        explicit_remote_endpoint,
        Some((
            AppServerRuntimeBuilder::build_from_provider_factory::<P>,
            P::provider_descriptor(),
        )),
        Some(std::env::current_exe().map_err(std::io::Error::other)?),
    ))
    .await
    {
        Err(err) if startup_draft::StartupCancelled::matches(&err) => Ok(AppExitInfo {
            token_usage: TokenUsage::default(),
            thread_id: None,
            resume_hint: None,
            disconnect_info: None,
            exit_reason: ExitReason::UserRequested,
        }),
        result => result,
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_ratatui_app(
    mut cli: Cli,
    arg0_paths: Arg0DispatchPaths,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    mut app_server_target: AppServerTarget,
    remote_cwd_override: Option<PathBuf>,
    initial_config: Config,
    overrides: ConfigOverrides,
    cli_kv_overrides: Vec<(String, toml::Value)>,
    log_db: Option<log_db::LogDbLayer>,
    mut state_db: Option<StateDbHandle>,
    environment_manager: Arc<EnvironmentManager>,
    managed_worktree: Option<ManagedTuiWorktree>,
    daemon_startup_warning: Option<String>,
    launch_telemetry: daemon_telemetry::Launch<impl FnOnce(&AppServerTarget, bool)>,
    startup_draft: startup_draft::StartupDraft,
    provider_factory: Option<ProviderRuntimeFactory>,
) -> color_eyre::Result<AppExitInfo> {
    let uses_remote_workspace = app_server_target.uses_remote_workspace();
    color_eyre::install()?;

    // Forward panic reports through tracing so they appear in the UI status
    // line, but do not swallow the default/color-eyre panic handler.
    // Chain to the previous hook so users still get a rich panic report
    // (including backtraces) after we restore the terminal.
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = tui::restore_after_exit();
        tracing::error!("panic: {info}");
        prev_hook(info);
    }));
    let (mut tui, mut terminal_restore_guard, mut startup_draft) = startup_draft.into_parts();

    // Initialize high-fidelity session event logging if enabled.
    session_log::maybe_init(&initial_config);

    let startup_app_server = startup_draft
        .run_until(
            &mut tui,
            start_app_server(
                &mut app_server_target,
                arg0_paths.clone(),
                initial_config.clone(),
                cli_kv_overrides.clone(),
                loader_overrides.clone(),
                strict_config,
                log_db.clone(),
                &mut state_db,
                environment_manager.clone(),
                provider_factory.clone(),
            ),
        )
        .await;
    launch_telemetry.record(&app_server_target, matches!(&startup_app_server, Ok(Ok(_))));
    let app_server_session = match startup_app_server {
        Ok(Ok(app_server)) => {
            AppServerSession::new(app_server, app_server_target.thread_params_mode())
                .with_local_genex_home(&initial_config.genex_home)
        }
        Ok(Err(err)) => {
            terminal_restore_guard.restore_silently();
            session_log::log_session_end();
            return Err(err);
        }
        Err(err) => {
            terminal_restore_guard.restore_silently();
            session_log::log_session_end();
            return Err(err.into());
        }
    }
    .with_remote_cwd_override(remote_cwd_override.clone());
    let mut app_server = Some(app_server_session);
    // Folder consent runs after the picker resolves the actual destination.
    let should_show_trust_screen_flag = false;

    let provider = startup_draft
        .run_until(
            &mut tui,
            app_server
                .as_mut()
                .expect("app server should exist before model provider status")
                .model_provider_read(),
        )
        .await;
    let provider = match provider {
        Ok(Ok(provider)) => provider,
        Ok(Err(err)) => {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            return Err(err);
        }
        Err(err) => {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            return Err(err.into());
        }
    };
    let provider_auth_fields = provider.auth_fields;
    let initial_model_provider_id = initial_config.model_provider_id.clone();

    let Some(active_app_server) = app_server.as_mut() else {
        unreachable!("app server should exist when auth is required");
    };
    let login_status = startup_draft
        .run_until(&mut tui, get_login_status(active_app_server))
        .await;
    let (login_status, mut startup_account) = match login_status {
        Ok(Ok((login_status, account))) => (login_status, Some(account)),
        Ok(Err(err)) => {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            return Err(err);
        }
        Err(err) => {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            return Err(err.into());
        }
    };
    let should_show_onboarding =
        should_show_onboarding(login_status, should_show_trust_screen_flag);

    let config = if should_show_onboarding {
        if let Err(err) = startup_draft.flush_pending_events(&mut tui).await {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            return Err(err.into());
        }
        // Authentication can change while any interactive onboarding screen is open.
        startup_account = None;
        let show_login_screen = should_show_login_screen(login_status);
        let onboarding_result = run_onboarding_app(
            OnboardingScreenArgs {
                show_login_screen,
                show_trust_screen: should_show_trust_screen_flag,
                remote_project_trust: None,
                login_status,
                app_server_request_handle: app_server
                    .as_ref()
                    .map(AppServerSession::request_handle),
                provider_auth_fields: provider_auth_fields.clone(),
                config: initial_config.clone(),
            },
            if show_login_screen {
                app_server.as_mut()
            } else {
                None
            },
            &mut tui,
        )
        .await;
        let onboarding_result = match onboarding_result {
            Ok(onboarding_result) => onboarding_result,
            Err(err) => {
                shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
                return Err(err);
            }
        };
        if onboarding_result.should_exit {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            let _ = tui.terminal.clear();
            return Ok(AppExitInfo {
                token_usage: crate::token_usage::TokenUsage::default(),
                thread_id: None,
                resume_hint: None,
                disconnect_info: None,
                exit_reason: ExitReason::UserRequested,
            });
        }
        let reloaded_config = startup_draft
            .run_until(&mut tui, async {
                // Reload config when persisted trust or auth changes alter the current process.
                Ok::<_, std::io::Error>(
                    if !uses_remote_workspace
                        && (onboarding_result.directory_trust_persisted || show_login_screen)
                    {
                        load_config_or_exit_with_fallback_cwd(
                            cli_kv_overrides.clone(),
                            overrides.clone(),
                            loader_overrides.clone(),
                            strict_config,
                            /*fallback_cwd*/ None,
                            managed_worktree.as_ref(),
                        )
                        .await
                    } else {
                        initial_config
                    },
                )
            })
            .await;
        match reloaded_config {
            Ok(Ok(config)) => config,
            Ok(Err(err)) | Err(err) => {
                shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
                return Err(err.into());
            }
        }
    } else {
        initial_config
    };
    startup_draft.apply_config(&config);
    if !(cli.resume_picker || cli.fork_picker || cli.agents_overview)
        && let Err(err) = startup_draft.show(&mut tui)
    {
        shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
        return Err(err.into());
    }

    let missing_session_exit =
        |id_str: &str,
         action: &str,
         tui: &mut Tui,
         terminal_restore_guard: &mut TerminalRestoreGuard| {
            error!("Error finding conversation path: {id_str}");
            terminal_restore_guard.restore_silently();
            session_log::log_session_end();
            let _ = tui.terminal.clear();
            Ok(AppExitInfo {
                token_usage: crate::token_usage::TokenUsage::default(),
                thread_id: None,
                resume_hint: None,
                disconnect_info: None,
                exit_reason: ExitReason::Fatal(format!(
                    "No saved session found with ID {id_str}. Run `genex {action}` without an ID to choose from existing sessions."
                )),
            })
        };

    // Startup pickers need the current theme before selection can reload config.
    // Leave the one-time override initialization below to use the final config.
    if (cli.resume_picker || cli.fork_picker)
        && let Some(name) = config.tui_theme.as_deref()
        && let Some(theme) =
            crate::render::highlight::resolve_theme_by_name(name, Some(config.genex_home.as_path()))
    {
        crate::render::highlight::set_syntax_theme(theme);
    }

    let use_fork = cli.fork_picker || cli.fork_last || cli.fork_session_id.is_some();
    let mut session_selection = if cli.agents_overview {
        resume_picker::SessionSelection::AgentsOverview
    } else if use_fork {
        if let Some(id_str) = cli.fork_session_id.as_deref() {
            let Some(startup_app_server) = app_server.as_mut() else {
                unreachable!("app server should be initialized for --fork <id>");
            };
            let lookup = startup_draft
                .run_until(
                    &mut tui,
                    lookup_session_target_with_app_server(startup_app_server, &config, id_str),
                )
                .await;
            let target_session = match lookup {
                Ok(result) => result?,
                Err(err) => {
                    shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
                    return Err(err.into());
                }
            };
            match target_session {
                Some(target_session) => resume_picker::SessionSelection::Fork(target_session),
                None => {
                    shutdown_app_server_if_present(app_server.take()).await;
                    return missing_session_exit(
                        id_str,
                        "fork",
                        &mut tui,
                        &mut terminal_restore_guard,
                    );
                }
            }
        } else if cli.fork_last {
            let filter_cwd = latest_session_cwd_filter(
                uses_remote_workspace,
                remote_cwd_override.as_deref(),
                &config,
                cli.fork_show_all,
            );
            let Some(startup_app_server) = app_server.as_mut() else {
                unreachable!("app server should be initialized for --fork --last");
            };
            let lookup = startup_draft
                .run_until(
                    &mut tui,
                    lookup_latest_session_target_with_app_server(
                        uses_remote_workspace_or_environment(
                            &app_server_target,
                            &environment_manager,
                        ),
                        startup_app_server,
                        &config,
                        filter_cwd,
                        /*include_non_interactive*/ false,
                    ),
                )
                .await;
            let target_session = match lookup {
                Ok(result) => result?,
                Err(err) => {
                    shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
                    return Err(err.into());
                }
            };
            match target_session {
                Some(target_session) => resume_picker::SessionSelection::Fork(target_session),
                None => resume_picker::SessionSelection::StartFresh,
            }
        } else if cli.fork_picker {
            if let Err(err) = startup_draft.flush_pending_events(&mut tui).await {
                shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
                return Err(err.into());
            }
            let Some(app_server) = app_server.take() else {
                unreachable!("app server should be initialized for --fork picker");
            };
            match resume_picker::run_fork_picker_with_app_server(
                uses_remote_workspace_or_environment(&app_server_target, &environment_manager),
                &mut tui,
                &config,
                &crate::local_settings::LocalSettings::from(&config),
                cli.fork_show_all,
                app_server,
            )
            .await?
            {
                resume_picker::SessionSelection::Exit => {
                    terminal_restore_guard.restore_silently();
                    session_log::log_session_end();
                    return Ok(AppExitInfo {
                        token_usage: crate::token_usage::TokenUsage::default(),
                        thread_id: None,
                        resume_hint: None,
                        disconnect_info: None,
                        exit_reason: ExitReason::UserRequested,
                    });
                }
                other => other,
            }
        } else {
            resume_picker::SessionSelection::StartFresh
        }
    } else if let Some(id_str) = cli.resume_session_id.as_deref() {
        let Some(startup_app_server) = app_server.as_mut() else {
            unreachable!("app server should be initialized for --resume <id>");
        };
        let lookup = startup_draft
            .run_until(
                &mut tui,
                lookup_session_target_with_app_server(startup_app_server, &config, id_str),
            )
            .await;
        let target_session = match lookup {
            Ok(result) => result?,
            Err(err) => {
                shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
                return Err(err.into());
            }
        };
        match target_session {
            Some(target_session) => resume_picker::SessionSelection::Resume(target_session),
            None => {
                shutdown_app_server_if_present(app_server.take()).await;
                return missing_session_exit(
                    id_str,
                    "resume",
                    &mut tui,
                    &mut terminal_restore_guard,
                );
            }
        }
    } else if cli.resume_last {
        let filter_cwd = latest_session_cwd_filter(
            uses_remote_workspace,
            remote_cwd_override.as_deref(),
            &config,
            cli.resume_show_all,
        );
        let Some(startup_app_server) = app_server.as_mut() else {
            unreachable!("app server should be initialized for --resume --last");
        };
        let lookup = startup_draft
            .run_until(
                &mut tui,
                lookup_latest_session_target_with_app_server(
                    uses_remote_workspace_or_environment(&app_server_target, &environment_manager),
                    startup_app_server,
                    &config,
                    filter_cwd,
                    cli.resume_include_non_interactive,
                ),
            )
            .await;
        let target_session = match lookup {
            Ok(result) => result?,
            Err(err) => {
                shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
                return Err(err.into());
            }
        };
        match target_session {
            Some(target_session) => resume_picker::SessionSelection::Resume(target_session),
            None => resume_picker::SessionSelection::StartFresh,
        }
    } else if cli.resume_picker {
        if let Err(err) = startup_draft.flush_pending_events(&mut tui).await {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            return Err(err.into());
        }
        let Some(app_server) = app_server.take() else {
            unreachable!("app server should be initialized for --resume picker");
        };
        match resume_picker::run_resume_picker_with_app_server(
            uses_remote_workspace_or_environment(&app_server_target, &environment_manager),
            &mut tui,
            &config,
            &crate::local_settings::LocalSettings::from(&config),
            cli.resume_show_all,
            cli.resume_include_non_interactive,
            app_server,
        )
        .await?
        {
            resume_picker::SessionSelection::Exit => {
                terminal_restore_guard.restore_silently();
                session_log::log_session_end();
                return Ok(AppExitInfo {
                    token_usage: crate::token_usage::TokenUsage::default(),
                    thread_id: None,
                    resume_hint: None,
                    disconnect_info: None,
                    exit_reason: ExitReason::UserRequested,
                });
            }
            other => other,
        }
    } else {
        resume_picker::SessionSelection::StartFresh
    };

    if let Err(err) = startup_draft.update_session_selection(&mut tui, &session_selection) {
        shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
        return Err(err.into());
    }

    if matches!(
        &session_selection,
        resume_picker::SessionSelection::Resume(_) | resume_picker::SessionSelection::Fork(_)
    ) && let Err(err) = startup_draft.flush_pending_events(&mut tui).await
    {
        shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
        return Err(err.into());
    }

    let current_cwd = config.cwd.clone();
    let fallback_cwd = match resolve_startup_resume_or_fork_cwd(
        &mut tui,
        &config,
        app_server.as_mut(),
        &session_selection,
        cli.cwd.as_deref(),
        uses_remote_workspace,
        uses_remote_workspace_or_environment(&app_server_target, &environment_manager),
    )
    .await
    {
        Ok(ResolveCwdOutcome::Continue(cwd)) => cwd,
        Ok(ResolveCwdOutcome::ContinueAfterPrompt(cwd)) => {
            // Another daemon client can change authentication while this prompt is open.
            startup_account = None;
            Some(cwd)
        }
        Ok(ResolveCwdOutcome::Exit) => {
            terminal_restore_guard.restore_silently();
            session_log::log_session_end();
            return Ok(AppExitInfo {
                token_usage: crate::token_usage::TokenUsage::default(),
                thread_id: None,
                resume_hint: None,
                disconnect_info: None,
                exit_reason: ExitReason::UserRequested,
            });
        }
        Err(err) => {
            terminal_restore_guard.restore_silently();
            session_log::log_session_end();
            return Err(err);
        }
    };

    if (cli.resume_picker || cli.fork_picker)
        && let Err(err) = startup_draft.show(&mut tui)
    {
        shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
        return Err(err.into());
    }

    let picker_cancelled_without_selection = matches!(
        session_selection,
        resume_picker::SessionSelection::StartFresh
    ) && (cli.resume_picker || cli.fork_picker);

    let reloaded_config = match &session_selection {
        resume_picker::SessionSelection::Resume(_) | resume_picker::SessionSelection::Fork(_) => {
            startup_draft
                .run_until(
                    &mut tui,
                    load_config_or_exit_with_fallback_cwd(
                        cli_kv_overrides.clone(),
                        overrides.clone(),
                        loader_overrides.clone(),
                        strict_config,
                        fallback_cwd,
                        managed_worktree.as_ref(),
                    ),
                )
                .await
        }
        resume_picker::SessionSelection::StartFresh if picker_cancelled_without_selection => {
            startup_draft
                .run_until(
                    &mut tui,
                    load_config_or_exit(
                        cli_kv_overrides.clone(),
                        overrides.clone(),
                        loader_overrides.clone(),
                        strict_config,
                    ),
                )
                .await
        }
        _ => Ok(config),
    };
    let mut config = match reloaded_config {
        Ok(config) => config,
        Err(err) => {
            shutdown_startup_session(app_server.take(), &mut terminal_restore_guard).await;
            return Err(err.into());
        }
    };
    startup_draft.apply_config(&config);

    if matches!(app_server_target, AppServerTarget::Embedded)
        && config.model_provider_id != initial_model_provider_id
    {
        startup_account = None;
        if matches!(&app_server_target, AppServerTarget::Embedded) {
            // App-server providers are fixed at startup, so onboarding cannot
            // reuse a server initialized before it persisted another provider.
            shutdown_app_server_if_present(app_server.take()).await;
        }
    }
    let mut app_server = match app_server {
        Some(app_server) => app_server,
        None => match startup_draft
            .run_until(
                &mut tui,
                start_app_server(
                    &mut app_server_target,
                    arg0_paths.clone(),
                    config.clone(),
                    cli_kv_overrides.clone(),
                    loader_overrides.clone(),
                    strict_config,
                    log_db.clone(),
                    &mut state_db,
                    environment_manager.clone(),
                    provider_factory.clone(),
                ),
            )
            .await
        {
            Ok(Ok(app_server)) => {
                // A picker can replace the server; account reads belong to their original session.
                startup_account = None;
                AppServerSession::new(app_server, app_server_target.thread_params_mode())
                    .with_local_genex_home(&config.genex_home)
                    .with_remote_cwd_override(remote_cwd_override.clone())
            }
            Ok(Err(err)) => {
                terminal_restore_guard.restore_silently();
                session_log::log_session_end();
                return Err(err);
            }
            Err(err) => {
                terminal_restore_guard.restore_silently();
                session_log::log_session_end();
                return Err(err.into());
            }
        },
    };

    // Remote startup keeps its existing explicit --cd trust check. Resolving other
    // remote folders requires authoritative project-root information from the server.
    if !uses_remote_workspace || remote_cwd_override.is_some() {
        let resumed_thread = if matches!(app_server_target, AppServerTarget::LocalDaemon { .. })
            && let resume_picker::SessionSelection::Resume(target) = &session_selection
        {
            Some(
                startup_draft
                    .run_until(
                        &mut tui,
                        app_server.thread_read(target.thread_id, /*include_turns*/ false),
                    )
                    .await??,
            )
        } else {
            None
        };
        let trust_cwd = remote_cwd_override
            .as_deref()
            .unwrap_or(config.cwd.as_path());
        let consent = onboarding::onboarding_screen::check_directory_trust(
            &mut tui,
            &app_server,
            &config,
            &app_server_target,
            trust_cwd,
            resumed_thread.as_ref(),
            Some(&mut startup_draft),
        )
        .await?;
        startup_account = None;
        if consent.directory_trust_persisted && !uses_remote_workspace {
            let previous_provider = config.model_provider_id.clone();
            config = load_config_or_exit_with_fallback_cwd(
                cli_kv_overrides.clone(),
                overrides.clone(),
                loader_overrides.clone(),
                strict_config,
                Some(config.cwd.to_path_buf()),
                managed_worktree.as_ref(),
            )
            .await;
            if config.model_provider_id != previous_provider
                && matches!(app_server_target, AppServerTarget::Embedded)
            {
                app_server.shutdown().await?;
                let client = start_app_server(
                    &mut app_server_target,
                    arg0_paths.clone(),
                    config.clone(),
                    cli_kv_overrides.clone(),
                    loader_overrides.clone(),
                    strict_config,
                    log_db.clone(),
                    &mut state_db,
                    environment_manager.clone(),
                    provider_factory.clone(),
                )
                .await?;
                app_server = AppServerSession::new(client, app_server_target.thread_params_mode())
                    .with_local_genex_home(&config.genex_home);
            }
        } else if consent.should_exit {
            if matches!(app_server_target, AppServerTarget::Embedded) {
                shutdown_startup_session(Some(app_server), &mut terminal_restore_guard).await;
                return Ok(AppExitInfo {
                    token_usage: crate::token_usage::TokenUsage::default(),
                    thread_id: None,
                    resume_hint: None,
                    disconnect_info: None,
                    exit_reason: ExitReason::UserRequested,
                });
            }
            if !uses_remote_workspace {
                config = load_config_or_exit_with_fallback_cwd(
                    cli_kv_overrides.clone(),
                    overrides.clone(),
                    loader_overrides.clone(),
                    strict_config,
                    Some(current_cwd.to_path_buf()),
                    managed_worktree.as_ref(),
                )
                .await;
            }
            session_selection = resume_picker::SessionSelection::AgentsOverview;
            cli.prompt = None;
            cli.images.clear();
            startup_draft.update_session_selection(&mut tui, &session_selection)?;
        }
    }
    startup_draft.apply_config(&config);

    let local_settings = crate::local_settings::LocalSettings::from(&config);
    // Configure syntax highlighting theme from the final config — onboarding
    // and resume/fork can both reload config with a different tui_theme, so
    // this must happen after the last possible reload.
    if let Some(w) = crate::render::highlight::set_theme_override(
        local_settings.tui.theme.clone(),
        find_genex_home().ok().map(AbsolutePathBuf::into_path_buf),
    ) {
        config.startup_warnings.push(w);
    }

    set_default_client_residency_requirement(config.enforce_residency.value());
    let should_show_trust_screen = should_show_trust_screen(&config);

    let Cli {
        prompt,
        shared,
        no_alt_screen,
        ..
    } = cli;
    let images = shared.into_inner().images;

    let use_alt_screen =
        determine_alt_screen_mode(no_alt_screen, local_settings.tui.alternate_screen);
    tui.set_alt_screen_enabled(use_alt_screen);
    // Persistent app-server resumes may attach to an already-running thread,
    // where resume config overrides are ignored.
    let is_persistent_resume = !matches!(&app_server_target, AppServerTarget::Embedded)
        && matches!(
            &session_selection,
            resume_picker::SessionSelection::Resume(_)
        );
    let bypass_hook_trust_for_startup_review = config.bypass_hook_trust && !is_persistent_resume;
    let hooks_request_handle = app_server.request_handle();
    let hooks_cwd = config.cwd.to_path_buf();
    let startup_prefetch_started_at = Instant::now();
    let startup_prefetch = startup_draft
        .run_until(&mut tui, async {
            tokio::join!(
                async {
                    match startup_account {
                        Some(account) => app_server.bootstrap_with_account(&config, account).await,
                        None => app_server.bootstrap(&config).await,
                    }
                },
                load_startup_hooks_review_entry(hooks_request_handle, hooks_cwd),
            )
        })
        .await;
    let (startup_bootstrap, startup_hooks_entry) = match startup_prefetch {
        Ok(startup_prefetch) => startup_prefetch,
        Err(err) => {
            shutdown_startup_session(Some(app_server), &mut terminal_restore_guard).await;
            return Err(err.into());
        }
    };
    if let Err(err) = startup_draft.flush_pending_events(&mut tui).await {
        shutdown_startup_session(Some(app_server), &mut terminal_restore_guard).await;
        return Err(err.into());
    }
    let startup_bootstrap = match startup_bootstrap {
        Ok(startup_bootstrap) => Some(startup_bootstrap),
        Err(err) => {
            shutdown_startup_session(Some(app_server), &mut terminal_restore_guard).await;
            return Err(err);
        }
    };
    let startup_elapsed_before_app = startup_prefetch_started_at.elapsed();
    let startup_hooks_review = maybe_run_startup_hooks_review(
        &mut app_server,
        &mut tui,
        &config,
        bypass_hook_trust_for_startup_review,
        startup_hooks_entry,
    )
    .await;
    let startup_hooks_browser = match startup_hooks_review {
        Err(err) => {
            shutdown_startup_session(Some(app_server), &mut terminal_restore_guard).await;
            return Err(err);
        }
        Ok(StartupHooksReviewOutcome::Continue) => None,
        Ok(StartupHooksReviewOutcome::OpenHooksBrowser(data)) => Some(data),
    };

    // Keep the large event-loop future out of the enclosing startup futures so session
    // transitions have enough stack headroom to rebuild configuration and the chat widget.
    let app_result = Box::pin(App::run(
        &mut tui,
        app_server,
        config,
        current_cwd.to_path_buf(),
        cli_kv_overrides.clone(),
        overrides.clone(),
        loader_overrides.clone(),
        prompt,
        images,
        session_selection,
        should_show_trust_screen, // Proxy to: is it a first run in this directory?
        app_server_target,
        state_db,
        environment_manager,
        startup_elapsed_before_app,
        startup_bootstrap,
        startup_hooks_browser,
        daemon_startup_warning,
        startup_draft,
        managed_worktree,
    ))
    .await;

    terminal_restore_guard.restore_silently();
    // Mark the end of the recorded session.
    session_log::log_session_end();
    // ignore error when collecting usage – report underlying error instead
    app_result
}

#[expect(
    clippy::print_stderr,
    reason = "TUI should no longer be displayed, so we can write to stderr."
)]
fn restore() {
    if let Err(err) = tui::restore_after_exit() {
        eprintln!(
            "failed to restore terminal. Run `reset` or restart your terminal to recover: {err}"
        );
    }
}

struct TerminalRestoreGuard {
    active: bool,
}

impl TerminalRestoreGuard {
    fn new() -> Self {
        Self { active: true }
    }

    #[cfg_attr(debug_assertions, allow(dead_code))]
    fn restore(&mut self) -> color_eyre::Result<()> {
        if self.active {
            crate::tui::restore_after_exit()?;
            self.active = false;
        }
        Ok(())
    }

    fn restore_silently(&mut self) {
        if self.active {
            restore();
            self.active = false;
        }
    }
}

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        self.restore_silently();
    }
}

/// Determine whether to use the terminal's alternate screen buffer.
///
/// - If `--no-alt-screen` is explicitly passed, always disable alternate screen
/// - Otherwise, respect the `tui.alternate_screen` config setting:
///   - `always`: Use alternate screen
///   - `never`: Inline mode only, preserves scrollback
///   - `auto` (default): Use alternate screen
fn determine_alt_screen_mode(no_alt_screen: bool, tui_alternate_screen: AltScreenMode) -> bool {
    if no_alt_screen {
        return false;
    }

    tui_alternate_screen != AltScreenMode::Never
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginStatus {
    AuthMode(AuthMode),
    NotAuthenticated,
}

/// Reads the account once to determine login status and preserve the response for bootstrap.
async fn get_login_status(
    app_server: &mut AppServerSession,
) -> color_eyre::Result<(LoginStatus, GetAccountResponse)> {
    let account = app_server.read_account().await?;
    let login_status = match account.auth_status {
        ProviderAuthStatus::Authenticated => LoginStatus::AuthMode(AuthMode::ApiKey),
        ProviderAuthStatus::Unauthenticated | ProviderAuthStatus::Invalid => {
            LoginStatus::NotAuthenticated
        }
    };
    Ok((login_status, account))
}

async fn load_config_or_exit(
    cli_kv_overrides: Vec<(String, toml::Value)>,
    overrides: ConfigOverrides,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
) -> Config {
    load_config_or_exit_with_fallback_cwd(
        cli_kv_overrides,
        overrides,
        loader_overrides,
        strict_config,
        /*fallback_cwd*/ None,
        /*worktree*/ None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn load_config_or_exit_with_fallback_cwd(
    cli_kv_overrides: Vec<(String, toml::Value)>,
    overrides: ConfigOverrides,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    fallback_cwd: Option<PathBuf>,
    worktree: Option<&ManagedTuiWorktree>,
) -> Config {
    #[allow(clippy::print_stderr)]
    match load_config_with_worktree_source_policy(
        cli_kv_overrides,
        overrides,
        loader_overrides,
        strict_config,
        fallback_cwd,
        worktree,
    )
    .await
    {
        Ok(config) => config,
        Err(err) => {
            restore_terminal_before_fatal_exit();
            eprintln!("Error loading configuration: {err}");
            if let Some(worktree) = worktree {
                worktree.report_startup_failure();
            }
            std::process::exit(1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn load_config_with_worktree_source_policy(
    cli_kv_overrides: Vec<(String, toml::Value)>,
    overrides: ConfigOverrides,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    fallback_cwd: Option<PathBuf>,
    worktree: Option<&ManagedTuiWorktree>,
) -> std::io::Result<Config> {
    let config = ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides.clone())
        .harness_overrides(overrides.clone())
        .loader_overrides(loader_overrides.clone())
        .strict_config(strict_config)
        .fallback_cwd(fallback_cwd)
        .build()
        .await
        .map_err(std::io::Error::other)?;
    if let Some(worktree) = worktree {
        worktree
            .check_source_policy(
                &cli_kv_overrides,
                &overrides,
                &loader_overrides,
                strict_config,
            )
            .await?;
    }
    Ok(config)
}

#[allow(clippy::print_stderr)]
async fn load_bootstrap_config_or_exit(
    genex_home: &Path,
    cwd: Option<&AbsolutePathBuf>,
    cli_kv_overrides: Vec<(String, genex_config::TomlValue)>,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
) -> ConfigTomlLoadResult {
    match load_config_toml_with_layer_stack(
        genex_home,
        cwd,
        cli_kv_overrides,
        genex_config::ConfigLoadOptions {
            loader_overrides,
            strict_config,
        },
    )
    .await
    {
        Ok(config_toml) => config_toml,
        Err(err) => {
            restore_terminal_before_fatal_exit();
            let config_error = err
                .get_ref()
                .and_then(|err| err.downcast_ref::<ConfigLoadError>())
                .map(ConfigLoadError::config_error);
            if let Some(config_error) = config_error {
                eprintln!(
                    "Error loading config.toml:\n{}",
                    format_config_error_with_source(config_error)
                );
            } else {
                eprintln!("Error loading config.toml: {err}");
            }
            std::process::exit(1);
        }
    }
}

/// Determine if the user has decided whether to trust the current directory.
fn should_show_trust_screen(config: &Config) -> bool {
    config.active_project.trust_level.is_none()
}

fn should_show_onboarding(login_status: LoginStatus, show_trust_screen: bool) -> bool {
    if show_trust_screen {
        return true;
    }

    should_show_login_screen(login_status)
}

fn should_show_login_screen(login_status: LoginStatus) -> bool {
    login_status == LoginStatus::NotAuthenticated
}

mod daemon_startup;
mod daemon_telemetry;
