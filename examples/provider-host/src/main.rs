use ruxex_app_server::AppServerRuntime;
use ruxex_app_server::AppServerRuntimeBuilder;
use ruxex_app_server::ProviderDescriptor;
use ruxex_app_server::ProviderInit;
use ruxex_arg0::Arg0DispatchPaths;
use ruxex_arg0::arg0_dispatch_or_else;
use ruxex_cli::APP_SERVER_DAEMON_MODE_ARG;
use ruxex_cli::run_host_app_server_daemon;
use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use ruxex_model_provider_info::CLOUDFLARE_PROVIDER_ID;
use ruxex_providers::CloudflareModelProvider;

const TEST_PROVIDER_INFO_ENV: &str = "RUXEX_TEST_PROVIDER_INFO";

fn test_provider_descriptor() -> ProviderDescriptor {
    ProviderDescriptor::new(CLOUDFLARE_PROVIDER_ID, CURRENT_PROVIDER_PROTOCOL_VERSION)
}

fn test_provider_runtime(init: ProviderInit) -> AppServerRuntime {
    let provider_info = serde_json::from_str(
        &std::env::var(TEST_PROVIDER_INFO_ENV)
            .expect("RUXEX_TEST_PROVIDER_INFO must be set for the test provider host"),
    )
    .expect("RUXEX_TEST_PROVIDER_INFO must contain a ModelProviderInfo JSON object");
    AppServerRuntimeBuilder::from_provider_info_with_http_client_factory(
        provider_info,
        Some(init.credential_manager),
        init.http_client_factory,
    )
    .build()
}

fn test_provider_factory() -> ruxex_cli::ProviderRuntimeFactory {
    (test_provider_runtime, test_provider_descriptor())
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|arg0_paths: Arg0DispatchPaths| async move {
        let test_provider = std::env::var_os(TEST_PROVIDER_INFO_ENV).is_some();
        if std::env::args_os().any(|arg| arg == APP_SERVER_DAEMON_MODE_ARG) {
            let exit = if test_provider {
                ruxex_cli::run_host_app_server_daemon_with_provider_factory(
                    arg0_paths,
                    Default::default(),
                    Default::default(),
                    false,
                    test_provider_factory(),
                )
                .await?
            } else {
                run_host_app_server_daemon::<CloudflareModelProvider>(
                    arg0_paths,
                    Default::default(),
                    Default::default(),
                    false,
                )
                .await?
            };
            if exit == ruxex_app_server::AppServerExit::Forced {
                std::process::exit(0);
            }
        } else if test_provider {
            ruxex_cli::run_with_provider_factory(arg0_paths, test_provider_factory()).await?;
        } else {
            ruxex_cli::run_with_provider::<CloudflareModelProvider>(arg0_paths).await?;
        }
        Ok(())
    })
}
