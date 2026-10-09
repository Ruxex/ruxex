use anyhow::Context;
use genex_config::skill_config_rules_from_stack;
use std::collections::HashSet;
use tracing::warn;

use crate::PluginsConfigInput;
use crate::PluginsManager;
use crate::marketplace::MarketplacePluginInstallPolicy;

#[derive(Debug, Clone)]
pub struct ToolSuggestPluginDiscoveryInput {
    pub plugins: PluginsConfigInput,
    pub configured_plugin_ids: HashSet<String>,
    pub disabled_plugin_ids: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolSuggestDiscoverablePlugin {
    pub id: String,
    pub remote_plugin_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub has_skills: bool,
    pub mcp_server_names: Vec<String>,
}

impl PluginsManager {
    pub async fn list_tool_suggest_discoverable_plugins(
        &self,
        input: &ToolSuggestPluginDiscoveryInput,
    ) -> anyhow::Result<Vec<ToolSuggestDiscoverablePlugin>> {
        if !input.plugins.plugins_enabled {
            return Ok(Vec::new());
        }

        let marketplaces = self
            .list_marketplaces_for_config(&input.plugins, &[])
            .context("failed to list plugin marketplaces for tool suggestions")?
            .marketplaces;
        let skill_config_rules = skill_config_rules_from_stack(&input.plugins.config_layer_stack);

        let mut discoverable_plugins = Vec::<ToolSuggestDiscoverablePlugin>::new();
        for marketplace in marketplaces {
            let marketplace_name = marketplace.name;

            for plugin in marketplace.plugins {
                let is_configured_plugin = input.configured_plugin_ids.contains(plugin.id.as_str());
                if plugin.installed
                    || plugin.policy.installation == MarketplacePluginInstallPolicy::NotAvailable
                    || input.disabled_plugin_ids.contains(plugin.id.as_str())
                    || !is_configured_plugin
                {
                    continue;
                }

                let plugin_id = plugin.id.clone();
                match self
                    .tool_suggest_metadata_for_marketplace_plugin(
                        &marketplace_name,
                        &plugin,
                        &skill_config_rules,
                    )
                    .await
                {
                    Ok(plugin) => {
                        discoverable_plugins.push(ToolSuggestDiscoverablePlugin {
                            id: plugin.config_name,
                            remote_plugin_id: None,
                            name: plugin.display_name,
                            description: plugin.description,
                            has_skills: plugin.has_skills,
                            mcp_server_names: plugin.mcp_server_names,
                        });
                    }
                    Err(err) => {
                        warn!("failed to load discoverable plugin suggestion {plugin_id}: {err:#}")
                    }
                }
            }
        }
        discoverable_plugins.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(discoverable_plugins)
    }
}
