mod cloudflare;

use cloudflare::CloudflareProvider;
use ruxex::provider::credentials::ProviderCredentialResolverRegistry;
use ruxex::provider::transport::HttpClientFactory;
use ruxex::provider::transport::OutboundProxyPolicy;
use ruxex_app_server::AppServerRuntime;
use ruxex_app_server::AppServerRuntimeBuilder;
use ruxex_app_server::ProviderDescriptor;
use ruxex_app_server::ProviderInit;
use ruxex_arg0::Arg0DispatchPaths;
use ruxex_arg0::arg0_dispatch_or_else;
use ruxex_cli::APP_SERVER_DAEMON_MODE_ARG;
use ruxex_cli::run_host_app_server_daemon;
use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use ruxex_model_provider::ModelProviderFactory;
use ruxex_model_provider_info::ModelProviderInfo;
use ruxex_providers::ResponsesModelProvider;

const TEST_PROVIDER_INFO_ENV: &str = "RUXEX_TEST_PROVIDER_INFO";

fn test_provider_descriptor() -> ProviderDescriptor {
    let provider_info: ModelProviderInfo = serde_json::from_str(
        &std::env::var(TEST_PROVIDER_INFO_ENV)
            .expect("RUXEX_TEST_PROVIDER_INFO must be set for the test provider host"),
    )
    .expect("RUXEX_TEST_PROVIDER_INFO must contain a ModelProviderInfo JSON object");
    ProviderDescriptor::new(
        provider_info
            .id
            .as_deref()
            .unwrap_or(provider_info.name.as_str()),
        CURRENT_PROVIDER_PROTOCOL_VERSION,
    )
}

fn test_provider_runtime(init: ProviderInit) -> AppServerRuntime {
    let provider_info: ModelProviderInfo = serde_json::from_str(
        &std::env::var(TEST_PROVIDER_INFO_ENV)
            .expect("RUXEX_TEST_PROVIDER_INFO must be set for the test provider host"),
    )
    .expect("RUXEX_TEST_PROVIDER_INFO must contain a ModelProviderInfo JSON object");
    AppServerRuntimeBuilder::build_from_provider_factory::<ResponsesModelProvider>(ProviderInit {
        provider_info,
        ..init
    })
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
                run_host_app_server_daemon::<CloudflareProvider>(
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
            ruxex_cli::run_with_provider_factory(arg0_paths, test_provider_factory(), None).await?;
        } else {
            let mut resolvers = ProviderCredentialResolverRegistry::new();
            resolvers.register_typed::<ruxex_providers::CloudflareCredentials, _>(
                ruxex_providers::create_cloudflare_credential_resolver(HttpClientFactory::new(
                    OutboundProxyPolicy::ReqwestDefault,
                )),
            )?;
            ruxex_cli::run_with_provider_factory_and_identity_and_resolvers(
                arg0_paths,
                (
                    AppServerRuntimeBuilder::build_from_provider_factory::<CloudflareProvider>,
                    CloudflareProvider::provider_descriptor(),
                ),
                Some(resolvers),
                ruxex::ProductIdentity::ruxex(),
                None,
            )
            .await?;
        }
        Ok(())
    })
}
