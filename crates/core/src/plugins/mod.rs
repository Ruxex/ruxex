mod injection;
mod mentions;
pub(crate) mod metrics;
mod render;
#[cfg(test)]
pub(crate) mod test_support;

use crate::config::Config;
use genex_core_plugins::PluginsManager;
use genex_credential_broker::SharedCredentialBroker;
use genex_skills_extension::HostSkillsService;
use std::sync::Arc;

pub(crate) use genex_plugin::PluginCapabilitySummary;

pub(crate) use injection::build_plugin_injections;
pub(crate) use render::render_explicit_plugin_instructions;

pub(crate) use mentions::collect_explicit_plugin_ids;
pub(crate) use mentions::collect_explicit_plugin_mentions;
pub(crate) use mentions::collect_tool_mentions_from_messages;

/// Constructs a standalone plugin manager with extension-owned plugin skill loading.
///
/// Callers that already own a host skills service should inject that existing service instead.
pub fn plugins_manager_for_config(
    config: &Config,
    credential_broker: SharedCredentialBroker,
) -> PluginsManager {
    let skill_root_loader = Arc::new(HostSkillsService::new(
        config.genex_home.clone(),
        /*bundled_skills_enabled*/ false,
    ));
    PluginsManager::new(
        config.genex_home.to_path_buf(),
        credential_broker,
        skill_root_loader,
    )
}
