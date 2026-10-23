//! Shared provider/runtime construction for all AppServer transports.

use std::sync::Arc;

use anyhow::Context;
use ruxex_core::config::Config;
use ruxex_login::CredentialManager;
use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use ruxex_model_provider::ModelProviderFactory;
#[cfg(test)]
use ruxex_model_provider::PROVIDER_HOME_MARKER_FILE;
use ruxex_model_provider::ProviderDescriptor;
use ruxex_model_provider::ProviderHomeError;
use ruxex_model_provider::ProviderHomeMarker;
use ruxex_model_provider::ProviderInit;
use ruxex_model_provider::SharedModelProvider;
use ruxex_model_provider::mount_provider_home;
use ruxex_model_provider::mount_provider_home_with_marker;
use ruxex_model_provider_info::ModelProviderInfo;

/// The single provider contract shared by an AppServer transport and its clients.
#[derive(Clone)]
pub struct AppServerRuntime {
    provider: SharedModelProvider,
}

impl AppServerRuntime {
    /// Returns the single provider owned by this runtime.
    pub fn provider(&self) -> SharedModelProvider {
        Arc::clone(&self.provider)
    }

    /// Returns the descriptor used for provider-home mount validation.
    pub fn provider_descriptor(&self) -> ProviderDescriptor {
        self.provider.descriptor()
    }
}

/// Constructs the shared provider/runtime state used by an AppServer.
pub struct AppServerRuntimeBuilder {
    provider: SharedModelProvider,
}

/// Mounts the provider-scoped data root for a configured host provider.
///
/// This must run before the caller opens SQLite, sessions, caches, or credentials.
pub fn mount_provider_home_for_config(
    config: &Config,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    mount_provider_home_for_config_with_marker(
        config,
        ruxex_model_provider::PROVIDER_HOME_MARKER_FILE,
    )
}

/// Mounts a configured provider home with a product-specific marker filename.
pub fn mount_provider_home_for_config_with_marker(
    config: &Config,
    marker_file: impl AsRef<std::path::Path>,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    let descriptor = AppServerRuntimeBuilder::descriptor_for(&config.model_provider);
    mount_provider_home_with_marker(&config.ruxex_home, marker_file, &descriptor)
}

/// Mounts the provider-scoped data root for an already-injected runtime.
///
/// This is the startup boundary for hosts that select a concrete provider
/// themselves. It must run before credentials, SQLite, sessions, or caches are
/// opened.
pub fn mount_provider_home_for_runtime(
    config: &Config,
    runtime: &AppServerRuntime,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    mount_provider_home_for_runtime_with_marker(
        config,
        runtime,
        ruxex_model_provider::PROVIDER_HOME_MARKER_FILE,
    )
}

/// Mounts an injected provider home with a product-specific marker filename.
pub fn mount_provider_home_for_runtime_with_marker(
    config: &Config,
    runtime: &AppServerRuntime,
    marker_file: impl AsRef<std::path::Path>,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    mount_provider_home_with_marker(
        &config.ruxex_home,
        marker_file,
        &runtime.provider_descriptor(),
    )
}

/// Mounts the provider home using the host-selected provider identity and
/// loads the process-local inputs for a provider constructor.
pub async fn provider_init_from_config_with_descriptor(
    config: &Config,
    enable_api_key_env: bool,
    descriptor: &ProviderDescriptor,
) -> anyhow::Result<ProviderInit> {
    mount_provider_home(&config.ruxex_home, descriptor)
        .context("failed to mount provider data root")?;
    let credential_manager = CredentialManager::shared_from_config(config, enable_api_key_env)
        .await
        .context("failed to load provider credentials")?;
    Ok(ProviderInit {
        provider_info: config.model_provider.clone(),
        credential_manager,
        http_client_factory: config.http_client_factory(),
    })
}

impl AppServerRuntimeBuilder {
    /// Starts building an AppServer runtime from its single provider
    /// implementation.
    pub fn new(provider: SharedModelProvider) -> Self {
        Self { provider }
    }

    /// Starts a runtime by constructing the host-selected provider type.
    pub fn from_provider_factory<P: ModelProviderFactory>(init: ProviderInit) -> Self {
        Self::new(Arc::new(P::new(init)))
    }

    /// Constructs a complete runtime from the host-selected provider type.
    pub fn build_from_provider_factory<P: ModelProviderFactory>(
        init: ProviderInit,
    ) -> AppServerRuntime {
        Self::from_provider_factory::<P>(init).build()
    }

    /// Returns the descriptor that must be used when mounting this configured provider's data
    /// root, without constructing credentials or opening provider state.
    pub fn descriptor_for(provider_info: &ModelProviderInfo) -> ProviderDescriptor {
        configured_provider_descriptor(provider_info)
    }

    /// Builds the runtime around the injected provider.
    pub fn build(self) -> AppServerRuntime {
        AppServerRuntime {
            provider: self.provider,
        }
    }
}

pub(crate) fn configured_provider_descriptor(
    provider_info: &ModelProviderInfo,
) -> ProviderDescriptor {
    ProviderDescriptor::new(
        provider_info
            .id
            .as_deref()
            .unwrap_or(provider_info.name.as_str()),
        CURRENT_PROVIDER_PROTOCOL_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruxex_model_provider::SharedModelProvider;

    fn test_provider_with_id(provider_id: &str) -> SharedModelProvider {
        let mut provider_info = ModelProviderInfo::create_responses_provider(None);
        provider_info.id = Some(provider_id.to_string());
        ruxex_providers::create_responses_provider(provider_info, None)
    }

    fn test_provider() -> SharedModelProvider {
        test_provider_with_id("test-provider")
    }

    #[test]
    fn contract_is_derived_from_the_runtime_provider() {
        let runtime = AppServerRuntimeBuilder::new(test_provider()).build();

        assert_eq!(
            runtime.provider_descriptor(),
            runtime.provider().descriptor()
        );
    }

    #[tokio::test]
    async fn config_home_mount_uses_the_configured_provider_descriptor() {
        let temp = tempfile::tempdir().expect("temporary provider home");
        let config = ruxex_core::config::ConfigBuilder::default()
            .ruxex_home(temp.path().to_path_buf())
            .build()
            .await
            .expect("config should build");
        let expected = AppServerRuntimeBuilder::descriptor_for(&config.model_provider);

        let marker =
            mount_provider_home_for_config(&config).expect("configured provider home should mount");

        assert_eq!(marker.provider_kind, expected.provider_kind);
        assert_eq!(
            marker.provider_protocol_version,
            expected.provider_protocol_version
        );
    }

    #[tokio::test]
    async fn config_home_mount_rejects_a_mismatched_provider_marker() {
        let temp = tempfile::tempdir().expect("temporary provider home");
        let config = ruxex_core::config::ConfigBuilder::default()
            .ruxex_home(temp.path().to_path_buf())
            .build()
            .await
            .expect("config should build");
        mount_provider_home(
            temp.path(),
            &ProviderDescriptor::new("different-provider", CURRENT_PROVIDER_PROTOCOL_VERSION),
        )
        .expect("fixture marker should be written");

        assert!(matches!(
            mount_provider_home_for_config(&config),
            Err(ProviderHomeError::DescriptorMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn injected_runtime_home_mount_uses_runtime_descriptor() {
        let temp = tempfile::tempdir().expect("temporary provider home");
        let config = ruxex_core::config::ConfigBuilder::default()
            .ruxex_home(temp.path().to_path_buf())
            .build()
            .await
            .expect("config should build");
        let runtime =
            AppServerRuntimeBuilder::new(test_provider_with_id("injected-provider")).build();

        let marker = mount_provider_home_for_runtime(&config, &runtime)
            .expect("injected provider home should mount");

        assert_eq!(marker.provider_kind, "injected-provider");
    }

    #[tokio::test]
    async fn configured_runtime_rejects_mismatched_marker_before_credential_loading() {
        let temp = tempfile::tempdir().expect("temporary provider home");
        let config = ruxex_core::config::ConfigBuilder::default()
            .ruxex_home(temp.path().to_path_buf())
            .build()
            .await
            .expect("config should build");
        let expected = AppServerRuntimeBuilder::descriptor_for(&config.model_provider);
        mount_provider_home(temp.path(), &expected).expect("expected fixture marker should exist");
        std::fs::write(temp.path().join("state.sqlite"), b"existing state")
            .expect("state fixture should be written");
        let different =
            ProviderDescriptor::new("different-provider", CURRENT_PROVIDER_PROTOCOL_VERSION);
        let different_marker = ProviderHomeMarker::from(&different);
        std::fs::write(
            temp.path().join(PROVIDER_HOME_MARKER_FILE),
            serde_json::to_vec_pretty(&different_marker).expect("fixture marker should serialize"),
        )
        .expect("fixture marker should be replaced");

        let error = match provider_init_from_config_with_descriptor(&config, true, &expected).await
        {
            Ok(_) => panic!("mismatched marker must reject runtime startup"),
            Err(error) => error,
        };

        assert!(format!("{error:#}").contains("incompatible marker"));
    }
}
