use std::sync::Arc;

use codex_core::ThreadManager;
use codex_core_plugins::EffectivePluginsChange;
use codex_login::AuthManager;

use crate::config_manager::ConfigManager;
use crate::request_processors::ConfigRequestProcessor;
use crate::request_serialization::RequestSerializationAccess;
use crate::request_serialization::RequestSerializationQueueKey;
use crate::request_serialization::RequestSerializationQueues;

/// Refresh plugin consumers and trust hooks from newly materialized Workspace + Listed bundles.
pub(crate) fn effective_plugins_changed_callback(
    auth_manager: Arc<AuthManager>,
    thread_manager: Arc<ThreadManager>,
    config_manager: ConfigManager,
    config_processor: ConfigRequestProcessor,
    request_serialization_queues: RequestSerializationQueues,
) -> Arc<dyn Fn(EffectivePluginsChange) + Send + Sync> {
    Arc::new(move |change| {
        thread_manager.plugins_manager().clear_cache();
        thread_manager.skills_service().clear_cache();

        let refresh_thread_manager = Arc::clone(&thread_manager);
        tokio::spawn(async move {
            refresh_thread_manager.invalidate_mcp_runtimes().await;
            refresh_thread_manager.refresh_hook_runtimes().await;
        });
    })
}

#[cfg(test)]
#[path = "effective_plugin_change_tests.rs"]
mod tests;
