use anyhow::Context;
use anyhow::Result;
use genex_core::config::Config;
use genex_core::config::ConfigBuilder;
use genex_core::config::ConfigOverrides;
use genex_core::config::LoaderOverrides;
use genex_core::config::find_genex_home;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_cli::CliConfigOverrides;

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

pub(crate) async fn config_builder(
    config_overrides: &CliConfigOverrides,
    loader_overrides: LoaderOverrides,
    harness_overrides: ConfigOverrides,
) -> Result<ConfigBuilder> {
    let cli_overrides = config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let genex_home = find_genex_home().context("failed to resolve GENEX_HOME")?;
    let cwd = match harness_overrides.cwd.as_deref() {
        Some(cwd) => AbsolutePathBuf::relative_to_current_dir(cwd),
        None => AbsolutePathBuf::current_dir(),
    }
    .context("failed to resolve current directory")?;
    Ok(ConfigBuilder::default()
        .genex_home(genex_home.to_path_buf())
        .cli_overrides(cli_overrides)
        .loader_overrides(loader_overrides)
        .harness_overrides(harness_overrides)
        .fallback_cwd(Some(cwd.to_path_buf())))
}
