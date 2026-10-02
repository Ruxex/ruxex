use crate::config::Config;
use genex_config::types::ToolSuggestDiscoverableType;
use genex_core_plugins::PluginsManager;
use genex_core_plugins::ToolSuggestPluginDiscoveryInput;
use genex_login::GenexAuth;
use genex_tools::DiscoverablePluginInfo;
use std::collections::HashSet;
use tracing::instrument;

#[instrument(level = "trace", skip_all)]
pub(crate) async fn list_tool_suggest_discoverable_plugins(
    config: &Config,
    plugins_manager: &PluginsManager,
    auth: Option<&GenexAuth>,
) -> anyhow::Result<Vec<DiscoverablePluginInfo>> {
    let input = ToolSuggestPluginDiscoveryInput {
        plugins: config.plugins_config_input(),
        configured_plugin_ids: config
            .tool_suggest
            .discoverables
            .iter()
            .filter(|discoverable| discoverable.kind == ToolSuggestDiscoverableType::Plugin)
            .map(|discoverable| discoverable.id.clone())
            .collect::<HashSet<_>>(),
        disabled_plugin_ids: config
            .tool_suggest
            .disabled_tools
            .iter()
            .filter(|disabled_tool| disabled_tool.kind == ToolSuggestDiscoverableType::Plugin)
            .map(|disabled_tool| disabled_tool.id.clone())
            .collect::<HashSet<_>>(),
    };
    plugins_manager
        .list_tool_suggest_discoverable_plugins(&input, auth)
        .await
        .map(|plugins| {
            plugins
                .into_iter()
                .map(|plugin| DiscoverablePluginInfo {
                    id: plugin.id,
                    remote_plugin_id: plugin.remote_plugin_id,
                    name: plugin.name,
                    description: plugin.description,
                    has_skills: plugin.has_skills,
                    mcp_server_names: plugin.mcp_server_names,
                })
                .collect()
        })
}
