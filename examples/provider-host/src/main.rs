use clap::Parser;
use genex_arg0::Arg0DispatchPaths;
use genex_arg0::arg0_dispatch_or_else;
use genex_cli::APP_SERVER_DAEMON_MODE_ARG;
use genex_cli::run_host_app_server_daemon;
use genex_config::LoaderOverrides;
use genex_providers::CloudflareModelProvider;
use genex_tui::Cli;
use genex_utils_cli::CliConfigOverrides;

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        if std::env::args_os().any(|arg| arg == APP_SERVER_DAEMON_MODE_ARG) {
            let exit = run_host_app_server_daemon::<CloudflareModelProvider>(
                arg0_paths,
                CliConfigOverrides::default(),
                LoaderOverrides::default(),
                false,
            )
            .await?;
            if exit == genex_app_server::AppServerExit::Forced {
                std::process::exit(0);
            }
        } else {
            let exit = genex_tui::run_main_with_provider::<CloudflareModelProvider>(
                Cli::parse(),
                arg0_paths,
                LoaderOverrides::default(),
                None,
            )
            .await?;
            let is_fatal = matches!(exit.exit_reason, genex_tui::ExitReason::Fatal(_));
            for line in exit.format_exit_messages(false) {
                println!("{line}");
            }
            if is_fatal {
                std::process::exit(1);
            }
        }
        Ok(())
    })
}
