use genex_arg0::Arg0DispatchPaths;
use genex_arg0::arg0_dispatch_or_else;
pub use genex_cli::LandlockCommand;
pub use genex_cli::SeatbeltCommand;
pub use genex_cli::run_command_under_landlock;
pub use genex_cli::run_command_under_seatbelt;
pub use genex_cli::run_host_app_server_lifecycle;
pub use genex_cli::run_login_delete;
pub use genex_cli::run_login_status;
pub use genex_cli::run_logout;

mod cloud_config;
mod doctor;
mod exec_server_telemetry;
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

fn main() -> anyhow::Result<()> {
    genex_build_info::initialize!();
    arg0_dispatch_or_else(move |arg0_paths: Arg0DispatchPaths| async move {
        cli_main::run_legacy(arg0_paths).await?;
        Ok(())
    })
}
