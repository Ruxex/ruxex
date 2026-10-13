pub(crate) mod debug_sandbox;
mod exit_status;
pub(crate) mod login;

use clap::Args;
use clap::Parser;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_cli::CliConfigOverrides;
use genex_utils_cli::ProfileV2Name;
use std::path::PathBuf;

pub use debug_sandbox::run_command_under_landlock;
pub use debug_sandbox::run_command_under_seatbelt;
pub use login::run_login_delete;
pub use login::run_login_status;
pub use login::run_logout;

use anyhow::Result;
use genex_app_server::AppServerExit;
use genex_app_server::AppServerRuntimeOptions;
use genex_app_server::AppServerTransport;
use genex_app_server::AppServerWebsocketAuthSettings;
use genex_app_server::ModelProviderFactory;
use genex_app_server::app_server_control_socket_path;
use genex_app_server_daemon::LifecycleCommand as AppServerLifecycleCommand;
use genex_app_server_daemon::LifecycleOutput as AppServerLifecycleOutput;
use genex_app_server_daemon::run_with_host_executable;
use genex_arg0::Arg0DispatchPaths;
use genex_config::LoaderOverrides;
use genex_protocol::protocol::SessionSource;
use std::ffi::OsString;

/// Hidden argument used by a provider-owned host when it re-enters itself as
/// the AppServer daemon process.
pub use genex_app_server_daemon::HOST_APP_SERVER_DAEMON_MODE_ARG as APP_SERVER_DAEMON_MODE_ARG;

/// Starts the current host executable as its provider-owned AppServer daemon.
pub async fn start_host_app_server_daemon() -> Result<AppServerLifecycleOutput> {
    let executable = std::env::current_exe()?;
    let command_args = [
        OsString::from(APP_SERVER_DAEMON_MODE_ARG),
        OsString::from("--managed-daemon"),
    ];
    Ok(
        run_with_host_executable(AppServerLifecycleCommand::Start, &executable, &command_args)
            .await?,
    )
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
    let genex_home = genex_core::config::find_genex_home()?;
    let socket_path = app_server_control_socket_path(&genex_home)?;
    let exit = genex_app_server::run_main_with_transport_options_and_provider::<P>(
        arg0_paths,
        cli_config_overrides,
        loader_overrides,
        strict_config,
        /*default_metrics_enabled*/ false,
        AppServerTransport::UnixSocket { socket_path },
        SessionSource::Cli,
        AppServerWebsocketAuthSettings::default(),
        AppServerRuntimeOptions {
            managed_daemon: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(exit)
}

#[derive(Debug, Default, Args)]
pub struct SandboxStateArgs {
    /// JSON value from `genex/sandbox-state-meta` to apply directly.
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

    /// Layer $GENEX_HOME/<name>.config.toml on top of the base user config.
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

    /// Layer $GENEX_HOME/<name>.config.toml on top of the base user config.
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
