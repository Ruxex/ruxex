//! Shared provider/runtime construction for all AppServer transports.

use std::sync::Arc;

use anyhow::Context;
use genex_core::config::Config;
use genex_http_client::HttpClientFactory;
use genex_http_client::OutboundProxyPolicy;
use genex_login::CredentialManager;
use genex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;
use genex_model_provider::ProviderDescriptor;
use genex_model_provider::ProviderHomeError;
use genex_model_provider::ProviderHomeMarker;
use genex_model_provider::SharedModelProvider;
use genex_model_provider::create_model_provider_with_http_client_factory;
use genex_model_provider::mount_provider_home;
use genex_model_provider_info::ModelProviderInfo;

/// The single provider contract shared by an AppServer transport and its clients.
#[derive(Clone)]
pub struct AppServerRuntime {
    provider: SharedModelProvider,
    credential_manager: Option<Arc<CredentialManager>>,
}

impl AppServerRuntime {
    /// Returns the single provider owned by this runtime.
    pub fn provider(&self) -> SharedModelProvider {
        Arc::clone(&self.provider)
    }

    pub fn credential_manager(&self) -> Option<Arc<CredentialManager>> {
        self.credential_manager.clone()
    }

    /// Returns the descriptor used for provider-home mount validation.
    pub fn provider_descriptor(&self) -> ProviderDescriptor {
        self.provider.descriptor()
    }
}

/// Constructs the shared provider/runtime state used by an AppServer.
pub struct AppServerRuntimeBuilder {
    provider: SharedModelProvider,
    credential_manager: Option<Arc<CredentialManager>>,
}

/// Mounts the provider-scoped data root for a configured host provider.
///
/// This must run before the caller opens SQLite, sessions, caches, or credentials.
pub fn mount_provider_home_for_config(
    config: &Config,
) -> Result<ProviderHomeMarker, ProviderHomeError> {
    let descriptor = AppServerRuntimeBuilder::descriptor_for(&config.model_provider);
    mount_provider_home(&config.genex_home, &descriptor)
}

/// Mounts the provider home and constructs the single runtime owned by a local host.
pub async fn configured_app_server_runtime_from_config(
    config: &Config,
    enable_api_key_env: bool,
) -> anyhow::Result<AppServerRuntime> {
    mount_provider_home_for_config(config).context("failed to mount provider data root")?;
    let credential_manager = CredentialManager::shared_from_config(config, enable_api_key_env)
        .await
        .context("failed to load provider credentials")?;
    Ok(
        AppServerRuntimeBuilder::from_provider_info_with_http_client_factory(
            config.model_provider.clone(),
            Some(credential_manager),
            config.http_client_factory(),
        )
        .build(),
    )
}

impl AppServerRuntimeBuilder {
    /// Starts building an AppServer runtime from its single provider
    /// implementation.
    pub fn new(provider: SharedModelProvider) -> Self {
        Self {
            provider,
            credential_manager: None,
        }
    }

    /// Starts building a runtime for the provider selected by host configuration.
    ///
    /// Provider selection belongs at this host boundary. Callers that already own a concrete
    /// provider can use [`Self::new`].
    pub fn from_provider_info(
        provider_info: ModelProviderInfo,
        credential_manager: Option<Arc<CredentialManager>>,
    ) -> Self {
        Self::from_provider_info_with_http_client_factory(
            provider_info,
            credential_manager.clone(),
            HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        )
    }

    /// Starts building a configured runtime with the host's outbound transport policy.
    pub fn from_provider_info_with_http_client_factory(
        provider_info: ModelProviderInfo,
        credential_manager: Option<Arc<CredentialManager>>,
        http_client_factory: HttpClientFactory,
    ) -> Self {
        let provider = create_model_provider_with_http_client_factory(
            provider_info,
            credential_manager.clone(),
            http_client_factory,
        );
        Self {
            provider,
            credential_manager,
        }
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
            credential_manager: self.credential_manager,
        }
    }
}

/// Creates the configured AppServer runtime for startup paths that do not
/// receive an already-built runtime from their host.
pub(crate) fn configured_app_server_runtime(
    provider_info: ModelProviderInfo,
    credential_manager: Arc<CredentialManager>,
    http_client_factory: HttpClientFactory,
) -> AppServerRuntime {
    AppServerRuntimeBuilder::from_provider_info_with_http_client_factory(
        provider_info,
        Some(credential_manager),
        http_client_factory,
    )
    .build()
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

    #[test]
    fn contract_is_derived_from_the_runtime_provider() {
        let runtime =
            AppServerRuntimeBuilder::from_provider_info(ModelProviderInfo::default(), None).build();

        assert_eq!(
            runtime.provider_descriptor(),
            runtime.provider().descriptor()
        );
    }

    #[tokio::test]
    async fn config_home_mount_uses_the_configured_provider_descriptor() {
        let temp = tempfile::tempdir().expect("temporary provider home");
        let config = genex_core::config::ConfigBuilder::default()
            .genex_home(temp.path().to_path_buf())
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
        let config = genex_core::config::ConfigBuilder::default()
            .genex_home(temp.path().to_path_buf())
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
    async fn configured_runtime_rejects_mismatched_marker_before_credential_loading() {
        let temp = tempfile::tempdir().expect("temporary provider home");
        let config = genex_core::config::ConfigBuilder::default()
            .genex_home(temp.path().to_path_buf())
            .build()
            .await
            .expect("config should build");
        std::fs::write(temp.path().join("state.sqlite"), b"existing state")
            .expect("state fixture should be written");
        mount_provider_home(
            temp.path(),
            &ProviderDescriptor::new("different-provider", CURRENT_PROVIDER_PROTOCOL_VERSION),
        )
        .expect("fixture marker should be written");

        let error = match configured_app_server_runtime_from_config(&config, true).await {
            Ok(_) => panic!("mismatched marker must reject runtime startup"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("incompatible marker"));
    }
}
