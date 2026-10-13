use genex_arg0::Arg0DispatchPaths;
use genex_arg0::arg0_dispatch_or_else;
use genex_cli::APP_SERVER_DAEMON_MODE_ARG;
use genex_cli::run_host_app_server_daemon;
use genex_providers::CloudflareModelProvider;

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        if std::env::args_os().any(|arg| arg == APP_SERVER_DAEMON_MODE_ARG) {
            let exit = run_host_app_server_daemon::<CloudflareModelProvider>(
                arg0_paths,
                Default::default(),
                Default::default(),
                false,
            )
            .await?;
            if exit == genex_app_server::AppServerExit::Forced {
                std::process::exit(0);
            }
        } else {
            genex_cli::run_with_provider::<CloudflareModelProvider>(arg0_paths).await?;
        }
        Ok(())
    })
}
