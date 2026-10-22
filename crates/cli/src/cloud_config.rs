use anyhow::Context;
use anyhow::Result;
use ruxex_core::config::Config;
use ruxex_core::config::ConfigBuilder;
use ruxex_core::config::ConfigOverrides;
use ruxex_core::config::LoaderOverrides;
use ruxex_core::config::find_ruxex_home;
use ruxex_model_provider::ProviderDescriptor;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_cli::CliConfigOverrides;
use ruxex_utils_home_dir::ProductIdentity;

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
pub(crate) fn mount_provider_home_for_provider(
    config: &Config,
    provider_descriptor: &ProviderDescriptor,
    product_identity: &ProductIdentity,
) -> Result<ruxex_model_provider::ProviderHomeMarker> {
    ruxex_model_provider::mount_provider_home_with_marker(
        &config.ruxex_home,
        product_identity.home_marker_file(),
        provider_descriptor,
    )
    .context("failed to mount provider data root")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;

    #[tokio::test]
    async fn host_provider_home_mount_uses_injected_descriptor() {
        let temp = tempfile::tempdir().expect("temporary provider home");
        let config = ConfigBuilder::default()
            .ruxex_home(temp.path().to_path_buf())
            .build()
            .await
            .expect("config should build");
        let descriptor =
            ProviderDescriptor::new("host-provider", CURRENT_PROVIDER_PROTOCOL_VERSION);

        let marker =
            mount_provider_home_for_provider(&config, &descriptor, &ProductIdentity::ruxex())
                .expect("host provider home should mount");

        assert_eq!(marker.provider_kind, "host-provider");
    }
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
