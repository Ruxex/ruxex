use std::sync::Arc;

use genex_api::AllowedCaller;
use genex_api::ApproximateLocation;
use genex_api::ExternalWebAccess;
use genex_api::ExternalWebAccessMode;
use genex_api::LocationType;
use genex_api::SearchContextSize;
use genex_api::SearchFilters;
use genex_api::SearchSettings;
use genex_core::config::Config;
use genex_extension_api::ConfigContributor;
use genex_extension_api::ExtensionData;
use genex_extension_api::ExtensionFuture;
use genex_extension_api::ExtensionRegistryBuilder;
use genex_extension_api::ThreadLifecycleContributor;
use genex_extension_api::ThreadOriginator;
use genex_extension_api::ThreadStartInput;
use genex_extension_api::ToolContributor;
use genex_login::CredentialManager;
use genex_model_provider::create_model_provider_for_id;
use genex_model_provider_info::ModelProviderInfo;
use genex_protocol::config_types::WebSearchContextSize;
use genex_protocol::config_types::WebSearchMode;

use crate::tool::WebSearchTool;

#[derive(Clone)]
struct WebSearchExtension {
    credential_manager: Arc<CredentialManager>,
}

#[derive(Clone)]
struct WebSearchExtensionConfig {
    available: bool,
    provider_id: String,
    provider: ModelProviderInfo,
    settings: SearchSettings,
}

impl From<&Config> for WebSearchExtensionConfig {
    fn from(config: &Config) -> Self {
        let web_search_mode = config.web_search_mode.value();
        Self {
            // Core selects this executor per turn using the feature flag or model metadata.
            available: standalone_web_search_available(&config.model_provider, web_search_mode),
            provider_id: config.model_provider_id.clone(),
            provider: config.model_provider.clone(),
            settings: search_settings(config, web_search_mode),
        }
    }
}

fn standalone_web_search_available(
    provider: &ModelProviderInfo,
    web_search_mode: WebSearchMode,
) -> bool {
    provider.supports_standalone_web_search && web_search_mode != WebSearchMode::Disabled
}

fn search_settings(config: &Config, web_search_mode: WebSearchMode) -> SearchSettings {
    let web_search_config = config.web_search_config.as_ref();
    SearchSettings {
        user_location: web_search_config
            .and_then(|config| config.user_location.as_ref())
            .map(|location| ApproximateLocation {
                r#type: LocationType::Approximate,
                country: location.country.clone(),
                region: location.region.clone(),
                city: location.city.clone(),
                timezone: location.timezone.clone(),
            }),
        search_context_size: web_search_config
            .and_then(|config| config.search_context_size)
            .map(|size| match size {
                WebSearchContextSize::Low => SearchContextSize::Low,
                WebSearchContextSize::Medium => SearchContextSize::Medium,
                WebSearchContextSize::High => SearchContextSize::High,
            }),
        filters: web_search_config
            .and_then(|config| config.filters.as_ref())
            .map(|filters| SearchFilters {
                allowed_domains: filters.allowed_domains.clone(),
                blocked_domains: None,
            }),
        allowed_callers: Some(vec![AllowedCaller::Direct]),
        external_web_access: Some(external_web_access_for_mode(web_search_mode)),
        ..Default::default()
    }
}

fn external_web_access_for_mode(web_search_mode: WebSearchMode) -> ExternalWebAccess {
    match web_search_mode {
        WebSearchMode::Disabled | WebSearchMode::Cached => ExternalWebAccess::Boolean(false),
        WebSearchMode::Indexed => ExternalWebAccess::Mode(ExternalWebAccessMode::Indexed),
        WebSearchMode::Live => ExternalWebAccess::Boolean(true),
    }
}

impl ThreadLifecycleContributor<Config> for WebSearchExtension {
    fn on_thread_start<'a>(
        &'a self,
        input: ThreadStartInput<'a, Config>,
    ) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            input
                .thread_store
                .insert(WebSearchExtensionConfig::from(input.config));
        })
    }
}

impl ConfigContributor<Config> for WebSearchExtension {
    fn on_config_changed(
        &self,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
        _previous_config: &Config,
        new_config: &Config,
    ) {
        thread_store.insert(WebSearchExtensionConfig::from(new_config));
    }
}

impl ToolContributor for WebSearchExtension {
    fn tools(
        &self,
        session_store: &ExtensionData,
        thread_store: &ExtensionData,
    ) -> Vec<
        Arc<dyn for<'call> genex_extension_api::ToolExecutor<genex_extension_api::ToolCall<'call>>>,
    > {
        let Some(config) = thread_store.get::<WebSearchExtensionConfig>() else {
            return Vec::new();
        };
        if !config.available {
            return Vec::new();
        }

        vec![Arc::new(WebSearchTool {
            session_id: session_store.level_id().to_string(),
            provider: create_model_provider_for_id(
                config.provider_id.clone(),
                config.provider.clone(),
                Some(self.credential_manager.clone()),
            ),
            settings: config.settings.clone(),
            originator: thread_store
                .get::<ThreadOriginator>()
                .map(|originator| originator.0.clone()),
        })]
    }
}

pub fn install(
    registry: &mut ExtensionRegistryBuilder<Config>,
    credential_manager: Arc<CredentialManager>,
) {
    let extension = Arc::new(WebSearchExtension { credential_manager });
    registry.thread_lifecycle_contributor(extension.clone());
    registry.config_contributor(extension.clone());
    registry.tool_contributor(extension);
}

#[cfg(test)]
mod tests {
    use genex_extension_api::ExtensionData;
    use genex_extension_api::ExtensionRegistryBuilder;
    use genex_extension_api::ToolName;
    use genex_login::GenexAuth;
    use genex_model_provider_info::ModelProviderInfo;
    use pretty_assertions::assert_eq;

    use super::Config;
    use super::CredentialManager;
    use super::WebSearchExtensionConfig;
    use super::external_web_access_for_mode;
    use super::install;
    use super::standalone_web_search_available;
    use crate::tool::RUN_TOOL_NAME;
    use crate::tool::WEB_NAMESPACE;
    use genex_api::ExternalWebAccess;
    use genex_api::ExternalWebAccessMode;
    use genex_protocol::config_types::WebSearchMode;

    #[test]
    fn external_web_access_preserves_legacy_values_until_indexed() {
        assert_eq!(
            [
                WebSearchMode::Disabled,
                WebSearchMode::Cached,
                WebSearchMode::Indexed,
                WebSearchMode::Live,
            ]
            .map(external_web_access_for_mode),
            [
                ExternalWebAccess::Boolean(false),
                ExternalWebAccess::Boolean(false),
                ExternalWebAccess::Mode(ExternalWebAccessMode::Indexed),
                ExternalWebAccess::Boolean(true),
            ]
        );
    }

    #[test]
    fn installed_extension_contributes_web_run_when_enabled() {
        let mut builder = ExtensionRegistryBuilder::<Config>::new();
        install(
            &mut builder,
            CredentialManager::from_auth_for_testing(GenexAuth::from_api_key("dummy")),
        );
        let registry = builder.build();
        let session_store = ExtensionData::new("session");
        let thread_store = ExtensionData::new("11111111-1111-4111-8111-111111111111");
        thread_store.insert(WebSearchExtensionConfig {
            available: true,
            provider_id: "test".to_string(),
            provider: ModelProviderInfo {
                supports_standalone_web_search: true,
                ..ModelProviderInfo::default()
            },
            settings: Default::default(),
        });

        let tool_names = registry
            .tool_contributors()
            .iter()
            .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
            .map(|tool| (tool.tool_name(), tool.supports_parallel_tool_calls()))
            .collect::<Vec<_>>();

        assert_eq!(
            tool_names,
            vec![(ToolName::namespaced(WEB_NAMESPACE, RUN_TOOL_NAME), true)]
        );
    }

    #[test]
    fn standalone_web_search_requires_provider_opt_in() {
        assert!(!standalone_web_search_available(
            &ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            WebSearchMode::Live,
        ));
        assert!(standalone_web_search_available(
            &ModelProviderInfo {
                supports_standalone_web_search: true,
                ..ModelProviderInfo::default()
            },
            WebSearchMode::Live,
        ));
        assert!(!standalone_web_search_available(
            &ModelProviderInfo {
                supports_standalone_web_search: true,
                ..ModelProviderInfo::default()
            },
            WebSearchMode::Disabled,
        ));
    }
}
