use genex_core_plugins::PluginsManager;
use genex_core_plugins::store::PluginStore;
use genex_login::CredentialManager;
use genex_login::GenexAuth;
use genex_plugin::PluginId;
use genex_protocol::protocol::Product;
use genex_skills_extension::HostSkillsLoadInput;
use genex_skills_extension::HostSkillsService;
use genex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::sync::Arc;

use super::test_support::load_plugins_config;
use super::test_support::write_file;

const PLUGIN_CONFIG_NAME: &str = "sample@test-remote";
const REMOTE_PLUGIN_ID: &str = "plugins~Plugin_sample";

#[tokio::test]
async fn host_skills_service_reuses_plugin_manager_skill_snapshot() {
    let genex_home = tempfile::tempdir().expect("create genex home");
    let plugin_root = genex_home
        .path()
        .join("plugins/cache/test-remote/sample/local");
    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample","description":"sample plugin"}"#,
    );
    let skill_path = plugin_root.join("skills/SKILL.md");
    write_file(&skill_path, "---\nname: search\ndescription: first\n---\n");
    write_file(
        &genex_home.path().join("config.toml"),
        &format!(
            r#"[features]
plugins = true
remote_plugin = true

[plugins."{PLUGIN_CONFIG_NAME}"]
enabled = true

[skills.bundled]
enabled = false
"#,
        ),
    );
    let plugin_id = PluginId::parse(PLUGIN_CONFIG_NAME).expect("remote plugin id should parse");
    PluginStore::new(genex_home.path().to_path_buf())
        .write_remote_plugin_id(&plugin_id, REMOTE_PLUGIN_ID)
        .expect("persist remote plugin id");
    let config = load_plugins_config(genex_home.path()).await;
    let plugins_input = config.plugins_config_input();
    let skills_service = Arc::new(HostSkillsService::new(
        config.genex_home.clone(),
        /*bundled_skills_enabled*/ false,
    ));
    let plugins_manager = PluginsManager::new_with_options(
        genex_home.path().to_path_buf(),
        Some(Product::Codex),
        CredentialManager::from_auth_for_testing(GenexAuth::from_api_key("test-api-key")),
        skills_service.clone(),
    );
    let plugin_outcome = plugins_manager.plugins_for_config(&plugins_input).await;

    write_file(&skill_path, "---\nname: search\ndescription: second\n---\n");

    let other_cwd = genex_home.path().join("other-workspace");
    std::fs::create_dir_all(&other_cwd).expect("create second workspace");
    let other_cwd = AbsolutePathBuf::from_absolute_path(other_cwd).expect("absolute workspace");

    for cwd in [config.cwd.clone(), other_cwd] {
        let skills_input = HostSkillsLoadInput::new(
            cwd,
            plugin_outcome.effective_plugin_skill_roots(),
            config.config_layer_stack.clone(),
        )
        .with_plugin_skill_snapshots(
            plugins_manager.plugin_skill_snapshots_for_config(&plugins_input),
        );
        let snapshot = skills_service
            .snapshot_for_config(&skills_input, /*fs*/ None)
            .await;

        assert_eq!(
            snapshot
                .outcome()
                .skills
                .iter()
                .filter(|skill| skill.plugin_id.as_deref() == Some(PLUGIN_CONFIG_NAME))
                .map(|skill| {
                    (
                        skill.description.as_str(),
                        skill.plugin_id.as_deref(),
                        skill.remote_plugin_id.as_deref(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![("first", Some(PLUGIN_CONFIG_NAME), Some(REMOTE_PLUGIN_ID),)]
        );
    }
}
