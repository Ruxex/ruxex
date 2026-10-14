use anyhow::Context;
use anyhow::Result;
use ruxex_app_server::runtime::mount_provider_home_for_config;
use ruxex_core::config::Config;
use ruxex_core::config::ConfigBuilder;
use ruxex_core::config::ConfigOverrides;
use ruxex_core::config::LoaderOverrides;
use ruxex_core::config::find_ruxex_home;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_cli::CliConfigOverrides;

pub(crate) async fn load_config(
    config_overrides: &CliConfigOverrides,
    loader_overrides: LoaderOverrides,
) -> Result<Config> {
    config_builder(
        config_overrides,
        loader_overrides,
        ConfigOverrides::default(),
    )
    .await?
    .build()
    .await
    .context("failed to load configuration")
}

/// Validate the provider-scoped data root before a CLI command opens provider credentials.
pub(crate) fn mount_provider_home(config: &Config) -> Result<()> {
    mount_provider_home_for_config(config)
        .map(|_| ())
        .context("failed to mount provider data root")
}

pub(crate) async fn config_builder(
    config_overrides: &CliConfigOverrides,
    loader_overrides: LoaderOverrides,
    harness_overrides: ConfigOverrides,
) -> Result<ConfigBuilder> {
    let cli_overrides = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let ruxex_home = find_ruxex_home().context("failed to resolve RUXEX_HOME")?;
    let cwd = match harness_overrides.cwd.as_deref() {
        Some(cwd) => AbsolutePathBuf::relative_to_current_dir(cwd),
        None => AbsolutePathBuf::current_dir(),
    }
    .context("failed to resolve current directory")?;
    Ok(ConfigBuilder::default()
        .ruxex_home(ruxex_home.to_path_buf())
        .cli_overrides(cli_overrides)
        .loader_overrides(loader_overrides)
        .harness_overrides(harness_overrides)
        .fallback_cwd(Some(cwd.to_path_buf())))
}
