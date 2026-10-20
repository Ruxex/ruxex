//! Executor-discovered plugin hook admission and trusted MCP routing.

use ruxex_exec_server::ExecutorCapabilityDiscoverySnapshot;
use ruxex_mcp::ToolInfo;
use ruxex_plugin::ExecutorPluginHookSource;
use ruxex_plugin::PluginId;
use ruxex_plugin::is_allowlisted_bundled_cleanup_hook;
use ruxex_plugin::manifest::PluginManifestHooks;
use ruxex_protocol::capabilities::CapabilityRootLocation;

use crate::manifest::parse_plugin_manifest_uri;

/// Returns accepted inline hook sources from executor-discovered plugin manifests.
/// Each source carries its trusted MCP routing metadata. `lookup_enabled_tool` must use a
/// consistent catalog/config snapshot and exclude tools disabled by policy.
///
/// Executor scoped hooks are best-effort because executor capabilities can become available
/// after earlier lifecycle events have passed.
///
/// Note: Executor manifests are not signed yet, so temporarily we only admit the known cleanup
/// hooks from the bundled cleanup allowlist and the remote Browser plugin.
pub fn executor_plugin_hook_sources<'a>(
    snapshot: &ExecutorCapabilityDiscoverySnapshot,
    _lookup_enabled_tool: impl Fn(&str, &str) -> Option<&'a ToolInfo>,
) -> Vec<ExecutorPluginHookSource> {
    let mut sources = Vec::new();

    for entry in snapshot.roots() {
        let Ok(plugin_id) = PluginId::parse(&entry.selected_root.id) else {
            continue;
        };
        let CapabilityRootLocation::Environment {
            environment_id,
            path: plugin_root,
        } = &entry.selected_root.location;
        let Ok(discovery) = &entry.result else {
            continue;
        };
        let Some(plugin) = &discovery.plugin else {
            continue;
        };
        let Ok(manifest) = parse_plugin_manifest_uri(
            plugin_root,
            &plugin.manifest.path,
            &plugin.manifest.contents,
        ) else {
            continue;
        };
        // Only inline hooks are supported for now, so skip any other source types.
        let Some(PluginManifestHooks::Inline(hook_files)) = manifest.paths.hooks else {
            continue;
        };

        for (hook_index, hook_file) in hook_files.into_iter().enumerate() {
            let manifest_relative_path = plugin
                .manifest
                .path
                .relative_path_from(plugin_root)
                .unwrap_or_else(|| plugin.manifest.path.to_string());

            sources.push(ExecutorPluginHookSource {
                plugin_id: plugin_id.clone(),
                environment_id: environment_id.clone(),
                mcp_environment_id: None,
                mcp_metadata: None,
                plugin_root: plugin_root.clone(),
                manifest_path: plugin.manifest.path.clone(),
                source_relative_path: format!("{manifest_relative_path}#hooks[{hook_index}]"),
                hooks: hook_file.hooks,
            });
        }
    }

    // FIXME: Remove this temporary filter once executor plugin hooks can be trusted.
    sources
        .into_iter()
        .filter_map(|mut source| {
            let plugin_id = source.plugin_id.as_key();
            for (event, groups) in source.hooks.matcher_groups_mut() {
                groups.retain_mut(|group| {
                    group.hooks.retain(|handler| {
                        is_allowlisted_bundled_cleanup_hook(
                            &plugin_id,
                            event,
                            group.matcher.as_deref(),
                            handler,
                        )
                    });
                    !group.hooks.is_empty()
                });
            }
            (!source.hooks.is_empty()).then_some(source)
        })
        .collect()
}

#[cfg(test)]
#[path = "executor_hooks_tests.rs"]
mod tests;
