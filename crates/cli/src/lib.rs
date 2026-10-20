mod cloud_config;
pub(crate) mod debug_sandbox;
mod doctor;
mod exec_server_telemetry;
mod exit_status;
pub(crate) mod login;
mod marketplace_cmd;
mod mcp_cmd;
mod mcp_login;
mod plugin_cmd;
mod queue_cmd;
mod state_db_recovery;

#[path = "cli_main.rs"]
mod cli_main;

pub(crate) use cli_main::InteractiveRemoteOptions;
pub(crate) use cli_main::SessionArchiveConfigOverrides;
pub(crate) use cli_main::finalize_session_archive_interactive;
pub(crate) use cli_main::resolve_remote_endpoint;
pub use cli_main::run;
pub use cli_main::run_with_provider_factory;
pub use cli_main::run_with_provider_factory_and_identity;

pub type ProviderRuntimeFactory = ruxex_tui::ProviderRuntimeFactory;

use clap::Args;
use clap::Parser;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_cli::CliConfigOverrides;
use ruxex_utils_cli::ProfileV2Name;
use std::path::PathBuf;

pub use debug_sandbox::run_command_under_landlock;
pub use debug_sandbox::run_command_under_seatbelt;
pub use login::run_login_delete;
pub use login::run_login_status;
pub use login::run_logout;

use anyhow::Result;
use ruxex_app_server::AppServerExit;
use ruxex_app_server::AppServerRuntimeOptions;
use ruxex_app_server::AppServerTransport;
use ruxex_app_server::AppServerWebsocketAuthSettings;
use ruxex_app_server::ModelProviderFactory;
use ruxex_app_server::app_server_control_socket_path;
use ruxex_app_server_daemon::LifecycleCommand as AppServerLifecycleCommand;
use ruxex_app_server_daemon::LifecycleOutput as AppServerLifecycleOutput;
use ruxex_app_server_daemon::run_with_host_executable;
use ruxex_arg0::Arg0DispatchPaths;
use ruxex_config::LoaderOverrides;
use ruxex_protocol::protocol::SessionSource;
use ruxex_utils_home_dir::ProductIdentity;
use ruxex_utils_home_dir::find_product_home;
use std::ffi::OsString;

/// Hidden argument used by a provider-owned host when it re-enters itself as
/// the AppServer daemon process.
pub use ruxex_app_server_daemon::HOST_APP_SERVER_DAEMON_MODE_ARG as APP_SERVER_DAEMON_MODE_ARG;

/// Starts the current host executable as its provider-owned AppServer daemon.
pub async fn start_host_app_server_daemon() -> Result<AppServerLifecycleOutput> {
    run_host_app_server_lifecycle(AppServerLifecycleCommand::Start).await
}

/// Runs a lifecycle command against this provider-owned host executable.
pub async fn run_host_app_server_lifecycle(
    command: AppServerLifecycleCommand,
) -> Result<AppServerLifecycleOutput> {
    let executable = std::env::current_exe()?;
    let command_args = [
        OsString::from(APP_SERVER_DAEMON_MODE_ARG),
        OsString::from("--managed-daemon"),
    ];
    Ok(run_with_host_executable(command, &executable, &command_args).await?)
}

/// Runs the generic AppServer entrypoint for the host's hidden daemon mode.
///
/// The caller's binary should dispatch to this function before normal CLI
/// parsing when [`APP_SERVER_DAEMON_MODE_ARG`] is present. `P` is compiled into
/// the host binary and is constructed in the daemon process by `P::new`.
pub async fn run_host_app_server_daemon<P: ModelProviderFactory>(
    arg0_paths: Arg0DispatchPaths,
    cli_config_overrides: CliConfigOverrides,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
) -> Result<AppServerExit> {
    run_host_app_server_daemon_with_product_identity::<P>(
        arg0_paths,
        cli_config_overrides,
        loader_overrides,
        strict_config,
        ProductIdentity::ruxex(),
    )
    .await
}

/// Runs the provider-owned daemon with a host-selected product identity.
pub async fn run_host_app_server_daemon_with_product_identity<P: ModelProviderFactory>(
    arg0_paths: Arg0DispatchPaths,
    cli_config_overrides: CliConfigOverrides,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    product_identity: ProductIdentity,
) -> Result<AppServerExit> {
    let ruxex_home = find_product_home(&product_identity)?;
    let socket_path = app_server_control_socket_path(&ruxex_home)?;
    let exit = ruxex_app_server::run_main_with_transport_options_and_provider::<P>(
        arg0_paths,
        cli_config_overrides,
        loader_overrides,
        strict_config,
        /*default_metrics_enabled*/ false,
        AppServerTransport::UnixSocket { socket_path },
        SessionSource::Cli,
        AppServerWebsocketAuthSettings::default(),
        AppServerRuntimeOptions {
            product_identity,
            managed_daemon: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(exit)
}

/// Runs the provider-owned AppServer daemon with a runtime factory supplied by the host.
pub async fn run_host_app_server_daemon_with_provider_factory(
    arg0_paths: Arg0DispatchPaths,
    cli_config_overrides: CliConfigOverrides,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    provider_factory: ruxex_tui::ProviderRuntimeFactory,
) -> Result<AppServerExit> {
    run_host_app_server_daemon_with_provider_factory_and_identity(
        arg0_paths,
        cli_config_overrides,
        loader_overrides,
        strict_config,
        provider_factory,
        ProductIdentity::ruxex(),
    )
    .await
}

/// Runs the provider-owned daemon with a host-selected factory and product identity.
pub async fn run_host_app_server_daemon_with_provider_factory_and_identity(
    arg0_paths: Arg0DispatchPaths,
    cli_config_overrides: CliConfigOverrides,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    provider_factory: ruxex_tui::ProviderRuntimeFactory,
    product_identity: ProductIdentity,
) -> Result<AppServerExit> {
    let ruxex_home = find_product_home(&product_identity)?;
    let socket_path = app_server_control_socket_path(&ruxex_home)?;
    let exit = ruxex_app_server::run_main_with_transport_options_and_provider_factory(
        arg0_paths,
        cli_config_overrides,
        loader_overrides,
        strict_config,
        /*default_metrics_enabled*/ false,
        AppServerTransport::UnixSocket { socket_path },
        SessionSource::Cli,
        AppServerWebsocketAuthSettings::default(),
        AppServerRuntimeOptions {
            product_identity,
            managed_daemon: true,
            ..Default::default()
        },
        provider_factory,
    )
    .await?;
    Ok(exit)
}

#[derive(Debug, Default, Args)]
pub struct SandboxStateArgs {
    /// JSON value from `ruxex/sandbox-state-meta` to apply directly.
    #[arg(
        long = "sandbox-state-json",
        value_name = "JSON",
        conflicts_with_all = ["permissions_profile", "cwd", "include_managed_config"]
    )]
    pub sandbox_state_json: Option<String>,

    /// Add a readable root to the supplied sandbox state. Repeat for multiple roots.
    #[arg(
        long,
        requires = "sandbox_state_json",
        value_parser = parse_absolute_path
    )]
    pub sandbox_state_readable_root: Vec<AbsolutePathBuf>,

    /// Disable direct network access in the supplied sandbox state.
    #[arg(long, requires = "sandbox_state_json", default_value_t = false)]
    pub sandbox_state_disable_network: bool,
}

// These command structs share common sandbox options, but remain separate
// because each host backend has a slightly different option surface.
#[derive(Debug, Parser)]
pub struct SeatbeltCommand {
    #[command(flatten)]
    pub sandbox_state: SandboxStateArgs,

    /// Named permissions profile to apply from the active configuration stack.
    #[arg(
        long = "permission-profile",
        alias = "permissions-profile",
        short = 'P',
        value_name = "NAME"
    )]
    pub permissions_profile: Option<String>,

    /// Layer $RUXEX_HOME/<name>.config.toml on top of the base user config.
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<ProfileV2Name>,

    /// Working directory used for profile resolution and command execution.
    #[arg(
        short = 'C',
        long = "cd",
        value_name = "DIR",
        requires = "permissions_profile"
    )]
    pub cwd: Option<PathBuf>,

    /// Include managed requirements while resolving an explicit permissions profile.
    #[arg(
        long = "include-managed-config",
        default_value_t = false,
        requires = "permissions_profile"
    )]
    pub include_managed_config: bool,

    /// Allow the sandboxed command to bind/connect AF_UNIX sockets rooted at this path. Relative paths are resolved against the current directory. Repeat to allow multiple paths.
    #[arg(long = "allow-unix-socket", value_parser = parse_absolute_path)]
    pub allow_unix_sockets: Vec<AbsolutePathBuf>,

    /// While the command runs, capture macOS sandbox denials via `log stream` and print them after exit
    #[arg(long = "log-denials", default_value_t = false)]
    pub log_denials: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Full command args to run under seatbelt.
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

fn parse_absolute_path(raw: &str) -> Result<AbsolutePathBuf, String> {
    AbsolutePathBuf::relative_to_current_dir(raw)
        .map_err(|err| format!("invalid path {raw}: {err}"))
}

#[derive(Debug, Parser)]
pub struct LandlockCommand {
    #[command(flatten)]
    pub sandbox_state: SandboxStateArgs,

    /// Named permissions profile to apply from the active configuration stack.
    #[arg(
        long = "permission-profile",
        alias = "permissions-profile",
        short = 'P',
        value_name = "NAME"
    )]
    pub permissions_profile: Option<String>,

    /// Layer $RUXEX_HOME/<name>.config.toml on top of the base user config.
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<ProfileV2Name>,

    /// Working directory used for profile resolution and command execution.
    #[arg(
        short = 'C',
        long = "cd",
        value_name = "DIR",
        requires = "permissions_profile"
    )]
    pub cwd: Option<PathBuf>,

    /// Include managed requirements while resolving an explicit permissions profile.
    #[arg(
        long = "include-managed-config",
        default_value_t = false,
        requires = "permissions_profile"
    )]
    pub include_managed_config: bool,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Full command args to run under the Linux sandbox.
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}
