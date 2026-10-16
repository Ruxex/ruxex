//! Resolves managed fork sources through the existing name/ID lookup before final configuration.
//! The temporary app-server never starts a thread or executes a turn.

use super::*;
use ruxex_app_server_client::AppServerRuntime;
use ruxex_app_server_client::configured_app_server_runtime_from_config;
use ruxex_app_server_client::provider_init_from_config_with_descriptor;

pub type ProviderRuntimeFactory = (fn(ProviderInit) -> AppServerRuntime, ProviderDescriptor);

pub(super) async fn fork_source(
    args: &mut crate::cli::ForkArgs,
    config: &Config,
    arg0_paths: &Arg0DispatchPaths,
    cli_overrides: &[(String, ruxex_config::TomlValue)],
    loader_overrides: &LoaderOverrides,
    strict_config: bool,
    provider_factory: Option<ProviderRuntimeFactory>,
) -> anyhow::Result<std::path::PathBuf> {
    let runtime = if let Some(provider_factory) = provider_factory {
        let init =
            provider_init_from_config_with_descriptor(config, true, &provider_factory.1).await?;
        (provider_factory.0)(init)
    } else {
        configured_app_server_runtime_from_config(config, true).await?
    };
    let state_db = ruxex_core::init_state_db(config).await;
    let environment_manager = EnvironmentManager::from_ruxex_home(
        config.ruxex_home.clone(),
        Some(ExecServerRuntimePaths::from_optional_paths(
            arg0_paths.ruxex_self_exe.clone(),
            arg0_paths.ruxex_linux_sandbox_exe.clone(),
        )?),
        config.http_client_factory(),
    )
    .await?;
    let client = InProcessAppServerClient::start(InProcessClientStartArgs {
        arg0_paths: arg0_paths.clone(),
        config: std::sync::Arc::new(config.clone()),
        cli_overrides: cli_overrides.to_vec(),
        loader_overrides: LoaderOverrides {
            ignore_project_config: true,
            ..loader_overrides.clone()
        },
        strict_config,
        log_db: None,
        state_db: state_db.clone(),
        environment_manager: std::sync::Arc::new(environment_manager),
        config_warnings: Vec::new(),
        session_source: SessionSource::Exec,
        enable_api_key_env: true,
        runtime: Some(runtime),
        client_name: "ruxex_exec".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: true,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await?;
    let result = async {
        let lookup = crate::cli::ResumeArgs {
            session_id: Some(args.session_id.clone()),
            last: false,
            all: true,
            images: Vec::new(),
            prompt: None,
        };
        let thread_id = resolve_resume_thread_id(&client, config, state_db.as_ref(), &lookup)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", args.session_id))?;
        let source: ThreadReadResponse = send_request_with_response(
            &client,
            ClientRequest::ThreadRead {
                request_id: RequestId::Integer(1),
                params: ThreadReadParams {
                    thread_id: thread_id.clone(),
                    include_turns: false,
                },
            },
            "thread/read",
        )
        .await
        .map_err(anyhow::Error::msg)?;
        args.session_id = thread_id;
        Ok::<_, anyhow::Error>(latest_thread_cwd(&source.thread).await)
    }
    .await;
    let shutdown = client.shutdown().await;
    let source_cwd = result?;
    shutdown?;
    Ok(source_cwd)
}
