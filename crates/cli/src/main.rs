use clap::Args;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;
use genex_app_server::runtime::configured_app_server_runtime_from_config;
use genex_app_server_daemon::BootstrapOptions as AppServerBootstrapOptions;
use genex_app_server_daemon::LifecycleCommand as AppServerLifecycleCommand;
use genex_arg0::Arg0DispatchPaths;
use genex_arg0::arg0_dispatch_or_else;
use genex_cli::run_login_delete;
use genex_cli::run_login_status;
use genex_cli::run_logout;
use genex_exec::Cli as ExecCli;
use genex_exec::Command as ExecCommand;
use genex_exec::ReviewArgs;
use genex_exec_server::ExecServerRuntimePaths;
use genex_execpolicy::ExecPolicyCheckCommand;
use genex_http_client::HttpClientFactory;
use genex_http_client::OutboundProxyPolicy;
use genex_rollout_trace::REDUCED_STATE_FILE_NAME;
use genex_rollout_trace::replay_bundle;
use genex_tui::AppExitInfo;
use genex_tui::Cli as TuiCli;
use genex_tui::ExitReason;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_cli::CliConfigOverrides;
use genex_utils_cli::ProfileV2Name;
use genex_utils_cli::SharedCliOptions;
use std::collections::HashSet;
use std::io::IsTerminal;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use supports_color::Stream;

#[cfg(all(
    target_os = "linux",
    target_env = "musl",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[global_allocator]
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod cloud_config;
mod doctor;
mod exec_server_telemetry;
mod marketplace_cmd;
mod mcp_cmd;
mod mcp_login;
mod plugin_cmd;
mod queue_cmd;
mod state_db_recovery;

use crate::mcp_cmd::McpCli;
use crate::plugin_cmd::PluginCli;
use crate::plugin_cmd::PluginSubcommand;
use crate::queue_cmd::QueueCommand;
use doctor::DoctorCommand;
use state_db_recovery as local_state_db;

use genex_config::LoaderOverrides;
use genex_core::build_models_manager_with_provider;
use genex_core::config::Config;
use genex_core::config::ConfigBuilder;
use genex_core::config::ConfigOverrides;
use genex_core::config::edit::ConfigEditsBuilder;
use genex_core::config::find_genex_home;
use genex_core::config::resolve_profile_v2_config_path;
use genex_features::FEATURES;
use genex_features::Stage;
use genex_features::is_known_feature_key;
use genex_home::GenexHomeUserInstructionsProvider;
use genex_models_manager::manager::RefreshStrategy;
use genex_protocol::protocol::AskForApproval;
use genex_protocol::user_input::UserInput;
use genex_terminal_detection::TerminalName;

/// Genex CLI
///
/// If no subcommand is specified, options will be forwarded to the interactive CLI.
#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    // If a sub‑command is given, ignore requirements of the default args.
    subcommand_negates_reqs = true,
    // The executable is sometimes invoked via a platform‑specific name like
    // `genex-x86_64-unknown-linux-musl`, but the help output should always use
    // the generic `genex` command name that users run.
    bin_name = "genex",
    override_usage = "genex [OPTIONS] [PROMPT]\n       genex [OPTIONS] <COMMAND> [ARGS]"
)]
struct MultitoolCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    pub feature_toggles: FeatureToggles,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    interactive: TuiCli,

    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Browse all agent sessions on the shared local app-server daemon.
    Agents(AgentsCommand),

    /// Internal: forward a local TCP socket through an HTTP/3 CONNECT proxy.
    #[clap(hide = true)]
    TcpTunnel(genex_tcp_tunnel::Args),
    /// Run Genex non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Run a code review non-interactively.
    Review(ReviewCommand),

    /// Manage login.
    Login(LoginCommand),

    /// Log out from the mounted model provider and delete its credentials.
    Logout(LogoutCommand),

    /// Manage external MCP servers for Genex.
    Mcp(McpCli),

    /// Manage Genex plugins.
    Plugin(PluginCli),

    /// [experimental] Run the app server or related tooling.
    AppServer(AppServerCommand),

    /// Generate shell completion scripts.
    Completion(CompletionCommand),

    /// Diagnose local Genex installation, config, auth, and runtime health.
    Doctor(DoctorCommand),

    /// Run commands within a Genex-provided sandbox.
    Sandbox(HostSandboxArgs),

    /// Debugging tools.
    Debug(DebugCommand),

    /// Execpolicy tooling.
    #[clap(hide = true)]
    Execpolicy(ExecpolicyCommand),

    /// Resume a previous interactive session (picker by default; use --last to continue the most recent).
    Resume(ResumeCommand),

    /// Queue a message for an existing session.
    Queue(QueueCommand),

    /// Archive a saved session by id or session name.
    Archive(SessionArchiveCommand),

    /// Permanently delete a saved session by id or session name.
    Delete(DeleteCommand),

    /// Unarchive a saved session by id or session name.
    Unarchive(SessionArchiveCommand),

    /// Fork a previous interactive session (picker by default; use --last to fork the most recent).
    Fork(ForkCommand),

    /// Internal: relay stdio to a Unix domain socket.
    #[clap(hide = true, name = "stdio-to-uds")]
    StdioToUds(StdioToUdsCommand),

    /// [EXPERIMENTAL] Run the standalone exec-server service.
    ExecServer(ExecServerCommand),

    /// Inspect feature flags.
    Features(FeaturesCli),
}

#[derive(Debug, Parser)]
struct CompletionCommand {
    /// Shell to generate completions for
    #[clap(value_enum, default_value_t = Shell::Bash)]
    shell: Shell,
}

#[derive(Debug, Parser)]
struct DebugCommand {
    #[command(subcommand)]
    subcommand: DebugSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugSubcommand {
    /// Render the raw model catalog as JSON.
    Models(DebugModelsCommand),

    /// Tooling: helps debug the app server.
    AppServer(DebugAppServerCommand),

    /// Render the model-visible prompt input list as JSON.
    PromptInput(DebugPromptInputCommand),

    /// Replay a rollout trace bundle and write reduced state JSON.
    #[clap(hide = true)]
    TraceReduce(DebugTraceReduceCommand),
}

#[derive(Debug, Parser)]
struct DebugAppServerCommand {
    #[command(subcommand)]
    subcommand: DebugAppServerSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugAppServerSubcommand {
    // Send message to app server V2.
    SendMessageV2(DebugAppServerSendMessageV2Command),
}

#[derive(Debug, Parser)]
struct DebugAppServerSendMessageV2Command {
    #[arg(value_name = "USER_MESSAGE", required = true)]
    user_message: String,
}

#[derive(Debug, Parser)]
struct DebugPromptInputCommand {
    /// Optional user prompt to append after session context.
    #[arg(value_name = "PROMPT")]
    prompt: Option<String>,

    /// Optional image(s) to attach to the user prompt.
    #[arg(long = "image", short = 'i', value_name = "FILE", value_delimiter = ',', num_args = 1..)]
    images: Vec<PathBuf>,
}

#[derive(Debug, Parser)]
struct DebugModelsCommand {}

#[derive(Debug, Parser)]
struct ReviewCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Genex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,

    #[clap(flatten)]
    args: ReviewArgs,
}

#[derive(Debug, Parser)]
struct DebugTraceReduceCommand {
    /// Trace bundle directory containing manifest.json and trace.jsonl.
    #[arg(value_name = "TRACE_BUNDLE")]
    trace_bundle: PathBuf,

    /// Output path for reduced RolloutTrace JSON. Defaults to TRACE_BUNDLE/state.json.
    #[arg(long = "output", short = 'o', value_name = "FILE")]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct AgentsCommand {
    /// The agents overview requires a shared server; this option is rejected.
    #[arg(long, hide = true)]
    no_daemon: bool,
    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    /// Use this directory for new tasks on a remote server.
    #[arg(long = "cd", short = 'C', value_name = "DIR")]
    cwd: Option<PathBuf>,

    /// Disable alternate screen mode.
    #[arg(long = "no-alt-screen", default_value_t = false)]
    no_alt_screen: bool,
}

#[derive(Debug, Parser)]
struct ResumeCommand {
    /// Session id (UUID) or session name. UUIDs take precedence if it parses.
    /// If omitted, use --last to pick the most recent recorded session.
    #[arg(value_name = "SESSION_ID")]
    session_id: Option<String>,

    /// Continue the most recent session without showing the picker.
    #[arg(long = "last", default_value_t = false)]
    last: bool,

    /// Show all sessions (disables cwd filtering and shows CWD column).
    #[arg(long = "all", default_value_t = false)]
    all: bool,

    /// Include non-interactive sessions in the resume picker and --last selection.
    #[arg(long = "include-non-interactive", default_value_t = false)]
    include_non_interactive: bool,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    config_overrides: SessionTuiCli,
}

#[derive(Debug, Parser)]
struct SessionArchiveCommand {
    /// Session id (UUID) or session name. UUIDs take precedence if it parses.
    #[arg(value_name = "SESSION")]
    target: String,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    config_overrides: SessionArchiveConfigOverrides,
}

#[derive(Debug, Args, Clone, Default)]
struct SessionArchiveConfigOverrides {
    #[clap(flatten)]
    shared: SharedCliOptions,

    /// Error out when config.toml contains fields that are not recognized by this version of Genex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,

    #[clap(flatten)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Args)]
struct DeleteCommand {
    #[clap(flatten)]
    session: SessionArchiveCommand,

    /// Delete without prompting. SESSION must be a UUID.
    #[arg(long, default_value_t = false)]
    force: bool,
}

#[derive(Debug, Parser)]
struct ForkCommand {
    /// Conversation/session id (UUID). When provided, forks this session.
    /// If omitted, use --last to pick the most recent recorded session.
    #[arg(value_name = "SESSION_ID")]
    session_id: Option<String>,

    /// Fork the most recent session without showing the picker.
    #[arg(long = "last", default_value_t = false)]
    last: bool,

    /// Show all sessions (disables cwd filtering and shows CWD column).
    #[arg(long = "all", default_value_t = false)]
    all: bool,

    #[clap(flatten)]
    remote: InteractiveRemoteOptions,

    #[clap(flatten)]
    config_overrides: SessionTuiCli,
}

/// TUI arguments for session commands where a parsed prompt implies an explicit session id.
///
/// This keeps `--last PROMPT` valid while rejecting `--last SESSION_ID PROMPT`.
#[derive(Debug)]
struct SessionTuiCli(TuiCli);

impl Args for SessionTuiCli {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        TuiCli::augment_args(cmd).mut_arg("prompt", |arg| arg.conflicts_with("last"))
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        TuiCli::augment_args_for_update(cmd).mut_arg("prompt", |arg| arg.conflicts_with("last"))
    }
}

impl clap::FromArgMatches for SessionTuiCli {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        TuiCli::from_arg_matches(matches).map(Self)
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        self.0.update_from_arg_matches(matches)
    }
}

#[cfg(target_os = "macos")]
type HostSandboxArgs = genex_cli::SeatbeltCommand;
#[cfg(target_os = "linux")]
type HostSandboxArgs = genex_cli::LandlockCommand;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
type HostSandboxArgs = UnsupportedSandboxArgs;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[derive(Debug, Parser)]
struct UnsupportedSandboxArgs {
    /// Layer $GENEX_HOME/<name>.config.toml on top of the base user config.
    #[arg(long = "profile", short = 'p')]
    pub config_profile: Option<ProfileV2Name>,

    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Full command args to run under the host sandbox.
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Parser)]
struct ExecpolicyCommand {
    #[command(subcommand)]
    sub: ExecpolicySubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum ExecpolicySubcommand {
    /// Check execpolicy files against a command.
    #[clap(name = "check")]
    Check(ExecPolicyCheckCommand),
}

#[derive(Debug, Parser)]
struct LoginCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    action: LoginSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum LoginSubcommand {
    /// Show login status.
    Status,
    /// Delete stored credentials for the configured model provider.
    Delete,
}

#[derive(Debug, Parser)]
struct LogoutCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
struct AppServerCommand {
    /// Omit to run the app server; specify a subcommand for tooling.
    #[command(subcommand)]
    subcommand: Option<AppServerSubcommand>,

    #[command(flatten)]
    code_mode_host: genex_app_server::AppServerCodeModeHostArgs,

    /// Error out when config.toml contains fields that are not recognized by this version of Genex.
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,

    /// Transport endpoint URL. Supported values: `stdio://` (default),
    /// `unix://`, `unix://PATH`, `ws://IP:PORT`, `off`.
    #[arg(
        long = "listen",
        value_name = "URL",
        default_value = genex_app_server::AppServerTransport::DEFAULT_LISTEN_URL
    )]
    listen: genex_app_server::AppServerTransport,

    /// Use stdio as the transport (equivalent to `--listen stdio://`).
    #[arg(long = "stdio", conflicts_with = "listen")]
    stdio: bool,

    /// Save loaded threads during managed daemon shutdown.
    #[arg(long, hide = true)]
    managed_daemon: bool,

    /// Controls whether analytics are enabled by default.
    ///
    /// Analytics are disabled by default for app-server. Users have to explicitly opt in
    /// via the `analytics` section in the config.toml file.
    ///
    /// However, for first-party use cases like the VSCode IDE extension, we default analytics
    /// to be enabled by default by setting this flag. Users can still opt out by setting this
    /// in their config.toml:
    ///
    /// ```toml
    /// [metrics]
    /// enabled = false
    /// ```
    ///
    /// See https://developers.provider.com/codex/config-advanced/#metrics for more details.
    #[arg(long = "metrics-default-enabled")]
    metrics_default_enabled: bool,

    #[command(flatten)]
    auth: genex_app_server::AppServerWebsocketAuthArgs,
}

#[derive(Debug, Parser)]
struct ExecServerCommand {
    /// Error out when config.toml contains fields that are not recognized by this version of Genex.
    #[arg(
        id = "exec_server_strict_config",
        long = "strict-config",
        default_value_t = false,
        global = true
    )]
    strict_config: bool,

    /// Maximum number of requests to process concurrently on each connection.
    #[arg(
        long = "concurrent-requests",
        value_name = "COUNT",
        default_value = "1"
    )]
    request_dispatch_mode: genex_exec_server::RequestDispatchMode,

    /// Transport endpoint URL. Supported values: `stdio` or `stdio://`.
    #[arg(long = "listen", value_name = "URL")]
    listen: Option<String>,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::enum_variant_names)]
enum AppServerSubcommand {
    /// Manage the local app-server daemon.
    Daemon(AppServerDaemonCommand),

    /// Proxy stdio bytes to the running app-server control socket.
    Proxy(AppServerProxyCommand),

    /// [experimental] Generate TypeScript bindings for the app server protocol.
    GenerateTs(GenerateTsCommand),

    /// [experimental] Generate JSON Schema for the app server protocol.
    GenerateJsonSchema(GenerateJsonSchemaCommand),

    /// [internal] Generate internal JSON Schema artifacts for Genex tooling.
    #[clap(hide = true)]
    GenerateInternalJsonSchema(GenerateInternalJsonSchemaCommand),
}

#[derive(Debug, Args)]
struct AppServerDaemonCommand {
    #[command(subcommand)]
    subcommand: AppServerDaemonSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum AppServerDaemonSubcommand {
    /// Install durable local app-server management for SSH-driven use.
    Bootstrap,

    /// Start the local app server daemon if it is not already running.
    Start,

    /// Restart the local app server daemon.
    Restart,

    /// Stop the local app server daemon.
    Stop,

    /// Print local CLI and running app-server versions as JSON.
    Version,
}

#[derive(Debug, Args)]
struct AppServerProxyCommand {
    /// Path to the app-server Unix domain socket to connect to.
    #[arg(long = "sock", value_name = "SOCKET_PATH", value_parser = parse_socket_path)]
    socket_path: Option<AbsolutePathBuf>,
}

#[derive(Debug, Args)]
struct GenerateTsCommand {
    /// Output directory where .ts files will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    experimental: bool,
}

#[derive(Debug, Args)]
struct GenerateJsonSchemaCommand {
    /// Output directory where the schema bundle will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Include experimental methods and fields in the generated output
    #[arg(long = "experimental", default_value_t = false)]
    experimental: bool,
}

#[derive(Debug, Args)]
struct GenerateInternalJsonSchemaCommand {
    /// Output directory where internal JSON Schema artifacts will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,
}

#[derive(Debug, Parser)]
struct StdioToUdsCommand {
    /// Path to the Unix domain socket to connect to.
    #[arg(value_name = "SOCKET_PATH", value_parser = parse_socket_path)]
    socket_path: AbsolutePathBuf,
}

fn parse_socket_path(raw: &str) -> Result<AbsolutePathBuf, String> {
    AbsolutePathBuf::relative_to_current_dir(raw)
        .map_err(|err| format!("failed to resolve socket path `{raw}`: {err}"))
}

/// Handle the app exit and print the results.
fn handle_app_exit(exit_info: AppExitInfo) -> anyhow::Result<()> {
    let is_fatal = match &exit_info.exit_reason {
        ExitReason::Fatal(message) => {
            eprintln!("ERROR: {message}");
            true
        }
        ExitReason::UserRequested
        | ExitReason::Archived(_)
        | ExitReason::TurnInterrupted
        | ExitReason::ThreadRemoved => false,
    };

    let color_enabled = supports_color::on(Stream::Stdout).is_some();
    for line in exit_info.format_exit_messages(color_enabled) {
        println!("{line}");
    }
    if is_fatal {
        std::io::stdout().flush()?;
        std::process::exit(1);
    }
    Ok(())
}

fn run_execpolicycheck(cmd: ExecPolicyCheckCommand) -> anyhow::Result<()> {
    cmd.run()
}

async fn run_session_archive_cli_command(
    action: genex_tui::SessionArchiveAction,
    cmd: SessionArchiveCommand,
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    root_remote: Option<String>,
    root_remote_auth_token_env: Option<String>,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<String> {
    let SessionArchiveCommand {
        target,
        remote,
        config_overrides,
    } = cmd;
    interactive =
        finalize_session_archive_interactive(interactive, root_config_overrides, config_overrides);
    let explicit_remote_endpoint = resolve_remote_endpoint(
        remote.remote.or(root_remote),
        remote.remote_auth_token_env.or(root_remote_auth_token_env),
    )?;
    genex_tui::run_session_archive_command(
        action,
        target,
        genex_tui::SessionArchiveCommandOptions {
            cli: interactive,
            arg0_paths,
            explicit_remote_endpoint,
        },
    )
    .await
    .map_err(|err| anyhow::anyhow!("{err}"))
}

fn delete_action(target: &str, force: bool) -> anyhow::Result<genex_tui::SessionArchiveAction> {
    if force && genex_protocol::ThreadId::from_string(target).is_err() {
        anyhow::bail!("--force requires a session UUID; names must be confirmed interactively");
    }
    let confirmation = match force {
        true => genex_tui::DeleteConfirmation::Skip,
        false => genex_tui::DeleteConfirmation::Prompt,
    };
    Ok(genex_tui::SessionArchiveAction::Delete(confirmation))
}

async fn run_debug_app_server_command(cmd: DebugAppServerCommand) -> anyhow::Result<()> {
    match cmd.subcommand {
        DebugAppServerSubcommand::SendMessageV2(cmd) => {
            let genex_bin = std::env::current_exe()?;
            genex_app_server_test_client::send_message_v2(&genex_bin, &[], cmd.user_message, &None)
                .await
        }
    }
}

#[derive(Debug, Default, Parser, Clone)]
struct FeatureToggles {
    /// Enable a feature (repeatable). Equivalent to `-c features.<name>=true`.
    #[arg(long = "enable", value_name = "FEATURE", action = clap::ArgAction::Append, global = true)]
    enable: Vec<String>,

    /// Disable a feature (repeatable). Equivalent to `-c features.<name>=false`.
    #[arg(long = "disable", value_name = "FEATURE", action = clap::ArgAction::Append, global = true)]
    disable: Vec<String>,
}

#[derive(Debug, Default, Parser, Clone)]
struct InteractiveRemoteOptions {
    /// Connect the TUI to a remote app server endpoint.
    ///
    /// Accepted forms: `ws://host:port`, `wss://host:port`, `unix://`, or `unix://PATH`.
    #[arg(long = "remote", value_name = "ADDR")]
    remote: Option<String>,

    /// Name of the environment variable containing the bearer token to send to
    /// a remote app server websocket.
    #[arg(long = "remote-auth-token-env", value_name = "ENV_VAR")]
    remote_auth_token_env: Option<String>,
}

impl FeatureToggles {
    fn to_overrides(&self) -> anyhow::Result<Vec<String>> {
        let mut v = Vec::new();
        for feature in &self.enable {
            Self::validate_feature(feature)?;
            v.push(format!("features.{feature}=true"));
        }
        for feature in &self.disable {
            Self::validate_feature(feature)?;
            v.push(format!("features.{feature}=false"));
        }
        Ok(v)
    }

    fn validate_feature(feature: &str) -> anyhow::Result<()> {
        if is_known_feature_key(feature) {
            Ok(())
        } else {
            anyhow::bail!("Unknown feature flag: {feature}")
        }
    }
}

#[derive(Debug, Parser)]
struct FeaturesCli {
    #[command(subcommand)]
    sub: FeaturesSubcommand,
}

#[derive(Debug, Parser)]
enum FeaturesSubcommand {
    /// List known features with their stage and effective state.
    List,
    /// Enable a feature in config.toml.
    Enable(FeatureSetArgs),
    /// Disable a feature in config.toml.
    Disable(FeatureSetArgs),
}

#[derive(Debug, Parser)]
struct FeatureSetArgs {
    /// Feature key to update (for example: unified_exec).
    feature: String,
}

fn stage_str(stage: Stage) -> &'static str {
    match stage {
        Stage::UnderDevelopment => "under development",
        Stage::Experimental { .. } => "experimental",
        Stage::Stable => "stable",
        Stage::Deprecated => "deprecated",
        Stage::Removed => "removed",
    }
}

fn main() -> anyhow::Result<()> {
    genex_build_info::initialize!();
    arg0_dispatch_or_else(move |arg0_paths: Arg0DispatchPaths| async move {
        cli_main(arg0_paths).await?;
        Ok(())
    })
}

async fn cli_main(arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    let MultitoolCli {
        config_overrides: mut root_config_overrides,
        feature_toggles,
        remote,
        mut interactive,
        subcommand,
    } = MultitoolCli::parse();
    reject_removed_top_level_command(&subcommand, &interactive)?;
    reject_unsupported_worktree_for_subcommand(interactive.shared.worktree, &subcommand)?;
    // Fold --enable/--disable into config overrides so they flow to all subcommands.
    let toggle_overrides = feature_toggles.to_overrides()?;
    root_config_overrides.raw_overrides.extend(toggle_overrides);
    let agents_options = match &subcommand {
        Some(Subcommand::Agents(options)) => Some(options),
        _ => None,
    };
    if let Some(options) = agents_options
        && let Some(root_endpoint) = &remote.remote
        && let Some(agents_endpoint) = &options.remote.remote
        && root_endpoint != agents_endpoint
    {
        anyhow::bail!("`genex agents` received conflicting remote server endpoints");
    }
    let root_remote = agents_options
        .and_then(|options| options.remote.remote.clone())
        .or(remote.remote);
    let root_remote_auth_token_env = agents_options
        .and_then(|options| options.remote.remote_auth_token_env.clone())
        .or(remote.remote_auth_token_env);
    if let Some(options) = agents_options {
        interactive.cwd = options.cwd.clone().or(interactive.cwd.take());
        interactive.no_alt_screen |= options.no_alt_screen;
        interactive.no_daemon |= options.no_daemon;
    }
    let root_strict_config = interactive.strict_config;
    reject_root_strict_config_for_subcommand(root_strict_config, &subcommand)?;
    if let Some(subcommand) = subcommand.as_ref() {
        profile_v2_for_subcommand(&interactive, subcommand)?;
    }

    let open_agents_overview = matches!(&subcommand, Some(Subcommand::Agents(_)));
    match subcommand {
        None | Some(Subcommand::Agents(_)) => {
            prepend_config_flags(
                &mut interactive.config_overrides,
                root_config_overrides.clone(),
            );
            if open_agents_overview {
                if interactive.prompt.is_some() || !interactive.images.is_empty() {
                    anyhow::bail!("`genex agents` does not accept an initial prompt or images");
                }
                if root_remote.is_some()
                    && (!interactive.add_dir.is_empty()
                        || interactive
                            .config_overrides
                            .parse_overrides()
                            .map_err(anyhow::Error::msg)?
                            .iter()
                            .any(|(key, value)| {
                                key == "sandbox_workspace_write.writable_roots"
                                    || (key == "sandbox_workspace_write"
                                        && value.get("writable_roots").is_some())
                            }))
                {
                    anyhow::bail!(
                        "`genex agents` cannot apply additional-directory overrides to a remote server"
                    );
                }
                if root_remote.is_none() {
                    resolve_remote_endpoint(
                        /*remote*/ None,
                        root_remote_auth_token_env.clone(),
                    )?;
                    #[cfg(not(unix))]
                    anyhow::bail!("`genex agents` requires `--remote` on this platform");
                }
                interactive.agents_overview = true;
            }
            let exit_info = run_interactive_tui(
                interactive,
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            handle_app_exit(exit_info)?;
        }
        Some(Subcommand::TcpTunnel(args)) => {
            return genex_tcp_tunnel::run(args).await;
        }
        Some(Subcommand::Exec(mut exec_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "exec",
            )?;
            exec_cli
                .shared
                .inherit_exec_root_options(&interactive.shared);
            exec_cli.strict_config |= root_strict_config;
            prepend_config_flags(
                &mut exec_cli.config_overrides,
                root_config_overrides.clone(),
            );
            genex_exec::run_main(exec_cli, arg0_paths.clone()).await?;
        }
        Some(Subcommand::Review(ReviewCommand {
            strict_config,
            args: review_args,
        })) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "review",
            )?;
            let mut exec_cli = ExecCli::try_parse_from(["genex", "exec"])?;
            exec_cli
                .shared
                .inherit_exec_root_options(&interactive.shared);
            exec_cli.command = Some(ExecCommand::Review(review_args));
            exec_cli.strict_config = strict_config || root_strict_config;
            prepend_config_flags(
                &mut exec_cli.config_overrides,
                root_config_overrides.clone(),
            );
            genex_exec::run_main(exec_cli, arg0_paths.clone()).await?;
        }
        Some(Subcommand::Mcp(mut mcp_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "mcp",
            )?;
            // Propagate any root-level config overrides (e.g. `-c key=value`).
            prepend_config_flags(&mut mcp_cli.config_overrides, root_config_overrides.clone());
            let loader_overrides =
                loader_overrides_for_profile(interactive.config_profile_v2.as_ref())?;
            mcp_cli.run(loader_overrides).await?;
        }
        Some(Subcommand::Plugin(plugin_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "plugin",
            )?;
            let PluginCli {
                mut config_overrides,
                subcommand,
            } = plugin_cli;
            prepend_config_flags(&mut config_overrides, root_config_overrides.clone());
            match subcommand {
                PluginSubcommand::Add(args) => {
                    let overrides = config_overrides
                        .parse_overrides()
                        .map_err(anyhow::Error::msg)?;
                    plugin_cmd::run_plugin_add(overrides, args).await?;
                }
                PluginSubcommand::List(args) => {
                    let overrides = config_overrides
                        .parse_overrides()
                        .map_err(anyhow::Error::msg)?;
                    plugin_cmd::run_plugin_list(overrides, args).await?;
                }
                PluginSubcommand::Marketplace(mut marketplace_cli) => {
                    prepend_config_flags(&mut marketplace_cli.config_overrides, config_overrides);
                    marketplace_cli.run().await?;
                }
                PluginSubcommand::Remove(args) => {
                    let overrides = config_overrides
                        .parse_overrides()
                        .map_err(anyhow::Error::msg)?;
                    plugin_cmd::run_plugin_remove(overrides, args).await?;
                }
            }
        }
        Some(Subcommand::AppServer(app_server_cli)) => {
            let AppServerCommand {
                subcommand,
                code_mode_host,
                strict_config: app_server_strict_config,
                listen,
                stdio,
                managed_daemon,
                metrics_default_enabled,
                auth,
            } = app_server_cli;
            let strict_config = app_server_strict_config || root_strict_config;
                reject_strict_config_for_app_server_subcommand(strict_config, subcommand.as_ref())?;
            match subcommand {
                None => {
                    let transport = if stdio {
                        genex_app_server::AppServerTransport::Stdio
                    } else {
                        listen
                    };
                    let auth = auth.try_into_settings()?;
                    let runtime_options = genex_app_server::AppServerRuntimeOptions {
                        code_mode_host_transport: code_mode_host.into(),
                        managed_daemon,
                        ..Default::default()
                    };
                    let exit = genex_app_server::run_main_with_transport_options(
                        arg0_paths.clone(),
                        root_config_overrides,
                        LoaderOverrides::default(),
                        strict_config,
                        metrics_default_enabled,
                        transport,
                        genex_protocol::protocol::SessionSource::VSCode,
                        auth,
                        runtime_options,
                    )
                    .await?;
                    if exit == genex_app_server::AppServerExit::Forced {
                        // Runtime teardown can wait forever for blocked rollout I/O.
                        std::process::exit(0);
                    }
                }
                Some(AppServerSubcommand::Daemon(daemon_cli)) => match daemon_cli.subcommand {
                    AppServerDaemonSubcommand::Start => {
                        mount_provider_home_before_daemon(&root_config_overrides).await?;
                        print_app_server_daemon_output(AppServerLifecycleCommand::Start).await?;
                    }
                    AppServerDaemonSubcommand::Bootstrap => {
                        mount_provider_home_before_daemon(&root_config_overrides).await?;
                        let output =
                            genex_app_server_daemon::bootstrap(AppServerBootstrapOptions {})
                                .await?;
                        println!("{}", serde_json::to_string(&output)?);
                    }
                    AppServerDaemonSubcommand::Restart => {
                        mount_provider_home_before_daemon(&root_config_overrides).await?;
                        print_app_server_daemon_output(AppServerLifecycleCommand::Restart).await?;
                    }
                    AppServerDaemonSubcommand::Stop => {
                        print_app_server_daemon_output(AppServerLifecycleCommand::Stop).await?;
                    }
                    AppServerDaemonSubcommand::Version => {
                        print_app_server_daemon_output(AppServerLifecycleCommand::Version).await?;
                    }
                },
                Some(AppServerSubcommand::Proxy(proxy_cli)) => {
                    let socket_path = match proxy_cli.socket_path {
                        Some(socket_path) => socket_path,
                        None => {
                            let genex_home = find_genex_home()?;
                            genex_app_server::app_server_control_socket_path(&genex_home)?
                        }
                    };
                    genex_stdio_to_uds::run(socket_path.as_path()).await?;
                }
                Some(AppServerSubcommand::GenerateTs(gen_cli)) => {
                    let options = genex_app_server_protocol::GenerateTsOptions {
                        experimental_api: gen_cli.experimental,
                        ..Default::default()
                    };
                    genex_app_server_protocol::generate_ts_with_options(&gen_cli.out_dir, options)?;
                }
                Some(AppServerSubcommand::GenerateJsonSchema(gen_cli)) => {
                    genex_app_server_protocol::generate_json_with_experimental(
                        &gen_cli.out_dir,
                        gen_cli.experimental,
                    )?;
                }
                Some(AppServerSubcommand::GenerateInternalJsonSchema(gen_cli)) => {
                    genex_app_server_protocol::generate_internal_json_schema(&gen_cli.out_dir)?;
                }
            }
        }
        Some(Subcommand::Resume(ResumeCommand {
            session_id,
            last,
            all,
            include_non_interactive,
            remote,
            config_overrides,
        })) => {
            let SessionTuiCli(config_overrides) = config_overrides;
            interactive = finalize_resume_interactive(
                interactive,
                root_config_overrides.clone(),
                session_id,
                last,
                all,
                include_non_interactive,
                config_overrides,
            );
            let exit_info = run_interactive_tui(
                interactive,
                remote.remote.or(root_remote.clone()),
                remote
                    .remote_auth_token_env
                    .or(root_remote_auth_token_env.clone()),
                arg0_paths.clone(),
            )
            .await?;
            handle_app_exit(exit_info)?;
        }
        Some(Subcommand::Archive(cmd)) => {
            let output = run_session_archive_cli_command(
                genex_tui::SessionArchiveAction::Archive,
                cmd,
                interactive,
                root_config_overrides.clone(),
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            println!("{output}");
        }
        Some(Subcommand::Queue(cmd)) => {
            let output = queue_cmd::run_queue_command(
                cmd,
                interactive,
                root_config_overrides.clone(),
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            println!("{output}");
        }
        Some(Subcommand::Delete(DeleteCommand { session, force })) => {
            let action = delete_action(&session.target, force)?;
            let output = run_session_archive_cli_command(
                action,
                session,
                interactive,
                root_config_overrides.clone(),
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            println!("{output}");
        }
        Some(Subcommand::Unarchive(cmd)) => {
            let output = run_session_archive_cli_command(
                genex_tui::SessionArchiveAction::Unarchive,
                cmd,
                interactive,
                root_config_overrides.clone(),
                root_remote.clone(),
                root_remote_auth_token_env.clone(),
                arg0_paths.clone(),
            )
            .await?;
            println!("{output}");
        }
        Some(Subcommand::Fork(ForkCommand {
            session_id,
            last,
            all,
            remote,
            config_overrides,
        })) => {
            let SessionTuiCli(config_overrides) = config_overrides;
            interactive = finalize_fork_interactive(
                interactive,
                root_config_overrides.clone(),
                session_id,
                last,
                all,
                config_overrides,
            );
            let exit_info = run_interactive_tui(
                interactive,
                remote.remote.or(root_remote.clone()),
                remote
                    .remote_auth_token_env
                    .or(root_remote_auth_token_env.clone()),
                arg0_paths.clone(),
            )
            .await?;
            handle_app_exit(exit_info)?;
        }
        Some(Subcommand::Login(mut login_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "login",
            )?;
            prepend_config_flags(
                &mut login_cli.config_overrides,
                root_config_overrides.clone(),
            );
            match login_cli.action {
                LoginSubcommand::Status => {
                    run_login_status(login_cli.config_overrides).await;
                }
                LoginSubcommand::Delete => {
                    run_login_delete(login_cli.config_overrides).await;
                }
            }
        }
        Some(Subcommand::Logout(mut logout_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "logout",
            )?;
            prepend_config_flags(
                &mut logout_cli.config_overrides,
                root_config_overrides.clone(),
            );
            run_logout(logout_cli.config_overrides).await;
        }
        Some(Subcommand::Completion(completion_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "completion",
            )?;
            print_completion(completion_cli);
        }
        Some(Subcommand::Doctor(doctor_cli)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "doctor",
            )?;
            doctor::run_doctor(
                doctor_cli,
                root_config_overrides.clone(),
                &interactive,
                &arg0_paths,
            )
            .await?;
        }
        Some(Subcommand::Sandbox(mut sandbox_cli)) => {
            let config_profile = sandbox_cli
                .config_profile
                .as_ref()
                .or(interactive.config_profile_v2.as_ref());
            prepend_config_flags(
                &mut sandbox_cli.config_overrides,
                root_config_overrides.clone(),
            );
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "sandbox",
            )?;
            let loader_overrides = loader_overrides_for_profile(config_profile)?;
            #[cfg(target_os = "macos")]
            genex_cli::run_command_under_seatbelt(
                sandbox_cli,
                arg0_paths.genex_linux_sandbox_exe.clone(),
                loader_overrides,
            )
            .await?;
            #[cfg(target_os = "linux")]
            genex_cli::run_command_under_landlock(
                sandbox_cli,
                arg0_paths.genex_linux_sandbox_exe.clone(),
                loader_overrides,
            )
            .await?;
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                let _ = loader_overrides;
                anyhow::bail!("`genex sandbox` is not supported on this operating system");
            }
        }
        Some(Subcommand::Debug(DebugCommand { subcommand })) => match subcommand {
            DebugSubcommand::Models(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug models",
                )?;
                run_debug_models_command(cmd, root_config_overrides).await?;
            }
            DebugSubcommand::AppServer(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug app-server",
                )?;
                run_debug_app_server_command(cmd).await?;
            }
            DebugSubcommand::PromptInput(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug prompt-input",
                )?;
                run_debug_prompt_input_command(
                    cmd,
                    root_config_overrides,
                    interactive,
                    arg0_paths.clone(),
                )
                .await?;
            }
            DebugSubcommand::TraceReduce(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "debug trace-reduce",
                )?;
                run_debug_trace_reduce_command(cmd).await?;
            }
        },
        Some(Subcommand::Execpolicy(ExecpolicyCommand { sub })) => match sub {
            ExecpolicySubcommand::Check(cmd) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "execpolicy check",
                )?;
                run_execpolicycheck(cmd)?
            }
        },
        Some(Subcommand::StdioToUds(cmd)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "stdio-to-uds",
            )?;
            let socket_path = cmd.socket_path;
            genex_stdio_to_uds::run(socket_path.as_path()).await?;
        }
        Some(Subcommand::ExecServer(cmd)) => {
            reject_remote_mode_for_subcommand(
                root_remote.as_deref(),
                root_remote_auth_token_env.as_deref(),
                "exec-server",
            )?;
            let strict_config = cmd.strict_config || root_strict_config;
            run_exec_server_command(cmd, &arg0_paths, &root_config_overrides, strict_config)
                .await?;
        }
        Some(Subcommand::Features(FeaturesCli { sub })) => match sub {
            FeaturesSubcommand::List => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "features list",
                )?;
                // Honor `--search` via the canonical web_search mode.
                if interactive.web_search {
                    root_config_overrides
                        .raw_overrides
                        .push("web_search=\"live\"".to_string());
                }

                let config =
                    cloud_config::load_config(&root_config_overrides, LoaderOverrides::default())
                        .await?;
                let mut rows = Vec::with_capacity(FEATURES.len());
                let mut name_width = 0;
                let mut stage_width = 0;
                for def in FEATURES {
                    let name = def.key;
                    let stage = stage_str(def.stage);
                    let enabled = config.features.enabled(def.id);
                    name_width = name_width.max(name.len());
                    stage_width = stage_width.max(stage.len());
                    rows.push((name, stage, enabled));
                }
                rows.sort_unstable_by_key(|(name, _, _)| *name);

                for (name, stage, enabled) in rows {
                    println!("{name:<name_width$}  {stage:<stage_width$}  {enabled}");
                }
            }
            FeaturesSubcommand::Enable(FeatureSetArgs { feature }) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "features enable",
                )?;
                enable_feature_in_config(&feature).await?;
            }
            FeaturesSubcommand::Disable(FeatureSetArgs { feature }) => {
                reject_remote_mode_for_subcommand(
                    root_remote.as_deref(),
                    root_remote_auth_token_env.as_deref(),
                    "features disable",
                )?;
                disable_feature_in_config(&feature).await?;
            }
        },
    }

    Ok(())
}

fn profile_v2_for_subcommand<'a>(
    interactive: &'a TuiCli,
    subcommand: &Subcommand,
) -> anyhow::Result<Option<&'a ProfileV2Name>> {
    let Some(profile_v2) = interactive.config_profile_v2.as_ref() else {
        return Ok(None);
    };

    match subcommand {
        Subcommand::Agents(_)
        | Subcommand::Exec(_)
        | Subcommand::Review(_)
        | Subcommand::Resume(_)
        | Subcommand::Queue(_)
        | Subcommand::Archive(_)
        | Subcommand::Delete(_)
        | Subcommand::Unarchive(_)
        | Subcommand::Fork(_)
        | Subcommand::Mcp(_)
        | Subcommand::Sandbox(_)
        | Subcommand::Debug(DebugCommand {
            subcommand: DebugSubcommand::PromptInput(_),
        }) => Ok(Some(profile_v2)),
        _ => anyhow::bail!(
            "--profile only applies to runtime commands and `genex mcp`: `genex`, `genex exec`, `genex review`, `genex resume`, `genex queue`, `genex archive`, `genex delete`, `genex unarchive`, `genex fork`, `genex mcp`, `genex sandbox`, and `genex debug prompt-input`."
        ),
    }
}

async fn run_exec_server_command(
    cmd: ExecServerCommand,
    arg0_paths: &Arg0DispatchPaths,
    root_config_overrides: &CliConfigOverrides,
    strict_config: bool,
) -> anyhow::Result<()> {
    let genex_self_exe = arg0_paths
        .genex_self_exe
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Genex executable path is not configured"))?;
    let runtime_paths =
        ExecServerRuntimePaths::new(genex_self_exe, arg0_paths.genex_linux_sandbox_exe.clone())?;
    let config_result = load_exec_server_config(root_config_overrides, strict_config).await;
    let config = if strict_config {
        Some(config_result?)
    } else {
        config_result.ok()
    };
    let (_otel, telemetry) = exec_server_telemetry::init(config.as_ref());
    #[cfg(target_os = "macos")]
    let runtime_paths =
        runtime_paths.with_allowed_symlinked_genex_home(config.as_ref().and_then(|config| {
            genex_config::allowed_symlinked_genex_home(
                &config.config_layer_stack,
                &config.genex_home,
            )
        }));
    let http_client_factory = config
        .as_ref()
        .map(Config::http_client_factory)
        .unwrap_or_else(|| HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault));
    let listen_url = cmd
        .listen
        .unwrap_or_else(|| genex_exec_server::DEFAULT_LISTEN_URL.to_string());
    let run = exec_server_telemetry::run_until_shutdown(
        genex_exec_server::run_main_with_telemetry(
            &listen_url,
            runtime_paths,
            telemetry,
            http_client_factory,
            cmd.request_dispatch_mode,
        ),
        exec_server_telemetry::ParentLifetime::Independent,
        exec_server_telemetry::ShutdownBehavior::Immediate,
    );
    run.await.map_err(anyhow::Error::from_boxed)
}

async fn load_exec_server_config(
    root_config_overrides: &CliConfigOverrides,
    strict_config: bool,
) -> anyhow::Result<genex_core::config::Config> {
    let cli_kv_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let builder = ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .strict_config(strict_config);
    Ok(builder.build().await?)
}

async fn enable_feature_in_config(feature: &str) -> anyhow::Result<()> {
    FeatureToggles::validate_feature(feature)?;
    let genex_home = find_genex_home()?;
    ConfigEditsBuilder::new(&genex_home)
        .set_feature_enabled(feature, /*enabled*/ true)
        .apply()
        .await?;
    println!("Enabled feature `{feature}` in config.toml.");
    maybe_print_under_development_feature_warning(&genex_home, feature);
    Ok(())
}

async fn disable_feature_in_config(feature: &str) -> anyhow::Result<()> {
    FeatureToggles::validate_feature(feature)?;
    let genex_home = find_genex_home()?;
    ConfigEditsBuilder::new(&genex_home)
        .set_feature_enabled(feature, /*enabled*/ false)
        .apply()
        .await?;
    println!("Disabled feature `{feature}` in config.toml.");
    Ok(())
}

fn loader_overrides_for_profile(
    profile_v2: Option<&ProfileV2Name>,
) -> anyhow::Result<LoaderOverrides> {
    match profile_v2 {
        Some(profile_v2) => {
            let genex_home = find_genex_home()?;
            Ok(loader_overrides_for_profile_at_genex_home(
                Some(profile_v2),
                &genex_home,
            ))
        }
        None => Ok(LoaderOverrides::default()),
    }
}

fn loader_overrides_for_profile_at_genex_home(
    profile_v2: Option<&ProfileV2Name>,
    genex_home: &std::path::Path,
) -> LoaderOverrides {
    match profile_v2 {
        Some(profile_v2) => LoaderOverrides {
            user_config_path: Some(resolve_profile_v2_config_path(genex_home, profile_v2)),
            user_config_profile: Some(profile_v2.clone()),
            ..Default::default()
        },
        None => LoaderOverrides::default(),
    }
}

fn maybe_print_under_development_feature_warning(genex_home: &std::path::Path, feature: &str) {
    let Some(spec) = FEATURES.iter().find(|spec| spec.key == feature) else {
        return;
    };
    if !matches!(spec.stage, Stage::UnderDevelopment) {
        return;
    }

    let config_path = genex_home.join(genex_config::CONFIG_TOML_FILE);
    eprintln!(
        "Under-development features enabled: {feature}. Under-development features are incomplete and may behave unpredictably. To suppress this warning, set `suppress_unstable_features_warning = true` in {}.",
        config_path.display()
    );
}

async fn run_debug_trace_reduce_command(cmd: DebugTraceReduceCommand) -> anyhow::Result<()> {
    let output = cmd
        .output
        .unwrap_or_else(|| cmd.trace_bundle.join(REDUCED_STATE_FILE_NAME));

    let trace = replay_bundle(&cmd.trace_bundle)?;
    let reduced_json = serde_json::to_vec_pretty(&trace)?;
    tokio::fs::write(&output, reduced_json).await?;
    println!("{}", output.display());

    Ok(())
}

async fn run_debug_prompt_input_command(
    cmd: DebugPromptInputCommand,
    root_config_overrides: CliConfigOverrides,
    interactive: TuiCli,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<()> {
    let loader_overrides = loader_overrides_for_profile(interactive.config_profile_v2.as_ref())?;
    let shared = interactive.shared.into_inner();
    let mut cli_kv_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    if interactive.web_search {
        cli_kv_overrides.push((
            "web_search".to_string(),
            toml::Value::String("live".to_string()),
        ));
    }

    let approval_policy = if shared.dangerously_bypass_approvals_and_sandbox {
        Some(AskForApproval::Never)
    } else {
        interactive.approval_policy.map(Into::into)
    };
    let sandbox_mode = if shared.dangerously_bypass_approvals_and_sandbox {
        Some(genex_protocol::config_types::SandboxMode::DangerFullAccess)
    } else {
        shared.sandbox_mode.map(Into::into)
    };
    let overrides = ConfigOverrides {
        model: shared.model,
        approval_policy,
        sandbox_mode,
        cwd: shared.cwd,
        genex_self_exe: arg0_paths.genex_self_exe,
        genex_linux_sandbox_exe: arg0_paths.genex_linux_sandbox_exe,
        show_raw_agent_reasoning: None,
        ephemeral: Some(true),
        bypass_hook_trust: shared.bypass_hook_trust.then_some(true),
        additional_writable_roots: shared.add_dir,
        ..Default::default()
    };
    let config = ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .harness_overrides(overrides)
        .loader_overrides(loader_overrides)
        .build()
        .await?;
    let runtime = configured_app_server_runtime_from_config(&config, false).await?;

    let mut input = shared
        .images
        .into_iter()
        .chain(cmd.images)
        .map(|path| UserInput::LocalImage { path, detail: None })
        .collect::<Vec<_>>();
    if let Some(prompt) = cmd.prompt.or(interactive.prompt) {
        input.push(UserInput::Text {
            text: prompt.replace("\r\n", "\n").replace('\r', "\n"),
            text_elements: Vec::new(),
        });
    }

    let user_instructions_provider = Arc::new(GenexHomeUserInstructionsProvider::new(
        config.genex_home.clone(),
    ));
    let mut extensions = genex_extension_api::ExtensionRegistryBuilder::new();
    genex_skills_extension::install(&mut extensions, |config: &Config| {
        genex_skills_extension::SkillsExtensionConfig {
            include_instructions: config.include_skill_instructions,
            max_context_tokens: config.skill_max_context_tokens,
            bundled_skills_enabled: config.bundled_skills_enabled(),
            orchestrator_skills_enabled: config.orchestrator_skills_enabled,
            shadow_selection_enabled: config
                .features
                .enabled(genex_features::Feature::SkillSearch),
        }
    });
    let prompt_input = genex_core::build_prompt_input(
        config,
        runtime.provider(),
        input,
        /*state_db*/ None,
        Arc::new(extensions.build()),
        user_instructions_provider,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&prompt_input)?);

    Ok(())
}

async fn run_debug_models_command(
    _cmd: DebugModelsCommand,
    root_config_overrides: CliConfigOverrides,
) -> anyhow::Result<()> {
    let cli_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = ConfigBuilder::default()
        .cli_overrides(cli_overrides)
        .build()
        .await?;
    let runtime = configured_app_server_runtime_from_config(&config, true).await?;
    let models_manager = build_models_manager_with_provider(&config, runtime.provider());
    let catalog = models_manager
        .raw_model_catalog(
            RefreshStrategy::OnlineIfUncached,
            config.http_client_factory(),
        )
        .await;

    serde_json::to_writer(std::io::stdout(), &catalog)?;
    println!();
    Ok(())
}

/// Prepend root-level overrides so they have lower precedence than
/// CLI-specific ones specified after the subcommand (if any).
fn prepend_config_flags(
    subcommand_config_overrides: &mut CliConfigOverrides,
    cli_config_overrides: CliConfigOverrides,
) {
    subcommand_config_overrides.prepend_root_overrides(cli_config_overrides);
}

fn reject_remote_mode_for_subcommand(
    remote: Option<&str>,
    remote_auth_token_env: Option<&str>,
    subcommand: &str,
) -> anyhow::Result<()> {
    if let Some(remote) = remote {
        anyhow::bail!(
            "`--remote {remote}` is only supported for interactive TUI commands, not `genex {subcommand}`"
        );
    }
    if remote_auth_token_env.is_some() {
        anyhow::bail!(
            "`--remote-auth-token-env` is only supported for interactive TUI commands, not `genex {subcommand}`"
        );
    }
    Ok(())
}

fn reject_unsupported_worktree_for_subcommand(
    root_worktree: bool,
    subcommand: &Option<Subcommand>,
) -> anyhow::Result<()> {
    let subcommand_worktree = match subcommand {
        Some(Subcommand::Exec(command)) => command.shared.worktree,
        Some(Subcommand::Resume(command)) => command.config_overrides.0.shared.worktree,
        Some(Subcommand::Fork(command)) => command.config_overrides.0.shared.worktree,
        Some(Subcommand::Archive(command)) | Some(Subcommand::Unarchive(command)) => {
            command.config_overrides.shared.worktree
        }
        Some(Subcommand::Delete(command)) => command.session.config_overrides.shared.worktree,
        Some(Subcommand::Queue(command)) => command.config_overrides.shared.worktree,
        _ => false,
    };

    if !root_worktree && !subcommand_worktree {
        return Ok(());
    }

    match subcommand {
        None => Ok(()),
        Some(Subcommand::Fork(command)) if command.session_id.is_some() && !command.last => Ok(()),
        Some(Subcommand::Fork(_)) => {
            anyhow::bail!("`genex fork --worktree` requires an explicit session ID")
        }
        Some(Subcommand::Exec(command)) => match &command.command {
            None | Some(ExecCommand::Fork(_)) => Ok(()),
            Some(ExecCommand::Resume(_)) => anyhow::bail!(
                "`--worktree` cannot resume an existing session; use `genex exec fork --worktree`"
            ),
            Some(ExecCommand::Review(_)) => {
                anyhow::bail!("`--worktree` is not supported for code review")
            }
        },
        _ => {
            anyhow::bail!(
                "`--worktree` supports new interactive sessions, `genex fork`, `genex exec`, and `genex exec fork`"
            )
        }
    }
}

fn reject_removed_top_level_command(
    subcommand: &Option<Subcommand>,
    interactive: &TuiCli,
) -> anyhow::Result<()> {
    if subcommand.is_some() {
        return Ok(());
    }

    match interactive.prompt.as_deref() {
        Some("app") => anyhow::bail!("`genex app` has been removed"),
        Some("update") => anyhow::bail!(
            "`genex update` has been removed; update Genex with the package manager or distribution that installed it"
        ),
        _ => Ok(()),
    }
}

fn reject_root_strict_config_for_subcommand(
    strict_config: bool,
    subcommand: &Option<Subcommand>,
) -> anyhow::Result<()> {
    if !strict_config {
        return Ok(());
    }

    match unsupported_subcommand_name_for_strict_config(subcommand) {
        Some(subcommand_name) => {
            reject_strict_config_for_unsupported_subcommand(strict_config, subcommand_name)
        }
        None => Ok(()),
    }
}

/// Return the selected subcommand name when a root-level `--strict-config`
/// flag should be rejected after parsing.
///
/// `--strict-config` is parsed on the root interactive CLI so commands like
/// `genex --strict-config` continue to work for the TUI and for wrappers that
/// forward root options into another command shape. Clap will still accept that
/// root flag before the dispatcher knows which subcommand the user selected, so
/// unsupported subcommands need an explicit post-parse reject path.
///
/// `Some(...)` returns the user-facing command name fragment to embed in the
/// rejection error, such as `cloud` or `app-server proxy`. `None` means the
/// selected command is allowed to inherit root `--strict-config`.
fn unsupported_subcommand_name_for_strict_config(
    subcommand: &Option<Subcommand>,
) -> Option<&'static str> {
    match subcommand {
        None
        | Some(Subcommand::Agents(_))
        | Some(Subcommand::Exec(_))
        | Some(Subcommand::Review(_))
        | Some(Subcommand::ExecServer(_))
        | Some(Subcommand::Resume(_))
        | Some(Subcommand::Queue(_))
        | Some(Subcommand::Archive(_))
        | Some(Subcommand::Delete(_))
        | Some(Subcommand::Unarchive(_))
        | Some(Subcommand::Fork(_))
        | Some(Subcommand::Doctor(_)) => None,
        Some(Subcommand::AppServer(app_server)) if app_server.subcommand.is_none() => None,
        Some(Subcommand::AppServer(app_server)) => {
            Some(app_server_subcommand_name(app_server.subcommand.as_ref()))
        }
        Some(Subcommand::Mcp(_)) => Some("mcp"),
        Some(Subcommand::Plugin(_)) => Some("plugin"),
        Some(Subcommand::Login(_)) => Some("login"),
        Some(Subcommand::Logout(_)) => Some("logout"),
        Some(Subcommand::Completion(_)) => Some("completion"),
        Some(Subcommand::Sandbox(_)) => Some("sandbox"),
        Some(Subcommand::Debug(_)) => Some("debug"),
        Some(Subcommand::Execpolicy(_)) => Some("execpolicy"),
        Some(Subcommand::StdioToUds(_)) => Some("stdio-to-uds"),
        Some(Subcommand::Features(_)) => Some("features"),
        Some(Subcommand::TcpTunnel(_)) => Some("tcp-tunnel"),
    }
}

fn reject_strict_config_for_app_server_subcommand(
    strict_config: bool,
    subcommand: Option<&AppServerSubcommand>,
) -> anyhow::Result<()> {
    if subcommand.is_none() {
        return Ok(());
    }
    reject_strict_config_for_unsupported_subcommand(
        strict_config,
        app_server_subcommand_name(subcommand),
    )
}

fn reject_strict_config_for_unsupported_subcommand(
    strict_config: bool,
    subcommand: &str,
) -> anyhow::Result<()> {
    if strict_config {
        anyhow::bail!("`--strict-config` is not supported for `genex {subcommand}`");
    }
    Ok(())
}

fn reject_remote_mode_for_app_server_subcommand(
    remote: Option<&str>,
    remote_auth_token_env: Option<&str>,
    subcommand: Option<&AppServerSubcommand>,
) -> anyhow::Result<()> {
    let subcommand_name = app_server_subcommand_name(subcommand);
    reject_remote_mode_for_subcommand(remote, remote_auth_token_env, subcommand_name)
}

fn app_server_subcommand_name(subcommand: Option<&AppServerSubcommand>) -> &'static str {
    match subcommand {
        None => "app-server",
        Some(AppServerSubcommand::Daemon(daemon)) => match daemon.subcommand {
            AppServerDaemonSubcommand::Bootstrap => "app-server daemon bootstrap",
            AppServerDaemonSubcommand::Start => "app-server daemon start",
            AppServerDaemonSubcommand::Restart => "app-server daemon restart",
            AppServerDaemonSubcommand::Stop => "app-server daemon stop",
            AppServerDaemonSubcommand::Version => "app-server daemon version",
        },
        Some(AppServerSubcommand::Proxy(_)) => "app-server proxy",
        Some(AppServerSubcommand::GenerateTs(_)) => "app-server generate-ts",
        Some(AppServerSubcommand::GenerateJsonSchema(_)) => "app-server generate-json-schema",
        Some(AppServerSubcommand::GenerateInternalJsonSchema(_)) => {
            "app-server generate-internal-json-schema"
        }
    }
}

async fn print_app_server_daemon_output(command: AppServerLifecycleCommand) -> anyhow::Result<()> {
    let output = genex_app_server_daemon::run(command).await?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

async fn mount_provider_home_before_daemon(
    config_overrides: &CliConfigOverrides,
) -> anyhow::Result<()> {
    let config = cloud_config::load_config(config_overrides, LoaderOverrides::default()).await?;
    cloud_config::mount_provider_home(&config)
}

fn read_remote_auth_token_from_env_var_with<F>(
    env_var_name: &str,
    get_var: F,
) -> anyhow::Result<String>
where
    F: FnOnce(&str) -> Result<String, std::env::VarError>,
{
    let auth_token = get_var(env_var_name)
        .map_err(|_| anyhow::anyhow!("environment variable `{env_var_name}` is not set"))?;
    let auth_token = auth_token.trim().to_string();
    if auth_token.is_empty() {
        anyhow::bail!("environment variable `{env_var_name}` is empty");
    }
    Ok(auth_token)
}

fn read_remote_auth_token_from_env_var(env_var_name: &str) -> anyhow::Result<String> {
    read_remote_auth_token_from_env_var_with(env_var_name, |name| std::env::var(name))
}

async fn run_interactive_tui(
    mut interactive: TuiCli,
    remote: Option<String>,
    remote_auth_token_env: Option<String>,
    arg0_paths: Arg0DispatchPaths,
) -> std::io::Result<AppExitInfo> {
    if interactive.no_daemon {
        if interactive.agents_overview {
            return Ok(AppExitInfo::fatal(
                "--no-daemon cannot be used with genex agents. The agents overview requires a shared server. Use genex --no-daemon to work without it.",
            ));
        }
        if remote.is_some() {
            return Ok(AppExitInfo::fatal(
                "--no-daemon cannot be used with --remote.",
            ));
        }
    }
    if let Some(prompt) = interactive.prompt.take() {
        // Normalize CRLF/CR to LF so CLI-provided text can't leak `\r` into TUI state.
        interactive.prompt = Some(prompt.replace("\r\n", "\n").replace('\r', "\n"));
    }

    let terminal_info = genex_terminal_detection::terminal_info();
    if terminal_info.name == TerminalName::Dumb {
        if !(std::io::stdin().is_terminal() && std::io::stderr().is_terminal()) {
            return Ok(AppExitInfo::fatal(
                "TERM is set to \"dumb\". Refusing to start the interactive TUI because no terminal is available for a confirmation prompt (stdin/stderr is not a TTY). Run in a supported terminal or unset TERM.",
            ));
        }

        eprintln!(
            "WARNING: TERM is set to \"dumb\". Genex's interactive TUI may not work in this terminal."
        );
        if !confirm("Continue anyway? [y/N]: ")? {
            return Ok(AppExitInfo::fatal(
                "Refusing to start the interactive TUI because TERM is set to \"dumb\". Run in a supported terminal or unset TERM.",
            ));
        }
    }

    #[cfg(unix)]
    if interactive.agents_overview && remote.is_none() {
        if !std::io::stdin().is_terminal() {
            return Ok(AppExitInfo::fatal("stdin is not a terminal"));
        }
        if !std::io::stdout().is_terminal() {
            return Ok(AppExitInfo::fatal("stdout is not a terminal"));
        }
        let config = cloud_config::load_config(
            &interactive.config_overrides,
            LoaderOverrides::default(),
        )
        .await
        .map_err(std::io::Error::other)?;
        cloud_config::mount_provider_home(&config).map_err(std::io::Error::other)?;
        genex_app_server_daemon::run(AppServerLifecycleCommand::Start)
            .await
            .map_err(|err| std::io::Error::other(format!(
                "{err:#}\nThe agents overview requires a shared server. Use genex --no-daemon to work without it."
            )))?;
    }

    let remote_endpoint = match resolve_remote_endpoint(remote, remote_auth_token_env.clone()) {
        Ok(remote_endpoint) => remote_endpoint,
        Err(err) if is_remote_auth_usage_error(&err) => {
            return Ok(AppExitInfo::fatal(err.to_string()));
        }
        Err(err) => return Err(err),
    };
    let start_tui = || {
        genex_tui::run_main(
            interactive.clone(),
            arg0_paths.clone(),
            genex_config::LoaderOverrides::default(),
            remote_endpoint.clone(),
        )
    };
    run_tui_with_recovery(start_tui, remote_auth_token_env.as_deref()).await
}

async fn run_tui_with_recovery<F, Fut>(
    mut start_tui: F,
    remote_auth_token_env: Option<&str>,
) -> std::io::Result<AppExitInfo>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::io::Result<AppExitInfo>>,
{
    let mut attempted_backups = HashSet::new();
    loop {
        // Keep the large TUI future out of the CLI dispatcher's stack frame.
        let err = match Box::pin(start_tui()).await {
            Ok(mut exit_info) => {
                if let Some(disconnect) = &mut exit_info.disconnect_info
                    && let Some(env_var) = remote_auth_token_env
                {
                    disconnect
                        .command
                        .extend(["--remote-auth-token-env".to_string(), env_var.to_string()]);
                }
                return Ok(exit_info);
            }
            Err(err) => err,
        };
        let Some(startup_error) = local_state_db::startup_error(&err) else {
            return Err(err);
        };
        if local_state_db::is_locked(startup_error.detail()) {
            local_state_db::print_locked_guidance(startup_error);
            return Ok(AppExitInfo::fatal(startup_error.to_string()));
        }
        if !local_state_db::is_auto_backup_recoverable(startup_error) {
            local_state_db::print_diagnostic_guidance(startup_error);
            return Ok(AppExitInfo::fatal(startup_error.to_string()));
        }
        if !attempted_backups.insert(startup_error.database_path().to_path_buf()) {
            local_state_db::print_diagnostic_guidance(startup_error);
            return Ok(AppExitInfo::fatal(startup_error.to_string()));
        }

        local_state_db::print_auto_backup_start(startup_error);
        match local_state_db::backup_files_for_fresh_start(startup_error).await {
            Ok(backups) => local_state_db::confirm_fresh_start_rebuild(startup_error, &backups)?,
            Err(backup_err) => {
                local_state_db::print_diagnostic_guidance(startup_error);
                return Ok(AppExitInfo::fatal(format!(
                    "failed to move damaged Genex local database files into a backup folder automatically: {backup_err}"
                )));
            }
        }
    }
}

fn resolve_remote_endpoint(
    remote: Option<String>,
    remote_auth_token_env: Option<String>,
) -> std::io::Result<Option<genex_tui::RemoteAppServerEndpoint>> {
    let mut remote_endpoint = remote
        .as_deref()
        .map(genex_tui::resolve_remote_addr)
        .transpose()
        .map_err(std::io::Error::other)?;
    if let Some(remote_auth_token_env) = remote_auth_token_env {
        let Some(endpoint) = remote_endpoint.as_mut() else {
            return Err(std::io::Error::other(
                "`--remote-auth-token-env` requires `--remote`.",
            ));
        };
        if !genex_tui::remote_addr_supports_auth_token(endpoint) {
            return Err(std::io::Error::other(
                "`--remote-auth-token-env` requires a `wss://` or loopback `ws://` remote.",
            ));
        }
        let auth_token = read_remote_auth_token_from_env_var(&remote_auth_token_env)
            .map_err(std::io::Error::other)?;
        let genex_tui::RemoteAppServerEndpoint::WebSocket {
            auth_token: slot, ..
        } = endpoint
        else {
            return Err(std::io::Error::other(
                "`--remote-auth-token-env` requires a `wss://` or loopback `ws://` remote.",
            ));
        };
        *slot = Some(auth_token);
    }
    Ok(remote_endpoint)
}

fn is_remote_auth_usage_error(err: &std::io::Error) -> bool {
    err.to_string()
        .starts_with("`--remote-auth-token-env` requires")
}

fn confirm(prompt: &str) -> std::io::Result<bool> {
    eprintln!("{prompt}");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let answer = input.trim();
    Ok(answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes"))
}

/// Build the final `TuiCli` for a `genex resume` invocation.
fn finalize_resume_interactive(
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    session_id: Option<String>,
    last: bool,
    show_all: bool,
    include_non_interactive: bool,
    mut resume_cli: TuiCli,
) -> TuiCli {
    // Start with the parsed interactive CLI so resume shares the same
    // configuration surface area as `genex` without additional flags.
    // Clap assigns the first positional to `session_id`. With `--last`, reinterpret it as the
    // prompt when no second positional prompt was provided.
    let resume_session_id = if last && resume_cli.prompt.is_none() {
        resume_cli.prompt = session_id;
        None
    } else {
        session_id
    };
    interactive.resume_picker = resume_session_id.is_none() && !last;
    interactive.resume_last = last;
    interactive.resume_session_id = resume_session_id;
    interactive.resume_show_all = show_all;
    interactive.resume_include_non_interactive = include_non_interactive;

    // Merge resume-scoped flags and overrides with highest precedence.
    merge_interactive_cli_flags(&mut interactive, resume_cli);

    // Propagate any root-level config overrides (e.g. `-c key=value`).
    prepend_config_flags(&mut interactive.config_overrides, root_config_overrides);

    interactive
}

/// Build the final `TuiCli` for a `genex fork` invocation.
fn finalize_fork_interactive(
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    session_id: Option<String>,
    last: bool,
    show_all: bool,
    mut fork_cli: TuiCli,
) -> TuiCli {
    // Start with the parsed interactive CLI so fork shares the same
    // configuration surface area as `genex` without additional flags.
    // Clap assigns the first positional to `session_id`. With `--last`, reinterpret it as the
    // prompt when no second positional prompt was provided.
    let fork_session_id = if last && fork_cli.prompt.is_none() {
        fork_cli.prompt = session_id;
        None
    } else {
        session_id
    };
    interactive.fork_picker = fork_session_id.is_none() && !last;
    interactive.fork_last = last;
    interactive.fork_session_id = fork_session_id;
    interactive.fork_show_all = show_all;

    // Merge fork-scoped flags and overrides with highest precedence.
    merge_interactive_cli_flags(&mut interactive, fork_cli);

    // Propagate any root-level config overrides (e.g. `-c key=value`).
    prepend_config_flags(&mut interactive.config_overrides, root_config_overrides);

    interactive
}

fn finalize_session_archive_interactive(
    mut interactive: TuiCli,
    root_config_overrides: CliConfigOverrides,
    archive_cli: SessionArchiveConfigOverrides,
) -> TuiCli {
    let SessionArchiveConfigOverrides {
        shared,
        strict_config,
        config_overrides,
    } = archive_cli;
    interactive.shared.apply_subcommand_overrides(shared);
    if strict_config {
        interactive.strict_config = true;
    }
    interactive
        .config_overrides
        .raw_overrides
        .extend(config_overrides.raw_overrides);
    prepend_config_flags(&mut interactive.config_overrides, root_config_overrides);
    interactive
}

/// Merge flags provided to runtime wrapper commands so they take precedence over any root-level
/// flags. Only overrides fields explicitly set on the subcommand-scoped CLI. Also appends
/// `-c key=value` overrides with highest precedence.
fn merge_interactive_cli_flags(interactive: &mut TuiCli, subcommand_cli: TuiCli) {
    let TuiCli {
        shared,
        strict_config,
        approval_policy,
        web_search,
        no_alt_screen,
        no_daemon,
        prompt,
        config_overrides,
        ..
    } = subcommand_cli;
    interactive
        .shared
        .apply_subcommand_overrides(shared.into_inner());
    if let Some(approval) = approval_policy {
        interactive.approval_policy = Some(approval);
    }
    if web_search {
        interactive.web_search = true;
    }
    interactive.no_alt_screen |= no_alt_screen;
    interactive.no_daemon |= no_daemon;
    if strict_config {
        interactive.strict_config = true;
    }
    if let Some(prompt) = prompt {
        // Normalize CRLF/CR to LF so CLI-provided text can't leak `\r` into TUI state.
        interactive.prompt = Some(prompt.replace("\r\n", "\n").replace('\r', "\n"));
    }

    interactive
        .config_overrides
        .raw_overrides
        .extend(config_overrides.raw_overrides);
}

fn print_completion(cmd: CompletionCommand) {
    let mut app = MultitoolCli::command();
    let name = "genex";
    generate(cmd.shell, &mut app, name, &mut std::io::stdout());
}
