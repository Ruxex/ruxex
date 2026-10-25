use super::*;
use ruxex_app_server_protocol::PluginAvailability;
use ruxex_app_server_protocol::PluginDetail;
use ruxex_app_server_protocol::PluginInstallParams;
use ruxex_app_server_protocol::PluginInstallResponse;
use ruxex_app_server_protocol::PluginInstalledParams;
use ruxex_app_server_protocol::PluginInstalledResponse;
use ruxex_app_server_protocol::PluginListParams;
use ruxex_app_server_protocol::PluginListResponse;
use ruxex_app_server_protocol::PluginMarketplaceEntry;
use ruxex_app_server_protocol::PluginReadParams;
use ruxex_app_server_protocol::PluginReadResponse;
use ruxex_app_server_protocol::PluginSource;
use ruxex_app_server_protocol::PluginSummary;
use ruxex_app_server_protocol::PluginUninstallParams;
use ruxex_app_server_protocol::PluginUninstallResponse;
use ruxex_app_server_protocol::SkillSummary;
use ruxex_core::ThreadManager;
use ruxex_core::config::Config;
use ruxex_core_plugins::ConfiguredMarketplace;
use ruxex_core_plugins::ConfiguredMarketplacePlugin;
use ruxex_core_plugins::PluginInstallRequest;
use ruxex_core_plugins::PluginMarketplaceContext;
use ruxex_core_plugins::PluginReadRequest;
use ruxex_core_plugins::marketplace::MarketplacePluginSource;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use std::collections::HashSet;
use std::sync::Arc;

mod local;
mod reconcile;
mod search;

fn plugin_skills_to_api(
    skills: &[ruxex_skills::SkillMetadata],
    disabled_skill_paths: &HashSet<AbsolutePathBuf>,
) -> Vec<SkillSummary> {
    skills
        .iter()
        .map(|skill| SkillSummary {
            name: skill.name.clone(),
            description: skill.description.clone(),
            short_description: skill.short_description.clone(),
            interface: skill.interface.clone().map(|interface| {
                ruxex_app_server_protocol::SkillInterface {
                    display_name: interface.display_name,
                    short_description: interface.short_description,
                    icon_small: interface.icon_small,
                    icon_large: interface.icon_large,
                    icon_small_url: None,
                    icon_large_url: None,
                    brand_color: interface.brand_color,
                    default_prompt: interface.default_prompt,
                }
            }),
            path: Some(skill.path_to_skills_md.clone()),
            enabled: !disabled_skill_paths.contains(&skill.path_to_skills_md),
        })
        .collect()
}

#[derive(Clone)]
pub(crate) struct PluginRequestProcessor {
    thread_manager: Arc<ThreadManager>,
    config_manager: ConfigManager,
    on_effective_plugins_changed:
        Arc<dyn Fn(ruxex_core_plugins::EffectivePluginsChange) + Send + Sync>,
}

impl PluginRequestProcessor {
    async fn load_latest_config(
        &self,
        fallback_cwd: Option<std::path::PathBuf>,
    ) -> Result<Config, JSONRPCErrorError> {
        self.config_manager
            .load_latest_config(fallback_cwd)
            .await
            .map_err(|err| internal_error(format!("failed to reload config: {err}")))
    }

    pub(crate) fn new(
        thread_manager: Arc<ThreadManager>,
        config_manager: ConfigManager,
        on_effective_plugins_changed: Arc<
            dyn Fn(ruxex_core_plugins::EffectivePluginsChange) + Send + Sync,
        >,
    ) -> Self {
        Self {
            thread_manager,
            config_manager,
            on_effective_plugins_changed,
        }
    }

    pub(crate) fn effective_plugins_changed_callback(
        &self,
    ) -> Arc<dyn Fn(ruxex_core_plugins::EffectivePluginsChange) + Send + Sync> {
        Arc::clone(&self.on_effective_plugins_changed)
    }

    pub(crate) async fn plugin_list(
        &self,
        params: PluginListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let roots = params.cwds.unwrap_or_default();
        let config = self.load_catalog_config(&roots).await?;
        let context = self.load_marketplace_context(roots, &config).await;
        let (marketplaces, errors) = self.local_marketplaces(&context)?;
        let _ = params.marketplace_kinds;
        Ok(Some(
            PluginListResponse {
                marketplaces,
                marketplace_load_errors: errors,
            }
            .into(),
        ))
    }

    pub(crate) async fn plugin_installed(
        &self,
        params: PluginInstalledParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let roots = params.cwds.unwrap_or_default();
        let config = self.load_catalog_config(&roots).await?;
        let context = self.load_marketplace_context(roots, &config).await;
        let (mut marketplaces, errors) = self.local_marketplaces(&context)?;
        for marketplace in &mut marketplaces {
            marketplace.plugins.retain(|plugin| plugin.installed);
        }
        Ok(Some(
            PluginInstalledResponse {
                marketplaces,
                marketplace_load_errors: errors,
            }
            .into(),
        ))
    }

    pub(crate) async fn plugin_read(
        &self,
        params: PluginReadParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let marketplace_path = params.marketplace_path.ok_or_else(|| {
            invalid_request(
                "plugin/read requires marketplacePath; remote marketplaces are not supported",
            )
        })?;
        let config = self
            .load_latest_config(marketplace_path.as_path().parent().map(Path::to_path_buf))
            .await?;
        let outcome = self
            .thread_manager
            .plugins_manager()
            .read_plugin_for_config(
                &config.plugins_config_input(),
                &PluginReadRequest {
                    plugin_name: params.plugin_name,
                    marketplace_path,
                },
            )
            .await
            .map_err(|err| internal_error(format!("failed to read plugin details: {err}")))?;
        Ok(Some(
            PluginReadResponse {
                plugin: plugin_detail_to_api(outcome),
            }
            .into(),
        ))
    }

    pub(crate) async fn plugin_install(
        &self,
        params: PluginInstallParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let marketplace_path = params.marketplace_path.ok_or_else(|| {
            invalid_request(
                "plugin/install requires marketplacePath; remote marketplaces are not supported",
            )
        })?;
        let config = self.load_latest_config(None).await?;
        let outcome = self
            .thread_manager
            .plugins_manager()
            .install_plugin(
                &config.config_layer_stack,
                PluginInstallRequest {
                    plugin_name: params.plugin_name,
                    marketplace_path,
                },
            )
            .await
            .map_err(|err| internal_error(format!("failed to install plugin: {err}")))?;
        (self.on_effective_plugins_changed)(ruxex_core_plugins::EffectivePluginsChange::default());
        Ok(Some(
            PluginInstallResponse {
                auth_policy: outcome.auth_policy.into(),
            }
            .into(),
        ))
    }

    pub(crate) async fn plugin_uninstall(
        &self,
        params: PluginUninstallParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.thread_manager
            .plugins_manager()
            .uninstall_plugin(params.plugin_id)
            .await
            .map_err(|err| internal_error(format!("failed to uninstall plugin: {err}")))?;
        (self.on_effective_plugins_changed)(ruxex_core_plugins::EffectivePluginsChange::default());
        Ok(Some(PluginUninstallResponse {}.into()))
    }

    fn local_marketplaces(
        &self,
        context: &PluginMarketplaceContext,
    ) -> Result<
        (
            Vec<PluginMarketplaceEntry>,
            Vec<ruxex_app_server_protocol::MarketplaceLoadErrorInfo>,
        ),
        JSONRPCErrorError,
    > {
        let outcome = self
            .thread_manager
            .plugins_manager()
            .list_marketplaces_for_context(context)
            .map_err(|err| internal_error(format!("failed to list marketplace plugins: {err}")))?;
        Ok((
            outcome
                .marketplaces
                .into_iter()
                .map(marketplace_to_api)
                .collect(),
            outcome
                .errors
                .into_iter()
                .map(
                    |error| ruxex_app_server_protocol::MarketplaceLoadErrorInfo {
                        marketplace_path: error.path,
                        message: error.message,
                    },
                )
                .collect(),
        ))
    }
}

fn marketplace_to_api(marketplace: ConfiguredMarketplace) -> PluginMarketplaceEntry {
    PluginMarketplaceEntry {
        name: marketplace.name,
        path: Some(marketplace.path),
        interface: marketplace.interface.map(|interface| {
            ruxex_app_server_protocol::MarketplaceInterface {
                display_name: interface.display_name,
            }
        }),
        plugins: marketplace
            .plugins
            .into_iter()
            .map(plugin_to_summary)
            .collect(),
    }
}

fn plugin_to_summary(plugin: ConfiguredMarketplacePlugin) -> PluginSummary {
    PluginSummary {
        id: plugin.id,
        local_version: plugin.local_version,
        name: plugin.name,
        source: marketplace_plugin_source_to_api(plugin.source),
        installed: plugin.installed,
        enabled: plugin.enabled,
        install_policy: plugin.policy.installation.into(),
        install_policy_source: None,
        must_show_installation_interstitial: None,
        auth_policy: plugin.policy.authentication.into(),
        availability: PluginAvailability::Available,
        interface: plugin.interface.map(|interface| PluginInterface {
            display_name: interface.display_name,
            short_description: interface.short_description,
            long_description: interface.long_description,
            developer_name: interface.developer_name,
            category: interface.category,
            capabilities: interface.capabilities,
            website_url: interface.website_url,
            privacy_policy_url: interface.privacy_policy_url,
            terms_of_service_url: interface.terms_of_service_url,
            default_prompt: interface.default_prompt,
            brand_color: interface.brand_color,
            composer_icon: interface.composer_icon,
            composer_icon_url: None,
            logo: interface.logo,
            logo_dark: interface.logo_dark,
            logo_url: None,
            logo_url_dark: None,
            screenshots: interface.screenshots,
            screenshot_urls: Vec::new(),
        }),
        keywords: plugin.keywords,
    }
}

fn plugin_detail_to_api(outcome: ruxex_core_plugins::PluginReadOutcome) -> PluginDetail {
    let skills = plugin_skills_to_api(&outcome.plugin.skills, &outcome.plugin.disabled_skill_paths);
    let onboarding_skill = if outcome.plugin.enabled {
        outcome.plugin.onboarding_skill.as_ref().and_then(|path| {
            skills
                .iter()
                .find(|skill| skill.enabled && skill.path.as_ref() == Some(path))
                .cloned()
        })
    } else {
        None
    };
    let summary = PluginSummary {
        id: outcome.plugin.id.clone(),
        local_version: outcome.plugin.local_version.clone(),
        name: outcome.plugin.name.clone(),
        source: marketplace_plugin_source_to_api(outcome.plugin.source),
        installed: outcome.plugin.installed,
        enabled: outcome.plugin.enabled,
        install_policy: outcome.plugin.policy.installation.into(),
        install_policy_source: None,
        must_show_installation_interstitial: None,
        auth_policy: outcome.plugin.policy.authentication.into(),
        availability: PluginAvailability::Available,
        interface: outcome.plugin.interface.map(|interface| PluginInterface {
            display_name: interface.display_name,
            short_description: interface.short_description,
            long_description: interface.long_description,
            developer_name: interface.developer_name,
            category: interface.category,
            capabilities: interface.capabilities,
            website_url: interface.website_url,
            privacy_policy_url: interface.privacy_policy_url,
            terms_of_service_url: interface.terms_of_service_url,
            default_prompt: interface.default_prompt,
            brand_color: interface.brand_color,
            composer_icon: interface.composer_icon,
            composer_icon_url: None,
            logo: interface.logo,
            logo_dark: interface.logo_dark,
            logo_url: None,
            logo_url_dark: None,
            screenshots: interface.screenshots,
            screenshot_urls: Vec::new(),
        }),
        keywords: outcome.plugin.keywords,
    };
    PluginDetail {
        marketplace_name: outcome.marketplace_name,
        marketplace_path: outcome.marketplace_path,
        summary,
        description: outcome.plugin.description,
        skills,
        onboarding_skill,
        hooks: Vec::new(),
        mcp_servers: outcome.plugin.mcp_server_names,
        scheduled_tasks: None,
    }
}

fn marketplace_plugin_source_to_api(source: MarketplacePluginSource) -> PluginSource {
    match source {
        MarketplacePluginSource::Local { path } => PluginSource::Local { path },
        MarketplacePluginSource::Git {
            url,
            path,
            ref_name,
            sha,
        } => PluginSource::Git {
            url,
            path,
            ref_name,
            sha,
        },
        MarketplacePluginSource::Npm {
            package,
            version,
            registry,
        } => PluginSource::Npm {
            package,
            version,
            registry,
        },
    }
}
