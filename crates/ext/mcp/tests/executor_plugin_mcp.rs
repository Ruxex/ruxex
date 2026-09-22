use codex_core::config::Config;
use codex_core::config::ConfigBuilder;
use codex_exec_server::EnvironmentManager;
use codex_exec_server::ExecutorCapabilityDiscoveryCache;
use codex_exec_server::LOCAL_ENVIRONMENT_ID;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionDataInit;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::McpServerContribution;
use codex_extension_api::McpServerContributionContext;
use codex_features::Feature;
use codex_protocol::capabilities::CapabilityRootLocation;
use codex_protocol::capabilities::SelectedCapabilityRoot;
use codex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, PartialEq, Eq)]
struct PackageSummary {
    plugin_id: String,
    plugin_display_name: String,
}

#[tokio::test]
async fn selected_plugin_package_is_contributed_without_servers() -> TestResult {
    let codex_home = tempfile::tempdir()?;
    let plugin_root = tempfile::tempdir()?;
    std::fs::create_dir_all(plugin_root.path().join(".codex-plugin"))?;
    std::fs::create_dir_all(plugin_root.path().join("skills/deploy"))?;
    std::fs::write(
        plugin_root.path().join(".codex-plugin/plugin.json"),
        r#"{"name":"skill-only","interface":{"displayName":"Skill Only"}}"#,
    )?;
    std::fs::write(
        plugin_root.path().join("skills/deploy/SKILL.md"),
        "---\nname: deploy\ndescription: Deploy the project.\n---\n",
    )?;
    let config = ConfigBuilder::default()
        .codex_home(codex_home.path().to_path_buf())
        .fallback_cwd(Some(codex_home.path().to_path_buf()))
        .build()
        .await?;

    let contributions = raw_selected_plugin_contributions(&config, plugin_root.path()).await?;
    let package = contributions.into_iter().find_map(|contribution| {
        let McpServerContribution::SelectedPluginPackage {
            plugin_id,
            plugin_display_name,
            ..
        } = contribution
        else {
            return None;
        };
        Some(PackageSummary {
            plugin_id,
            plugin_display_name,
        })
    });

    assert_eq!(
        package,
        Some(PackageSummary {
            plugin_id: "selected-root".to_string(),
            plugin_display_name: "Skill Only".to_string(),
        })
    );
    Ok(())
}

async fn raw_selected_plugin_contributions(
    config: &Config,
    plugin_root: &std::path::Path,
) -> Result<Vec<McpServerContribution>, Box<dyn std::error::Error>> {
    let mut builder = ExtensionRegistryBuilder::new();
    let environment_manager = Arc::new(EnvironmentManager::default_for_tests());
    codex_mcp_extension::install_executor_plugins(&mut builder, Arc::clone(&environment_manager));
    let registry = builder.build();
    let thread_init = ExtensionDataInit::new();
    let selected_capability_roots = vec![SelectedCapabilityRoot {
        id: "selected-root".to_string(),
        location: CapabilityRootLocation::Environment {
            environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
            path: PathUri::from_host_native_path(plugin_root)?,
        },
    }];
    let thread_store = ExtensionData::new_with_init("test-thread", thread_init.clone());
    let executor_capability_discovery = if config
        .features
        .enabled(Feature::ExecutorCapabilityDiscovery)
    {
        Some(
            ExecutorCapabilityDiscoveryCache::new(environment_manager)
                .snapshot(&selected_capability_roots, &Default::default())
                .await,
        )
    } else {
        None
    };

    Ok(registry.mcp_server_contributors()[0]
        .contribute(McpServerContributionContext::for_step(
            config,
            &thread_init,
            &thread_store,
            "test_originator",
            &selected_capability_roots,
            executor_capability_discovery.as_ref(),
        ))
        .await)
}
