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
#[cfg(test)]
use genex_login::CredentialManager;
use genex_http_client::HttpClientFactory;
use genex_model_provider::SharedModelProvider;
use genex_model_provider::SharedProviderRuntime;
use genex_model_provider::provider_contract;
use genex_protocol::config_types::WebSearchContextSize;
use genex_protocol::config_types::WebSearchMode;

use crate::tool::WebSearchTool;

#[derive(Clone)]
struct WebSearchExtension {
    provider_contract: SharedModelProvider,
    http_client_factory: HttpClientFactory,
}

#[derive(Clone)]
struct WebSearchExtensionConfig {
    available: bool,
    settings: SearchSettings,
}

impl WebSearchExtensionConfig {
    fn from_config(config: &Config, provider: &SharedModelProvider) -> Self {
        let web_search_mode = config.web_search_mode.value();
        Self {
            // Core selects this executor per turn using the feature flag or model metadata.
            available: standalone_web_search_available(
                provider.capabilities().standalone_web_search,
                web_search_mode,
            ),
            settings: search_settings(config, web_search_mode),
        }
    }
}

fn standalone_web_search_available(
    provider_supports_web_search: bool,
    web_search_mode: WebSearchMode,
) -> bool {
    provider_supports_web_search && web_search_mode != WebSearchMode::Disabled
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
                .insert(WebSearchExtensionConfig::from_config(
                    input.config,
                    &self.provider_contract,
                ));
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
        thread_store.insert(WebSearchExtensionConfig::from_config(
            new_config,
            &self.provider_contract,
        ));
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
                    provider: self.provider_contract.clone(),
                    http_client_factory: self.http_client_factory.clone(),
            settings: config.settings.clone(),
            originator: thread_store
                .get::<ThreadOriginator>()
                .map(|originator| originator.0.clone()),
        })]
    }
}

pub fn install(registry: &mut ExtensionRegistryBuilder<Config>, provider: SharedProviderRuntime) {
    install_with_provider_contract(
        registry,
        provider_contract(provider),
        HttpClientFactory::new(genex_http_client::OutboundProxyPolicy::ReqwestDefault),
    );
}

pub fn install_with_provider_contract(
    registry: &mut ExtensionRegistryBuilder<Config>,
    provider_contract: SharedModelProvider,
    http_client_factory: HttpClientFactory,
) {
    let extension = Arc::new(WebSearchExtension {
        provider_contract,
        http_client_factory,
    });
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
    use genex_model_provider::create_model_provider;
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
    use std::sync::Arc;

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
        let credential_manager =
            CredentialManager::from_auth_for_testing(GenexAuth::from_api_key("dummy"));
        let provider = create_model_provider(
            ModelProviderInfo {
                supports_standalone_web_search: true,
                ..ModelProviderInfo::default()
            },
            Some(Arc::clone(&credential_manager)),
        );
        install(&mut builder, provider);
        let registry = builder.build();
        let session_store = ExtensionData::new("session");
        let thread_store = ExtensionData::new("11111111-1111-4111-8111-111111111111");
        thread_store.insert(WebSearchExtensionConfig {
            available: true,
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
        assert!(!standalone_web_search_available(false, WebSearchMode::Live,));
        assert!(standalone_web_search_available(true, WebSearchMode::Live,));
        assert!(!standalone_web_search_available(
            true,
            WebSearchMode::Disabled,
        ));
    }
}
