use super::*;
use crate::LoadedPlugin;
use crate::PluginLoadOutcome;
use crate::TEST_API_CURATED_MARKETPLACE_NAME;
use crate::ToolSuggestDiscoverablePlugin;
use crate::ToolSuggestPluginDiscoveryInput;
use crate::installed_marketplaces::marketplace_install_root;
use crate::loader::load_plugin_skill_inventory;
use crate::loader::refresh_non_curated_plugin_cache;
use crate::loader::refresh_non_curated_plugin_cache_force_reinstall;
use crate::marketplace::MarketplacePluginInstallPolicy;
use crate::test_support::curated_plugins_repo_path;
use crate::test_support::load_plugins_config as load_plugins_config_input;
use crate::test_support::test_plugins_manager;
use crate::test_support::test_plugins_manager_with_options;
use crate::test_support::test_skill_root_loader;
use crate::test_support::write_curated_plugin;
use crate::test_support::write_file;
use genex_config::AppToolApproval;
use genex_config::CONFIG_TOML_FILE;
use genex_config::ConfigLayerEntry;
use genex_config::ConfigLayerSource;
use genex_config::ConfigLayerStack;
use genex_config::ConfigRequirements;
use genex_config::ConfigRequirementsToml;
use genex_config::McpServerConfig;
use genex_config::McpServerToolConfig;
use genex_config::RequirementSource;
use genex_config::RequirementsLayerEntry;
use genex_config::SkillConfigRules;
use genex_config::compose_requirements;
use genex_config::types::McpServerTransportConfig;
use genex_plugin::PluginId;
use genex_protocol::auth::AuthMode;
use genex_protocol::protocol::Product;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_absolute_path::test_support::PathBufExt;
use genex_utils_plugins::SkillDiscoveryMode;
use pretty_assertions::assert_eq;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use toml::Value;

const MAX_CAPABILITY_SUMMARY_DESCRIPTION_LEN: usize = 1024;

fn unrestricted_config_layer_stack() -> ConfigLayerStack {
    ConfigLayerStack::default()
}

fn config_layer_stack_with_requirements(
    genex_home: &Path,
    user_config: &str,
    requirements: &str,
) -> ConfigLayerStack {
    let with_sources = compose_requirements([RequirementsLayerEntry::from_toml(
        RequirementSource::Unknown,
        requirements,
    )])
    .expect("compose requirements")
    .expect("requirements should be present");
    let requirements_toml = with_sources.clone().into_toml();
    let requirements = ConfigRequirements::try_from(with_sources).expect("normalize requirements");
    let config_file =
        AbsolutePathBuf::try_from(genex_home.join(CONFIG_TOML_FILE)).expect("absolute config path");
    ConfigLayerStack::new(
        vec![ConfigLayerEntry::new(
            ConfigLayerSource::User {
                file: config_file,
                profile: None,
            },
            toml::from_str(user_config).expect("parse user config"),
        )],
        requirements,
        requirements_toml,
    )
    .expect("build config layer stack")
}

fn plugins_config_input_with_requirements(
    genex_home: &Path,
    user_config: &str,
    requirements: &str,
) -> PluginsConfigInput {
    PluginsConfigInput::new(
        config_layer_stack_with_requirements(genex_home, user_config, requirements),
        /*plugins_enabled*/ true,
    )
}

#[test]
fn marketplace_source_refresh_notifies_only_after_installed_cache_changes() {
    let tmp = TempDir::new().unwrap();
    let manager = test_plugins_manager(tmp.path().to_path_buf());
    let callback_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let callback_count_for_callback = Arc::clone(&callback_count);
    let callback: EffectivePluginsChangedCallback = Arc::new(move |_change| {
        callback_count_for_callback.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });

    manager.clear_caches_after_marketplace_source_refresh(
        /*installed_plugin_cache_refreshed*/ false,
        Some(&callback),
    );
    assert_eq!(callback_count.load(std::sync::atomic::Ordering::Relaxed), 0);

    manager.clear_caches_after_marketplace_source_refresh(
        /*installed_plugin_cache_refreshed*/ true,
        Some(&callback),
    );
    assert_eq!(callback_count.load(std::sync::atomic::Ordering::Relaxed), 1);
}

#[tokio::test]
async fn marketplace_policy_projection_disables_installed_plugin_and_invalidates_cache() {
    let genex_home = TempDir::new().expect("create Genex home");
    write_plugin(
        &genex_home.path().join("plugins/cache/company"),
        "sample/local",
        "sample",
    );
    let user_config = r#"
[marketplaces.company]
source_type = "git"
source = "https://github.com/example/company.git"

[plugins."sample@company"]
enabled = true
"#;
    let allowed = plugins_config_input_with_requirements(
        genex_home.path(),
        user_config,
        r#"
[marketplaces]
restrict_to_allowed_sources = true

[marketplaces.allowed_sources.company]
source = "git"
url = "https://github.com/example/company.git"
"#,
    );
    let blocked = plugins_config_input_with_requirements(
        genex_home.path(),
        user_config,
        r#"
[marketplaces]
restrict_to_allowed_sources = true

[marketplaces.allowed_sources.other]
source = "git"
url = "https://github.com/example/other.git"
"#,
    );
    let manager = test_plugins_manager(genex_home.path().to_path_buf());

    let allowed_outcome = manager.plugins_for_config(&allowed).await;
    assert_eq!(allowed_outcome.plugins().len(), 1);
    assert_eq!(allowed_outcome.plugins()[0].config_name, "sample@company");

    let blocked_outcome = manager.plugins_for_config(&blocked).await;
    assert_eq!(blocked_outcome, PluginLoadOutcome::default());
}

#[tokio::test]
async fn plugin_read_rejects_marketplace_blocked_by_requirements() {
    let genex_home = TempDir::new().expect("create Genex home");
    let marketplace_root = genex_home.path().join("marketplace");
    write_plugin(&marketplace_root, "sample", "sample");
    write_file(
        &marketplace_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "company",
  "plugins": [
    {
      "name": "sample",
      "source": {"source": "local", "path": "./sample"}
    }
  ]
}"#,
    );
    let config = plugins_config_input_with_requirements(
        genex_home.path(),
        "",
        r#"
[marketplaces]
restrict_to_allowed_sources = true
"#,
    );
    let marketplace_path =
        AbsolutePathBuf::try_from(marketplace_root.join(".agents/plugins/marketplace.json"))
            .expect("absolute marketplace path");

    let err = test_plugins_manager(genex_home.path().to_path_buf())
        .read_plugin_for_config(
            &config,
            &PluginReadRequest {
                plugin_name: "sample".to_string(),
                marketplace_path,
            },
        )
        .await
        .expect_err("blocked marketplace should not be readable");
    assert!(matches!(
        err,
        MarketplaceError::InvalidMarketplaceFile { .. }
    ));
}

#[test]
fn marketplace_policy_filters_discovered_marketplaces_by_configured_name() {
    let genex_home = TempDir::new().expect("create Genex home");
    let repo_root = genex_home.path().join("repo");
    let subdirectory = repo_root.join("worktree/subdirectory");
    fs::create_dir_all(&subdirectory).expect("create input subdirectory");
    write_plugin(&repo_root, "sample", "sample");
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "company",
  "plugins": [
    {
      "name": "sample",
      "source": {"source": "local", "path": "./sample"}
    }
  ]
}"#,
    );
    init_git_repo(&repo_root);
    let repo_root = AbsolutePathBuf::try_from(repo_root).expect("absolute repository root");
    let subdirectory =
        AbsolutePathBuf::try_from(subdirectory).expect("absolute input subdirectory");
    let manager = test_plugins_manager(genex_home.path().to_path_buf());
    let user_config = format!(
        r#"
[marketplaces.company]
source_type = "local"
source = {:?}
"#,
        repo_root.as_path()
    );
    let allowed = plugins_config_input_with_requirements(
        genex_home.path(),
        &user_config,
        &format!(
            r#"
[marketplaces]
restrict_to_allowed_sources = true

[marketplaces.allowed_sources.company]
source = "local"
path = {:?}
"#,
            repo_root.as_path()
        ),
    );
    let blocked = plugins_config_input_with_requirements(
        genex_home.path(),
        &user_config,
        &format!(
            r#"
[marketplaces]
restrict_to_allowed_sources = true

[marketplaces.allowed_sources.subdirectory]
source = "local"
path = {:?}
"#,
            subdirectory.as_path()
        ),
    );

    let allowed_outcome = manager
        .list_marketplaces_for_config(&allowed, std::slice::from_ref(&subdirectory))
        .expect("list allowed marketplace");
    assert_eq!(allowed_outcome.marketplaces.len(), 1);
    assert_eq!(allowed_outcome.marketplaces[0].name, "company");

    let blocked_outcome = manager
        .list_marketplaces_for_config(&blocked, std::slice::from_ref(&subdirectory))
        .expect("list blocked marketplace");
    assert_eq!(blocked_outcome.marketplaces, Vec::new());
}

fn write_plugin_with_version(
    root: &Path,
    dir_name: &str,
    manifest_name: &str,
    manifest_version: Option<&str>,
) {
    let plugin_root = root.join(dir_name);
    fs::create_dir_all(plugin_root.join(".genex-plugin")).unwrap();
    fs::create_dir_all(plugin_root.join("skills")).unwrap();
    let version = manifest_version
        .map(|manifest_version| format!(r#","version":"{manifest_version}""#))
        .unwrap_or_default();
    fs::write(
        plugin_root.join(".genex-plugin/plugin.json"),
        format!(r#"{{"name":"{manifest_name}"{version}}}"#),
    )
    .unwrap();
    fs::write(
        plugin_root.join("skills/SKILL.md"),
        format!("---\nname: {manifest_name}-skill\ndescription: test skill\n---\n\n# Test skill\n"),
    )
    .unwrap();
    fs::write(plugin_root.join(".mcp.json"), r#"{"mcpServers":{}}"#).unwrap();
}

fn write_plugin(root: &Path, dir_name: &str, manifest_name: &str) {
    write_plugin_with_version(
        root,
        dir_name,
        manifest_name,
        /*manifest_version*/ None,
    );
}

fn init_git_repo(repo: &Path) {
    run_git(repo, &["init"]);
    run_git(repo, &["config", "user.email", "genex-test@example.com"]);
    run_git(repo, &["config", "user.name", "Genex Test"]);
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "initial"]);
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("git should run: {err}"));
    assert!(
        output.status.success(),
        "git -C {} {} failed\nstdout:\n{}\nstderr:\n{}",
        repo.display(),
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn plugin_config_toml(enabled: bool, plugins_feature_enabled: bool) -> String {
    let mut root = toml::map::Map::new();

    let mut features = toml::map::Map::new();
    features.insert(
        "plugins".to_string(),
        Value::Boolean(plugins_feature_enabled),
    );
    root.insert("features".to_string(), Value::Table(features));

    let mut plugin = toml::map::Map::new();
    plugin.insert("enabled".to_string(), Value::Boolean(enabled));

    let mut plugins = toml::map::Map::new();
    plugins.insert("sample@test".to_string(), Value::Table(plugin));
    root.insert("plugins".to_string(), Value::Table(plugins));

    toml::to_string(&Value::Table(root)).expect("plugin test config should serialize")
}

async fn load_plugins_from_config(
    config_toml: &str,
    genex_home: &Path,
    auth_mode: Option<AuthMode>,
) -> PluginLoadOutcome {
    write_file(&genex_home.join(CONFIG_TOML_FILE), config_toml);
    let config = load_config(genex_home, genex_home).await;
    test_plugins_manager_with_options(genex_home.to_path_buf(), Some(Product::Codex), auth_mode)
        .plugins_for_config(&config)
        .await
}

async fn load_config(genex_home: &Path, cwd: &Path) -> PluginsConfigInput {
    load_plugins_config_input(genex_home, cwd).await
}

#[tokio::test]
async fn load_plugins_loads_manifest_mcp_server_objects() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/counter-sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{
  "name": "counter-sample",
  "version": "1.1.1",
  "description": "Plugin that declares MCP servers in the manifest",
  "mcpServers": {
    "counter": {
      "type": "http",
      "url": "https://sample.example/counter/mcp"
    }
  }
}"#,
    );

    let config_toml = r#"
[features]
plugins = true

[plugins."counter-sample@test"]
enabled = true
"#;
    let outcome =
        load_plugins_from_config(config_toml, genex_home.path(), /*auth_mode*/ None).await;

    assert_eq!(outcome.plugins()[0].error, None);
    assert_eq!(
        outcome.plugins()[0].mcp_servers,
        HashMap::from([(
            "counter".to_string(),
            McpServerConfig {
                auth: Default::default(),
                transport: McpServerTransportConfig::StreamableHttp {
                    url: "https://sample.example/counter/mcp".to_string(),
                    bearer_token_env_var: None,
                    http_headers: None,
                    env_http_headers: None,
                    http_headers_helper: None,
                },
                environment_id: "local".to_string(),
                enabled: true,
                required: false,
                supports_parallel_tool_calls: false,
                omit_tools_from: None,
                disabled_reason: None,
                startup_timeout_sec: None,
                tool_timeout_sec: None,
                default_tools_approval_mode: None,
                enabled_tools: None,
                disabled_tools: None,
                scopes: None,
                oauth: None,
                oauth_resource: None,
                tools: HashMap::new(),
            },
        )])
    );
}

#[tokio::test]
async fn load_plugins_applies_plugin_mcp_server_policy() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{
  "name": "sample"
}"#,
    );
    write_file(
        &plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "type": "http",
      "url": "https://sample.example/mcp",
      "default_tools_approval_mode": "prompt",
      "enabled_tools": ["read", "search"],
      "tools": {
        "search": { "approval_mode": "prompt", "output_token_limit": 8000 }
      }
    }
  }
}"#,
    );
    let config_toml = r#"
[features]
plugins = true

[plugins."sample@test"]
enabled = true

[plugins."sample@test".mcp_servers.sample]
enabled = false
default_tools_approval_mode = "approve"
enabled_tools = ["search"]
disabled_tools = ["delete"]

[plugins."sample@test".mcp_servers.sample.tools.search]
approval_mode = "approve"
output_token_limit = 12000
"#;

    let outcome =
        load_plugins_from_config(config_toml, genex_home.path(), /*auth_mode*/ None).await;
    let server = outcome.plugins()[0]
        .mcp_servers
        .get("sample")
        .expect("sample server");

    assert!(!server.enabled);
    assert_eq!(
        server.default_tools_approval_mode,
        Some(AppToolApproval::Approve)
    );
    assert_eq!(server.enabled_tools, Some(vec!["search".to_string()]));
    assert_eq!(server.disabled_tools, Some(vec!["delete".to_string()]));
    assert_eq!(
        server.tools.get("search"),
        Some(&McpServerToolConfig {
            approval_mode: Some(AppToolApproval::Approve),
            output_token_limit: std::num::NonZeroUsize::new(8_000),
        })
    );
}

#[tokio::test]
async fn load_plugins_resolves_disabled_skill_names_against_loaded_plugin_skills() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    let skill_path = plugin_root.join("skills/sample-search/SKILL.md");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    );
    write_file(
        &skill_path,
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );

    let config_toml = r#"[features]
plugins = true

[[skills.config]]
name = "sample:sample-search"
enabled = false

[plugins."sample@test"]
enabled = true
"#;
    let outcome =
        load_plugins_from_config(config_toml, genex_home.path(), /*auth_mode*/ None).await;
    let skill_path = std::fs::canonicalize(skill_path)
        .expect("skill path should canonicalize")
        .abs();

    assert_eq!(
        outcome.plugins()[0].disabled_skill_paths,
        HashSet::from([skill_path])
    );
    assert!(!outcome.plugins()[0].has_enabled_skills);
    assert!(outcome.capability_summaries().is_empty());
}

#[tokio::test]
async fn load_plugins_ignores_unknown_disabled_skill_names() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    );
    write_file(
        &plugin_root.join("skills/sample-search/SKILL.md"),
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );

    let config_toml = r#"[features]
plugins = true

[[skills.config]]
name = "sample:missing-skill"
enabled = false

[plugins."sample@test"]
enabled = true
"#;
    let outcome =
        load_plugins_from_config(config_toml, genex_home.path(), /*auth_mode*/ None).await;

    assert!(outcome.plugins()[0].disabled_skill_paths.is_empty());
    assert!(outcome.plugins()[0].has_enabled_skills);
    assert_eq!(
        outcome.capability_summaries(),
        &[PluginCapabilitySummary {
            config_name: "sample@test".to_string(),
            display_name: "sample".to_string(),
            plugin_namespace: Some("sample".to_string()),
            description: None,
            has_skills: true,
            mcp_server_names: Vec::new(),
        }]
    );
}

#[tokio::test]
async fn plugin_capability_summary_uses_manifest_mcp_server_objects() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/counter-sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{
  "name": "counter-sample",
  "version": "1.1.1",
  "mcpServers": {
    "counter": {
      "type": "http",
      "url": "https://sample.example/counter/mcp"
    }
  }
}"#,
    );

    let summary = plugin_capability_summary_from_root(
        &PluginId::parse("counter-sample@test").expect("plugin id should parse"),
        &plugin_root.abs(),
        test_skill_root_loader().as_ref(),
    )
    .await;

    assert_eq!(
        summary,
        Some(PluginCapabilitySummary {
            config_name: "counter-sample@test".to_string(),
            display_name: "counter-sample".to_string(),
            plugin_namespace: Some("counter-sample".to_string()),
            description: None,
            has_skills: false,
            mcp_server_names: vec!["counter".to_string()],
        })
    );
}

#[tokio::test]
async fn capability_summary_sanitizes_plugin_descriptions_to_one_line() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{
  "name": "sample",
  "description": "Plugin that\n includes   the sample\tserver"
}"#,
    );
    write_file(
        &plugin_root.join("skills/sample-search/SKILL.md"),
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(/*enabled*/ true, /*plugins_feature_enabled*/ true),
        genex_home.path(),
        /*auth_mode*/ None,
    )
    .await;

    assert_eq!(
        outcome.plugins()[0].manifest_description.as_deref(),
        Some("Plugin that\n includes   the sample\tserver")
    );
    assert_eq!(
        outcome.capability_summaries()[0].description.as_deref(),
        Some("Plugin that includes the sample server")
    );
}

#[tokio::test]
async fn capability_summary_truncates_overlong_plugin_descriptions() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    let too_long = "x".repeat(MAX_CAPABILITY_SUMMARY_DESCRIPTION_LEN + 1);

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        &format!(
            r#"{{
  "name": "sample",
  "description": "{too_long}"
}}"#
        ),
    );
    write_file(
        &plugin_root.join("skills/sample-search/SKILL.md"),
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(/*enabled*/ true, /*plugins_feature_enabled*/ true),
        genex_home.path(),
        /*auth_mode*/ None,
    )
    .await;

    assert_eq!(
        outcome.plugins()[0].manifest_description.as_deref(),
        Some(too_long.as_str())
    );
    assert_eq!(
        outcome.capability_summaries()[0].description,
        Some("x".repeat(MAX_CAPABILITY_SUMMARY_DESCRIPTION_LEN))
    );
}

#[tokio::test]
async fn install_plugin_materializes_default_command_skills() {
    let genex_home = TempDir::new().unwrap();
    let source_root = genex_home.path().join("source/sample");

    write_file(
        &source_root.join(".genex-plugin/plugin.json"),
        r#"{
  "name": "sample",
  "skills": "./custom-skills/"
}"#,
    );
    fs::create_dir_all(source_root.join("custom-skills")).unwrap();
    write_file(
        &source_root.join("custom-skills/source-command-pr-review/SKILL.md"),
        "---\nname: source-command-pr-review\ndescription: Native review skill\n---\n",
    );
    write_file(
        &source_root.join("commands/pr/review.md"),
        "---\ndescription: Review a pull request\n---\nInspect the proposed changes.\n",
    );
    write_file(
        &source_root.join("commands/summarize.md"),
        "---\ndescription: Summarize a change\n---\nSummarize the proposed changes.\n",
    );
    write_file(
        &source_root.join("commands/oversized.md"),
        &format!("---\ndescription: Oversized\n---\n{}", "x".repeat(4_000)),
    );
    write_file(
        &source_root.join(".genex-plugin/migrated-command-skills/undeclared-command/SKILL.md"),
        "---\nname: undeclared-command\ndescription: undeclared command\n---\n",
    );
    let result = PluginStore::new(genex_home.path().to_path_buf())
        .install(
            source_root.abs(),
            PluginId::parse("sample@test").expect("plugin id should parse"),
        )
        .unwrap();
    let migrated_skill = result
        .installed_path
        .join(".genex-plugin/migrated-command-skills/source-command-pr-review/SKILL.md");
    let expected_migrated_skill = "---\nname: \"source-command-pr-review\"\ndescription: \"Review a pull request\"\n---\n\n# source-command-pr-review\n\nUse this skill when the user asks to run the migrated source command `pr-review`.\n\n## Command Template\n\nInspect the proposed changes.\n";
    assert_eq!(
        fs::read_to_string(&migrated_skill).unwrap(),
        expected_migrated_skill
    );
    assert!(
        !result
            .installed_path
            .join(".genex-plugin/migrated-command-skills/undeclared-command")
            .exists()
    );
    assert!(
        !result
            .installed_path
            .join(".genex-plugin/migrated-command-skills/source-command-oversized")
            .exists()
    );

    let manifest = crate::manifest::load_plugin_manifest(&result.installed_path).unwrap();
    let resolved = load_plugin_skill_inventory(
        &result.installed_path,
        &PluginIdentity {
            plugin_id: result.plugin_id.as_key(),
            remote_plugin_id: None,
        },
        &manifest,
        PluginManifestFormat::Legacy,
        /*restriction_product*/ None,
        /*plugin_skill_snapshots*/ None,
        test_skill_root_loader().as_ref(),
    )
    .await
    .resolve(&SkillConfigRules::default());
    assert_eq!(
        resolved
            .skills
            .iter()
            .map(|skill| skill.path_to_skills_md.clone())
            .collect::<Vec<_>>(),
        vec![
            AbsolutePathBuf::from_absolute_path_checked(
                fs::canonicalize(
                    result
                        .installed_path
                        .join("custom-skills/source-command-pr-review/SKILL.md")
                )
                .unwrap()
            )
            .unwrap(),
            AbsolutePathBuf::from_absolute_path_checked(
                fs::canonicalize(result.installed_path.join(
                    ".genex-plugin/migrated-command-skills/source-command-summarize/SKILL.md"
                ))
                .unwrap()
            )
            .unwrap()
        ]
    );
}

#[test]
fn install_plugin_ignores_invalid_commands_manifest_field() {
    let genex_home = TempDir::new().unwrap();
    let source_root = genex_home.path().join("source/sample");
    write_file(
        &source_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample","commands":{}}"#,
    );
    write_file(
        &source_root.join("commands/review.md"),
        "---\ndescription: Review\n---\nReview the current change.\n",
    );

    let result = PluginStore::new(genex_home.path().to_path_buf())
        .install(
            source_root.abs(),
            PluginId::parse("sample@test").expect("plugin id should parse"),
        )
        .unwrap();

    assert!(
        !result
            .installed_path
            .join(".genex-plugin/migrated-command-skills")
            .exists()
    );
}

#[test]
fn install_plugin_ignores_command_migration_errors() {
    let genex_home = TempDir::new().unwrap();
    let source_root = genex_home.path().join("source/sample");
    write_file(
        &source_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample","commands":"./commands/review.md"}"#,
    );
    fs::create_dir_all(source_root.join("commands")).unwrap();
    fs::write(source_root.join("commands/review.md"), [0xff]).unwrap();

    let result = PluginStore::new(genex_home.path().to_path_buf())
        .install(
            source_root.abs(),
            PluginId::parse("sample@test").expect("plugin id should parse"),
        )
        .unwrap();

    assert!(result.installed_path.join("commands/review.md").is_file());
}

#[tokio::test]
async fn load_plugin_skills_dedupes_overlapping_manifest_roots() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local")
        .abs();
    write_file(
        &plugin_root.join("skills/abc/SKILL.md"),
        "---\nname: abc\ndescription: abc skill\n---\n",
    );
    write_file(
        &plugin_root.join("skills/edk/SKILL.md"),
        "---\nname: edk\ndescription: edk skill\n---\n",
    );
    let manifest = crate::manifest::PluginManifest {
        name: "sample".to_string(),
        version: None,
        description: None,
        keywords: Vec::new(),
        paths: crate::manifest::PluginManifestPaths {
            skills: vec![
                plugin_root.join("skills"),
                plugin_root.join("skills/abc"),
                plugin_root.join("skills/edk"),
                plugin_root.join("skills/abc"),
            ],
            mcp_servers: None,
            hooks: None,
        },
        interface: None,
    };
    let plugin_id = PluginId::parse("sample@test").expect("plugin id should parse");
    let resolved = load_plugin_skill_inventory(
        &plugin_root,
        &PluginIdentity {
            plugin_id: plugin_id.as_key(),
            remote_plugin_id: None,
        },
        &manifest,
        PluginManifestFormat::Legacy,
        /*restriction_product*/ None,
        /*plugin_skill_snapshots*/ None,
        test_skill_root_loader().as_ref(),
    )
    .await
    .resolve(&SkillConfigRules::default());

    let skill_paths = resolved
        .skills
        .iter()
        .map(|skill| skill.path_to_skills_md.clone())
        .collect::<Vec<_>>();
    let canonical_skill_path = |path| {
        AbsolutePathBuf::from_absolute_path_checked(
            fs::canonicalize(plugin_root.join(path)).expect("canonical skill path"),
        )
        .expect("absolute skill path")
    };
    assert_eq!(
        skill_paths,
        vec![
            canonical_skill_path("skills/abc/SKILL.md"),
            canonical_skill_path("skills/edk/SKILL.md")
        ]
    );
}

#[tokio::test]
async fn load_plugins_ignores_invalid_manifest_skills_shape() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{
  "name": "sample",
  "skills": { "path": "./custom-skills/" }
}"#,
    );
    write_file(
        &plugin_root.join("skills/default-skill/SKILL.md"),
        "---\nname: default-skill\ndescription: default skill\n---\n",
    );
    write_file(
        &plugin_root.join("custom-skills/custom-skill/SKILL.md"),
        "---\nname: custom-skill\ndescription: custom skill\n---\n",
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(/*enabled*/ true, /*plugins_feature_enabled*/ true),
        genex_home.path(),
        /*auth_mode*/ None,
    )
    .await;

    assert_eq!(outcome.plugins()[0].error, None);
    assert_eq!(
        outcome.plugins()[0].skill_roots,
        vec![plugin_root.join("skills").abs()]
    );
}

#[tokio::test]
async fn load_plugins_preserves_disabled_plugins_without_effective_contributions() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    );
    write_file(
        &plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "type": "http",
      "url": "https://sample.example/mcp"
    }
  }
}"#,
    );

    let outcome = load_plugins_from_config(
        &plugin_config_toml(
            /*enabled*/ false, /*plugins_feature_enabled*/ true,
        ),
        genex_home.path(),
        /*auth_mode*/ None,
    )
    .await;

    assert_eq!(
        outcome.plugins(),
        vec![LoadedPlugin {
            config_name: "sample@test".to_string(),
            remote_plugin_id: None,
            manifest_name: None,
            plugin_namespace: None,
            manifest_description: None,
            root: AbsolutePathBuf::try_from(plugin_root).unwrap(),
            enabled: false,
            skill_roots: Vec::new(),
            skill_discovery_mode: SkillDiscoveryMode::Recursive,
            disabled_skill_paths: HashSet::new(),
            has_enabled_skills: false,
            mcp_servers: HashMap::new(),
            hook_sources: Vec::new(),
            hook_load_warnings: Vec::new(),
            error: None,
        }]
    );
    assert!(outcome.effective_plugin_skill_roots().is_empty());
    assert!(outcome.effective_mcp_servers().is_empty());
}

#[tokio::test]
async fn load_plugins_returns_empty_when_feature_disabled() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");

    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    );
    write_file(
        &plugin_root.join("skills/sample-search/SKILL.md"),
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );
    write_file(
        &genex_home.path().join(CONFIG_TOML_FILE),
        &plugin_config_toml(
            /*enabled*/ true, /*plugins_feature_enabled*/ false,
        ),
    );

    let config = load_config(genex_home.path(), genex_home.path()).await;
    let outcome = test_plugins_manager(genex_home.path().to_path_buf())
        .plugins_for_config(&config)
        .await;

    assert_eq!(outcome, PluginLoadOutcome::default());
}

#[tokio::test]
async fn plugin_cache_reuses_effective_configurations() {
    let genex_home = TempDir::new().unwrap();
    let plugin_root = genex_home
        .path()
        .join("plugins/cache")
        .join("test/sample/local");
    write_plugin(
        genex_home.path().join("plugins/cache/test").as_path(),
        "sample/local",
        "sample",
    );
    write_file(
        &plugin_root.join(".mcp.json"),
        r#"{
  "mcpServers": {
    "sample": {
      "url": "https://sample.example/mcp"
    }
  }
}"#,
    );

    let user_file = genex_home.path().join(CONFIG_TOML_FILE).abs();
    let user_config: toml::Value = toml::from_str(&plugin_config_toml(
        /*enabled*/ true, /*plugins_feature_enabled*/ true,
    ))
    .expect("user config should parse");
    let stack = |session_config: &str| {
        ConfigLayerStack::new(
            vec![
                ConfigLayerEntry::new(
                    ConfigLayerSource::User {
                        file: user_file.clone(),
                        profile: None,
                    },
                    user_config.clone(),
                ),
                ConfigLayerEntry::new(
                    ConfigLayerSource::SessionFlags,
                    toml::from_str(session_config).expect("session config should parse"),
                ),
            ],
            ConfigRequirements::default(),
            ConfigRequirementsToml::default(),
        )
        .expect("config layer stack should build")
    };
    let config = |session_config| {
        PluginsConfigInput::new(stack(session_config), /*plugins_enabled*/ true)
    };
    let manager = test_plugins_manager(genex_home.path().to_path_buf());

    let first_config = config(r#"model = "first""#);
    let second_config = config(
        r#"[plugins."sample@test".mcp_servers.sample]
enabled = false"#,
    );
    let first = manager.plugins_for_config(&first_config).await;
    let first_snapshots = manager
        .plugin_skill_snapshots_for_config(&first_config)
        .expect("first configuration snapshots");
    let second = manager.plugins_for_config(&second_config).await;
    let second_snapshots = manager
        .plugin_skill_snapshots_for_config(&second_config)
        .expect("second configuration snapshots");
    std::fs::remove_file(plugin_root.join(".mcp.json")).unwrap();

    assert_eq!(
        manager.plugin_skill_snapshots_for_config(&first_config),
        Some(first_snapshots),
    );
    assert_eq!(
        manager
            .plugins_for_config(&config(r#"model = "second""#))
            .await,
        first,
    );
    assert_eq!(
        manager.plugin_skill_snapshots_for_config(&second_config),
        Some(second_snapshots),
    );
    assert_eq!(manager.plugins_for_config(&second_config).await, second);
    manager.clear_cache();
    assert_eq!(
        [first_config, second_config]
            .iter()
            .map(|config| manager.plugin_skill_snapshots_for_config(config))
            .collect::<Vec<_>>(),
        vec![None, None],
    );
}

#[tokio::test]
async fn install_plugin_updates_config_with_relative_path_and_plugin_key() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&repo_root, "sample-plugin", "sample-plugin");
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      },
      "policy": {
        "authentication": "ON_USE"
      }
    }
  ]
}"#,
    )
    .unwrap();

    let result = test_plugins_manager(tmp.path().to_path_buf())
        .install_plugin(
            &unrestricted_config_layer_stack(),
            PluginInstallRequest {
                plugin_name: "sample-plugin".to_string(),
                marketplace_path: AbsolutePathBuf::try_from(
                    repo_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();

    let installed_path = tmp.path().join("plugins/cache/debug/sample-plugin/local");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("sample-plugin".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "local".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnUse,
        }
    );

    let config = fs::read_to_string(tmp.path().join("config.toml")).unwrap();
    assert!(config.contains(r#"[plugins."sample-plugin@debug"]"#));
    assert!(config.contains("enabled = true"));
}

#[tokio::test]
async fn strict_install_requires_allowed_local_marketplace_to_be_added_first() {
    let genex_home = TempDir::new().expect("create Genex home");
    let marketplace_root = genex_home.path().join("company-marketplace");
    write_plugin(&marketplace_root, "sample", "sample");
    write_file(
        &marketplace_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "company",
  "plugins": [
    {
      "name": "sample",
      "source": {"source": "local", "path": "./sample"}
    }
  ]
}"#,
    );
    let marketplace_root = marketplace_root
        .canonicalize()
        .expect("canonical marketplace root");
    let requirements = format!(
        r#"
[marketplaces]
restrict_to_allowed_sources = true

[marketplaces.allowed_sources.company]
source = "local"
path = {marketplace_root:?}
"#
    );
    let config = config_layer_stack_with_requirements(genex_home.path(), "", &requirements);
    let marketplace_path =
        AbsolutePathBuf::try_from(marketplace_root.join(".agents/plugins/marketplace.json"))
            .expect("absolute marketplace path");
    let manager = test_plugins_manager(genex_home.path().to_path_buf());

    let err = manager
        .install_plugin(
            &config,
            PluginInstallRequest {
                plugin_name: "sample".to_string(),
                marketplace_path: marketplace_path.clone(),
            },
        )
        .await
        .expect_err("unconfigured local marketplace should not be installable in strict mode");
    assert!(matches!(
        err,
        PluginInstallError::Marketplace(MarketplaceError::InvalidMarketplaceFile { .. })
    ));
    assert!(err.to_string().contains("must be added to config"));
    assert!(!genex_home.path().join(CONFIG_TOML_FILE).exists());

    let user_config = format!(
        r#"
[marketplaces.company]
source_type = "local"
source = {marketplace_root:?}
"#
    );
    write_file(&genex_home.path().join(CONFIG_TOML_FILE), &user_config);
    let config =
        config_layer_stack_with_requirements(genex_home.path(), &user_config, &requirements);
    let outcome = manager
        .install_plugin(
            &config,
            PluginInstallRequest {
                plugin_name: "sample".to_string(),
                marketplace_path,
            },
        )
        .await
        .expect("configured allowlisted marketplace should be installable");
    assert_eq!(
        outcome.plugin_id,
        PluginId::new("sample".to_string(), "company".to_string()).expect("plugin id")
    );
}

#[tokio::test]
async fn install_plugin_uses_manifest_version_for_non_curated_plugins() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(
        &repo_root,
        "sample-plugin",
        "sample-plugin",
        Some("1.2.3-beta+7"),
    );
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();

    let result = test_plugins_manager(tmp.path().to_path_buf())
        .install_plugin(
            &unrestricted_config_layer_stack(),
            PluginInstallRequest {
                plugin_name: "sample-plugin".to_string(),
                marketplace_path: AbsolutePathBuf::try_from(
                    repo_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();

    let installed_path = tmp
        .path()
        .join("plugins/cache/debug/sample-plugin/1.2.3-beta+7");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("sample-plugin".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "1.2.3-beta+7".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
}

#[tokio::test]
async fn install_plugin_writes_marketplace_manifest_fallback_when_missing_plugin_json() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    let plugin_root = repo_root.join("plugins/quality-review");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    fs::create_dir_all(plugin_root.join("skills/thermo-nuclear-code-quality-review")).unwrap();
    fs::write(
        plugin_root.join("skills/thermo-nuclear-code-quality-review/SKILL.md"),
        "review skill",
    )
    .unwrap();
    write_file(
        &plugin_root.join("commands/review.md"),
        "---\ndescription: Review code\n---\nReview the current change.\n",
    );
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "quality-review",
      "description": "Strict code quality review focused on maintainability.",
      "source": "./plugins/quality-review",
      "author": {
        "name": "Byron Grogan"
      },
      "skills": [
        "./skills/thermo-nuclear-code-quality-review"
      ],
      "commands": ["./commands/review.md"],
      "category": "code-review"
    }
  ]
}"#,
    )
    .unwrap();

    let result = test_plugins_manager(tmp.path().to_path_buf())
        .install_plugin(
            &unrestricted_config_layer_stack(),
            PluginInstallRequest {
                plugin_name: "quality-review".to_string(),
                marketplace_path: AbsolutePathBuf::try_from(
                    repo_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();

    let installed_path = tmp.path().join("plugins/cache/debug/quality-review/local");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("quality-review".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "local".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path.clone()).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
    assert!(!plugin_root.join(".genex-plugin/plugin.json").exists());
    assert!(
        !tmp.path()
            .join("plugins/.marketplace-plugin-source-staging")
            .exists()
    );

    let manifest = crate::manifest::load_plugin_manifest(&installed_path).unwrap();
    assert_eq!(manifest.name, "quality-review");
    assert_eq!(
        manifest.description.as_deref(),
        Some("Strict code quality review focused on maintainability.")
    );
    assert_eq!(
        manifest.paths.skills,
        vec![
            AbsolutePathBuf::try_from(
                installed_path.join("skills/thermo-nuclear-code-quality-review")
            )
            .unwrap()
        ]
    );
    let interface = manifest.interface.expect("fallback interface");
    assert_eq!(interface.developer_name.as_deref(), Some("Byron Grogan"));
    assert_eq!(interface.category.as_deref(), Some("code-review"));
    let fallback_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(installed_path.join(".genex-plugin/plugin.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        fallback_json["author"],
        serde_json::json!({ "name": "Byron Grogan" })
    );
    assert_eq!(fallback_json["category"], "code-review");
    assert_eq!(
        fs::read_to_string(
            installed_path
                .join(".genex-plugin/migrated-command-skills/source-command-review/SKILL.md")
        )
        .unwrap(),
        "---\nname: \"source-command-review\"\ndescription: \"Review code\"\n---\n\n# source-command-review\n\nUse this skill when the user asks to run the migrated source command `review`.\n\n## Command Template\n\nReview the current change.\n"
    );
}

#[tokio::test]
async fn install_plugin_supports_git_subdir_marketplace_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("marketplace");
    let remote_repo = tmp.path().join("remote-plugin-repo");
    let remote_repo_url = url::Url::from_directory_path(&remote_repo)
        .unwrap()
        .to_string();
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&remote_repo, "plugins/toolkit", "toolkit");
    init_git_repo(&remote_repo);
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        format!(
            r#"{{
  "name": "debug",
  "plugins": [
    {{
      "name": "toolkit",
      "source": {{
        "source": "git-subdir",
        "url": "{remote_repo_url}",
        "path": "plugins/toolkit"
      }}
    }}
  ]
}}"#
        ),
    )
    .unwrap();

    let result = test_plugins_manager(tmp.path().to_path_buf())
        .install_plugin(
            &unrestricted_config_layer_stack(),
            PluginInstallRequest {
                plugin_name: "toolkit".to_string(),
                marketplace_path: AbsolutePathBuf::try_from(
                    repo_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();

    let installed_path = tmp.path().join("plugins/cache/debug/toolkit/local");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("toolkit".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "local".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path.clone()).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
    assert!(installed_path.join(".genex-plugin/plugin.json").is_file());
}

#[tokio::test]
async fn install_plugin_supports_relative_git_subdir_marketplace_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("marketplace");
    let remote_repo = repo_root.join("remote-plugin-repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&remote_repo, "plugins/toolkit", "toolkit");
    init_git_repo(&remote_repo);
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "toolkit",
      "source": {
        "source": "git-subdir",
        "url": "./remote-plugin-repo",
        "path": "plugins/toolkit"
      }
    }
  ]
}"#,
    )
    .unwrap();

    let result = test_plugins_manager(tmp.path().to_path_buf())
        .install_plugin(
            &unrestricted_config_layer_stack(),
            PluginInstallRequest {
                plugin_name: "toolkit".to_string(),
                marketplace_path: AbsolutePathBuf::try_from(
                    repo_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();

    let installed_path = tmp.path().join("plugins/cache/debug/toolkit/local");
    assert_eq!(
        result,
        PluginInstallOutcome {
            plugin_id: PluginId::new("toolkit".to_string(), "debug".to_string()).unwrap(),
            plugin_version: "local".to_string(),
            installed_path: AbsolutePathBuf::try_from(installed_path.clone()).unwrap(),
            auth_policy: MarketplacePluginAuthPolicy::OnInstall,
        }
    );
    assert!(installed_path.join(".genex-plugin/plugin.json").is_file());
}

#[tokio::test]
async fn uninstall_plugin_removes_cache_and_config_entry() {
    let tmp = tempfile::tempdir().unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/local",
        "sample-plugin",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    let manager = test_plugins_manager(tmp.path().to_path_buf());
    manager
        .uninstall_plugin("sample-plugin@debug".to_string())
        .await
        .unwrap();
    manager
        .uninstall_plugin("sample-plugin@debug".to_string())
        .await
        .unwrap();

    assert!(
        !tmp.path()
            .join("plugins/cache/debug/sample-plugin")
            .exists()
    );
    let config = fs::read_to_string(tmp.path().join(CONFIG_TOML_FILE)).unwrap();
    assert!(!config.contains(r#"[plugins."sample-plugin@debug"]"#));
}

#[tokio::test]
async fn list_marketplaces_includes_enabled_state() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "enabled-plugin/local",
        "enabled-plugin",
    );
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "disabled-plugin/local",
        "disabled-plugin",
    );
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "enabled-plugin",
      "source": {
        "source": "local",
        "path": "./enabled-plugin"
      }
    },
    {
      "name": "disabled-plugin",
      "source": {
        "source": "local",
        "path": "./disabled-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."enabled-plugin@debug"]
enabled = true

[plugins."disabled-plugin@debug"]
enabled = false
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("expected repo marketplace entry");

    assert_eq!(
        marketplace,
        ConfiguredMarketplace {
            name: "debug".to_string(),
            path: AbsolutePathBuf::try_from(
                tmp.path().join("repo/.agents/plugins/marketplace.json"),
            )
            .unwrap(),
            interface: None,
            plugins: vec![
                ConfiguredMarketplacePlugin {
                    id: "enabled-plugin@debug".to_string(),
                    name: "enabled-plugin".to_string(),
                    local_version: None,
                    installed_version: Some("local".to_string()),
                    source: MarketplacePluginSource::Local {
                        path: AbsolutePathBuf::try_from(tmp.path().join("repo/enabled-plugin"))
                            .unwrap(),
                    },
                    policy: MarketplacePluginPolicy {
                        installation: MarketplacePluginInstallPolicy::Available,
                        authentication: MarketplacePluginAuthPolicy::OnInstall,
                        products: None,
                    },
                    interface: None,
                    keywords: Vec::new(),
                    manifest_fallback: None,
                    installed: true,
                    enabled: true,
                },
                ConfiguredMarketplacePlugin {
                    id: "disabled-plugin@debug".to_string(),
                    name: "disabled-plugin".to_string(),
                    local_version: None,
                    installed_version: Some("local".to_string()),
                    source: MarketplacePluginSource::Local {
                        path: AbsolutePathBuf::try_from(tmp.path().join("repo/disabled-plugin"),)
                            .unwrap(),
                    },
                    policy: MarketplacePluginPolicy {
                        installation: MarketplacePluginInstallPolicy::Available,
                        authentication: MarketplacePluginAuthPolicy::OnInstall,
                        products: None,
                    },
                    interface: None,
                    keywords: Vec::new(),
                    manifest_fallback: None,
                    installed: true,
                    enabled: false,
                },
            ],
        }
    );
}

#[tokio::test]
async fn list_marketplaces_returns_empty_when_feature_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "enabled-plugin",
      "source": {
        "source": "local",
        "path": "./enabled-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = false

[plugins."enabled-plugin@debug"]
enabled = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    assert_eq!(marketplaces, Vec::new());
}

#[tokio::test]
async fn list_marketplaces_excludes_plugins_with_explicit_empty_products() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "disabled-plugin",
      "source": {
        "source": "local",
        "path": "./disabled-plugin"
      },
      "policy": {
        "products": []
      }
    },
    {
      "name": "default-plugin",
      "source": {
        "source": "local",
        "path": "./default-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("expected repo marketplace entry");
    assert_eq!(
        marketplace.plugins,
        vec![ConfiguredMarketplacePlugin {
            id: "default-plugin@debug".to_string(),
            name: "default-plugin".to_string(),
            local_version: None,
            installed_version: None,
            source: MarketplacePluginSource::Local {
                path: AbsolutePathBuf::try_from(tmp.path().join("repo/default-plugin")).unwrap(),
            },
            policy: MarketplacePluginPolicy {
                installation: MarketplacePluginInstallPolicy::Available,
                authentication: MarketplacePluginAuthPolicy::OnInstall,
                products: None,
            },
            interface: None,
            keywords: Vec::new(),
            manifest_fallback: None,
            installed: false,
            enabled: false,
        }]
    );
}

#[tokio::test]
async fn read_plugin_for_config_returns_plugins_disabled_when_feature_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    let marketplace_path =
        AbsolutePathBuf::try_from(repo_root.join(".agents/plugins/marketplace.json")).unwrap();
    fs::write(
        marketplace_path.as_path(),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "enabled-plugin",
      "source": {
        "source": "local",
        "path": "./enabled-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = false

[plugins."enabled-plugin@debug"]
enabled = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let err = test_plugins_manager(tmp.path().to_path_buf())
        .read_plugin_for_config(
            &config,
            &PluginReadRequest {
                plugin_name: "enabled-plugin".to_string(),
                marketplace_path,
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(err, MarketplaceError::PluginsDisabled));
}

#[tokio::test]
async fn read_plugin_for_config_does_not_fallback_from_invalid_plugin_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    let plugin_root = repo_root.join("sample-plugin");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": "./sample-plugin",
      "description": "Fallback metadata"
    }
  ]
}"#,
    );
    write_file(&plugin_root.join(".genex-plugin/plugin.json"), "{");
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let err = test_plugins_manager(tmp.path().to_path_buf())
        .read_plugin_for_config(
            &config,
            &PluginReadRequest {
                plugin_name: "sample-plugin".to_string(),
                marketplace_path: AbsolutePathBuf::try_from(
                    repo_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "missing or invalid plugin.json");
}

#[tokio::test]
async fn read_plugin_for_config_uses_user_layer_skill_settings_only() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    let plugin_root = repo_root.join("enabled-plugin");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "enabled-plugin",
      "source": {
        "source": "local",
        "path": "./enabled-plugin"
      }
    }
  ]
}"#,
    );
    write_file(
        &plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"enabled-plugin"}"#,
    );
    write_file(
        &plugin_root.join("skills/sample-search/SKILL.md"),
        "---\nname: sample-search\ndescription: search sample data\n---\n",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."enabled-plugin@debug"]
enabled = true
"#,
    );
    write_file(
        &repo_root.join(".genex/config.toml"),
        r#"[[skills.config]]
name = "enabled-plugin:sample-search"
enabled = false
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let outcome = test_plugins_manager(tmp.path().to_path_buf())
        .read_plugin_for_config(
            &config,
            &PluginReadRequest {
                plugin_name: "enabled-plugin".to_string(),
                marketplace_path: AbsolutePathBuf::try_from(
                    repo_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap(),
            },
        )
        .await
        .unwrap();

    assert!(outcome.plugin.disabled_skill_paths.is_empty());
}

#[tokio::test]
async fn list_marketplaces_installed_git_source_reads_metadata_from_cache_without_cloning() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    let missing_remote_repo = tmp.path().join("missing-remote-plugin-repo");
    let missing_remote_repo_url = url::Url::from_directory_path(&missing_remote_repo)
        .unwrap()
        .to_string();
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        &format!(
            r#"{{
  "name": "debug",
  "plugins": [
    {{
      "name": "toolkit",
      "source": {{
        "source": "git-subdir",
        "url": "{missing_remote_repo_url}",
        "path": "plugins/toolkit"
      }},
      "category": "Developer Tools"
    }}
  ]
}}"#
        ),
    );
    let cached_plugin_root = tmp.path().join("plugins/cache/debug/toolkit/local");
    write_file(
        &cached_plugin_root.join(".genex-plugin/plugin.json"),
        r##"{
  "name": "toolkit",
  "interface": {
    "displayName": "Toolkit",
    "shortDescription": "Search cached data",
    "category": "Cached Category",
    "brandColor": "#3B82F6",
    "composerIcon": "./assets/icon.png",
    "logo": "./assets/logo.png",
    "screenshots": ["./assets/screenshot.png"]
  }
}"##,
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."toolkit@debug"]
enabled = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(
            &config,
            &[AbsolutePathBuf::try_from(repo_root.clone()).unwrap()],
        )
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(repo_root.join(".agents/plugins/marketplace.json"))
                    .unwrap()
        })
        .expect("debug marketplace should be listed");

    let mut plugins = marketplace.plugins;
    assert!(plugins[0].manifest_fallback.is_some());
    plugins[0].manifest_fallback = None;
    assert_eq!(
        plugins,
        vec![ConfiguredMarketplacePlugin {
            id: "toolkit@debug".to_string(),
            name: "toolkit".to_string(),
            local_version: None,
            installed_version: Some("local".to_string()),
            source: MarketplacePluginSource::Git {
                url: missing_remote_repo_url,
                path: Some("plugins/toolkit".to_string()),
                ref_name: None,
                sha: None,
            },
            policy: MarketplacePluginPolicy {
                installation: MarketplacePluginInstallPolicy::Available,
                authentication: MarketplacePluginAuthPolicy::OnInstall,
                products: None,
            },
            interface: Some(PluginManifestInterface {
                display_name: Some("Toolkit".to_string()),
                short_description: Some("Search cached data".to_string()),
                category: Some("Developer Tools".to_string()),
                brand_color: Some("#3B82F6".to_string()),
                composer_icon: Some(
                    AbsolutePathBuf::try_from(cached_plugin_root.join("assets/icon.png")).unwrap(),
                ),
                logo: Some(
                    AbsolutePathBuf::try_from(cached_plugin_root.join("assets/logo.png")).unwrap(),
                ),
                screenshots: vec![
                    AbsolutePathBuf::try_from(cached_plugin_root.join("assets/screenshot.png"))
                        .unwrap(),
                ],
                ..Default::default()
            }),
            keywords: Vec::new(),
            manifest_fallback: None,
            installed: true,
            enabled: true,
        }]
    );
    assert!(
        !tmp.path()
            .join("plugins/.marketplace-plugin-source-staging")
            .exists()
    );
}

#[tokio::test]
async fn list_marketplaces_skips_missing_api_curated_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let curated_root = curated_plugins_repo_path(tmp.path());

    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );
    write_file(
        &curated_root.join(".agents/plugins/marketplace.json"),
        "{not valid json",
    );

    let config = load_config(tmp.path(), tmp.path()).await;
    let manager = test_plugins_manager_with_options(
        tmp.path().to_path_buf(),
        Some(Product::Codex),
        Some(AuthMode::ApiKey),
    );
    let outcome = manager.list_marketplaces_for_config(&config, &[]).unwrap();

    assert_eq!(outcome.errors, Vec::new());
    assert_eq!(
        outcome
            .marketplaces
            .iter()
            .any(|marketplace| marketplace.name == TEST_API_CURATED_MARKETPLACE_NAME),
        false
    );
}

#[tokio::test]
async fn list_marketplaces_includes_installed_marketplace_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let marketplace_root = marketplace_install_root(tmp.path()).join("debug");
    let plugin_root = marketplace_root.join("plugins/sample");

    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[marketplaces.debug]
last_updated = "2026-04-10T12:34:56Z"
source_type = "git"
source = "/tmp/debug"
"#,
    );
    fs::create_dir_all(marketplace_root.join(".agents/plugins")).unwrap();
    fs::create_dir_all(plugin_root.join(".genex-plugin")).unwrap();
    fs::write(
        marketplace_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample",
      "source": {
        "source": "local",
        "path": "./plugins/sample"
      }
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(
        plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )
    .unwrap();
    let config = load_config(tmp.path(), tmp.path()).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    marketplace_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("installed marketplace should be listed");

    assert_eq!(
        marketplace.path,
        AbsolutePathBuf::try_from(marketplace_root.join(".agents/plugins/marketplace.json"))
            .unwrap()
    );
    assert_eq!(marketplace.plugins.len(), 1);
    assert_eq!(marketplace.plugins[0].id, "sample@debug");
    assert_eq!(
        marketplace.plugins[0].source,
        MarketplacePluginSource::Local {
            path: AbsolutePathBuf::try_from(plugin_root).unwrap(),
        }
    );
}

#[tokio::test]
async fn configured_marketplace_upgrade_invalidates_cached_tool_suggest_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let remote_repo = tmp.path().join("remote-marketplace");
    let remote_repo_url = url::Url::from_directory_path(&remote_repo)
        .unwrap()
        .to_string();
    write_file(
        &remote_repo.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample",
      "source": {
        "source": "local",
        "path": "./plugins/sample"
      }
    }
  ]
}"#,
    );
    write_curated_plugin(&remote_repo, "sample");
    write_file(
        &remote_repo.join("plugins/sample/.genex-plugin/plugin.json"),
        r#"{"name":"sample","description":"Before upgrade"}"#,
    );
    init_git_repo(&remote_repo);
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        &format!(
            r#"[features]
plugins = true

[marketplaces.debug]
source_type = "git"
source = "{remote_repo_url}"
"#
        ),
    );

    let manager = test_plugins_manager(tmp.path().to_path_buf());
    let config = load_config(tmp.path(), tmp.path()).await;
    let reload_stack = config.config_layer_stack.clone();
    let reload_config: ConfigLayerReload = Arc::new(move || Ok(reload_stack.clone()));
    let initial_upgrade = manager
        .upgrade_configured_marketplaces_for_config(
            &config,
            /*marketplace_name*/ None,
            &reload_config,
        )
        .expect("initial marketplace install should succeed");
    assert_eq!(initial_upgrade.errors, Vec::new());
    assert_eq!(initial_upgrade.upgraded_roots.len(), 1);

    let config = load_config(tmp.path(), tmp.path()).await;
    let input = ToolSuggestPluginDiscoveryInput {
        plugins: config.clone(),
        configured_plugin_ids: HashSet::from(["sample@debug".to_string()]),
        disabled_plugin_ids: HashSet::new(),
    };
    let expected = ToolSuggestDiscoverablePlugin {
        id: "sample@debug".to_string(),
        remote_plugin_id: None,
        name: "sample".to_string(),
        description: Some("Before upgrade".to_string()),
        has_skills: true,
        mcp_server_names: vec!["sample-docs".to_string()],
    };
    assert_eq!(
        manager
            .list_tool_suggest_discoverable_plugins(&input)
            .await
            .expect("initial tool-suggest metadata should load"),
        vec![expected.clone()]
    );

    write_file(
        &remote_repo.join("plugins/sample/.genex-plugin/plugin.json"),
        r#"{"name":"sample","description":"After upgrade"}"#,
    );
    run_git(&remote_repo, &["add", "."]);
    run_git(&remote_repo, &["commit", "-m", "update plugin"]);
    let upgrade = manager
        .upgrade_configured_marketplaces_for_config(&config, Some("debug"), &reload_config)
        .expect("marketplace upgrade should succeed");
    assert_eq!(upgrade.errors, Vec::new());
    assert_eq!(upgrade.upgraded_roots.len(), 1);

    assert_eq!(
        manager
            .list_tool_suggest_discoverable_plugins(&input)
            .await
            .expect("refreshed tool-suggest metadata should load"),
        vec![ToolSuggestDiscoverablePlugin {
            description: Some("After upgrade".to_string()),
            ..expected
        }]
    );
}

#[tokio::test]
async fn list_marketplaces_uses_config_when_known_registry_is_malformed() {
    let tmp = tempfile::tempdir().unwrap();
    let marketplace_root = marketplace_install_root(tmp.path()).join("debug");
    let plugin_root = marketplace_root.join("plugins/sample");
    let registry_path = tmp.path().join(".tmp/known_marketplaces.json");

    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[marketplaces.debug]
last_updated = "2026-04-10T12:34:56Z"
source_type = "git"
source = "/tmp/debug"
"#,
    );
    fs::create_dir_all(marketplace_root.join(".agents/plugins")).unwrap();
    fs::create_dir_all(plugin_root.join(".genex-plugin")).unwrap();
    fs::write(
        marketplace_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample",
      "source": {
        "source": "local",
        "path": "./plugins/sample"
      }
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(
        plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )
    .unwrap();
    fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
    fs::write(registry_path, "{not valid json").unwrap();

    let config = load_config(tmp.path(), tmp.path()).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    marketplace_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("configured marketplace should be discovered");

    assert_eq!(marketplace.plugins[0].id, "sample@debug");
}

#[tokio::test]
async fn list_marketplaces_ignores_installed_roots_missing_from_config() {
    let tmp = tempfile::tempdir().unwrap();
    let marketplace_root = marketplace_install_root(tmp.path()).join("debug");
    let plugin_root = marketplace_root.join("plugins/sample");

    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true
"#,
    );
    fs::create_dir_all(marketplace_root.join(".agents/plugins")).unwrap();
    fs::create_dir_all(plugin_root.join(".genex-plugin")).unwrap();
    fs::write(
        marketplace_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample",
      "source": {
        "source": "local",
        "path": "./plugins/sample"
      }
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(
        plugin_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"sample"}"#,
    )
    .unwrap();
    let config = load_config(tmp.path(), tmp.path()).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[])
        .unwrap()
        .marketplaces;

    assert!(
        marketplaces.iter().all(|marketplace| {
            marketplace.path
                != AbsolutePathBuf::try_from(
                    marketplace_root.join(".agents/plugins/marketplace.json"),
                )
                .unwrap()
        }),
        "installed marketplace root missing from config should not be listed"
    );
}

#[tokio::test]
async fn list_marketplaces_uses_first_duplicate_plugin_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_a_root = tmp.path().join("repo-a");
    let repo_b_root = tmp.path().join("repo-b");
    fs::create_dir_all(repo_a_root.join(".git")).unwrap();
    fs::create_dir_all(repo_b_root.join(".git")).unwrap();
    fs::create_dir_all(repo_a_root.join(".agents/plugins")).unwrap();
    fs::create_dir_all(repo_b_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_a_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "dup-plugin",
      "source": {
        "source": "local",
        "path": "./from-a"
      }
    }
  ]
}"#,
    )
    .unwrap();
    fs::write(
        repo_b_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "dup-plugin",
      "source": {
        "source": "local",
        "path": "./from-b"
      }
    },
    {
      "name": "b-only-plugin",
      "source": {
        "source": "local",
        "path": "./from-b-only"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."dup-plugin@debug"]
enabled = true

[plugins."b-only-plugin@debug"]
enabled = false
"#,
    );

    let config = load_config(tmp.path(), &repo_a_root).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(
            &config,
            &[
                AbsolutePathBuf::try_from(repo_a_root).unwrap(),
                AbsolutePathBuf::try_from(repo_b_root).unwrap(),
            ],
        )
        .unwrap()
        .marketplaces;

    let repo_a_marketplace = marketplaces
        .iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo-a/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("repo-a marketplace should be listed");
    assert_eq!(
        repo_a_marketplace.plugins,
        vec![ConfiguredMarketplacePlugin {
            id: "dup-plugin@debug".to_string(),
            name: "dup-plugin".to_string(),
            local_version: None,
            installed_version: None,
            source: MarketplacePluginSource::Local {
                path: AbsolutePathBuf::try_from(tmp.path().join("repo-a/from-a")).unwrap(),
            },
            policy: MarketplacePluginPolicy {
                installation: MarketplacePluginInstallPolicy::Available,
                authentication: MarketplacePluginAuthPolicy::OnInstall,
                products: None,
            },
            interface: None,
            keywords: Vec::new(),
            manifest_fallback: None,
            installed: false,
            enabled: true,
        }]
    );

    let repo_b_marketplace = marketplaces
        .iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo-b/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("repo-b marketplace should be listed");
    assert_eq!(
        repo_b_marketplace.plugins,
        vec![ConfiguredMarketplacePlugin {
            id: "b-only-plugin@debug".to_string(),
            name: "b-only-plugin".to_string(),
            local_version: None,
            installed_version: None,
            source: MarketplacePluginSource::Local {
                path: AbsolutePathBuf::try_from(tmp.path().join("repo-b/from-b-only")).unwrap(),
            },
            policy: MarketplacePluginPolicy {
                installation: MarketplacePluginInstallPolicy::Available,
                authentication: MarketplacePluginAuthPolicy::OnInstall,
                products: None,
            },
            interface: None,
            keywords: Vec::new(),
            manifest_fallback: None,
            installed: false,
            enabled: false,
        }]
    );

    let duplicate_plugin_count = marketplaces
        .iter()
        .flat_map(|marketplace| marketplace.plugins.iter())
        .filter(|plugin| plugin.name == "dup-plugin")
        .count();
    assert_eq!(duplicate_plugin_count, 1);
}

#[tokio::test]
async fn list_marketplaces_marks_configured_plugin_uninstalled_when_cache_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    fs::write(
        repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    let config = load_config(tmp.path(), &repo_root).await;
    let marketplaces = test_plugins_manager(tmp.path().to_path_buf())
        .list_marketplaces_for_config(&config, &[AbsolutePathBuf::try_from(repo_root).unwrap()])
        .unwrap()
        .marketplaces;

    let marketplace = marketplaces
        .into_iter()
        .find(|marketplace| {
            marketplace.path
                == AbsolutePathBuf::try_from(
                    tmp.path().join("repo/.agents/plugins/marketplace.json"),
                )
                .unwrap()
        })
        .expect("expected repo marketplace entry");

    assert_eq!(
        marketplace,
        ConfiguredMarketplace {
            name: "debug".to_string(),
            path: AbsolutePathBuf::try_from(
                tmp.path().join("repo/.agents/plugins/marketplace.json"),
            )
            .unwrap(),
            interface: None,
            plugins: vec![ConfiguredMarketplacePlugin {
                id: "sample-plugin@debug".to_string(),
                name: "sample-plugin".to_string(),
                local_version: None,
                installed_version: None,
                source: MarketplacePluginSource::Local {
                    path: AbsolutePathBuf::try_from(tmp.path().join("repo/sample-plugin")).unwrap(),
                },
                policy: MarketplacePluginPolicy {
                    installation: MarketplacePluginInstallPolicy::Available,
                    authentication: MarketplacePluginAuthPolicy::OnInstall,
                    products: None,
                },
                interface: None,
                keywords: Vec::new(),
                manifest_fallback: None,
                installed: false,
                enabled: true,
            }],
        }
    );
}

#[test]
fn refresh_non_curated_plugin_cache_replaces_existing_local_version_with_manifest_version() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/local",
        "sample-plugin",
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
            &["sample-plugin@debug".to_string()],
        )
        .expect("cache refresh should succeed")
    );

    assert!(
        !tmp.path()
            .join("plugins/cache/debug/sample-plugin/local")
            .exists()
    );
    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}

#[tokio::test]
async fn non_curated_plugin_cache_refresh_preserves_manual_priority() {
    let genex_home = TempDir::new().unwrap();
    let marketplace_root = genex_home.path().join("marketplace");
    fs::create_dir_all(marketplace_root.join(".git")).unwrap();
    write_plugin_with_version(
        &marketplace_root,
        "sample-plugin",
        "sample-plugin",
        Some("1.0.0"),
    );
    write_file(
        &marketplace_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [{
    "name": "sample-plugin",
    "source": { "source": "local", "path": "./sample-plugin" }
  }]
}"#,
    );
    write_file(
        &genex_home.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );
    let config = load_plugins_config_input(genex_home.path(), &marketplace_root).await;
    let context = PluginMarketplaceContext {
        global_config: config.clone(),
        scopes: vec![PluginMarketplaceScope {
            cwd: Some(AbsolutePathBuf::try_from(marketplace_root.clone()).unwrap()),
            config,
        }],
        load_errors: Vec::new(),
    };
    let manager = Arc::new(test_plugins_manager(genex_home.path().to_path_buf()));
    manager
        .non_curated_cache_refresh_state
        .write()
        .expect("refresh state lock")
        .in_flight = true;
    let marketplaces = manager
        .list_marketplaces_for_context(&context)
        .unwrap()
        .marketplaces;

    for git_mode in [
        PluginGitMode::Automatic,
        PluginGitMode::Manual,
        PluginGitMode::Automatic,
    ] {
        let request = context
            .non_curated_cache_refresh_request(
                &manager,
                &marketplaces,
                NonCuratedCacheRefreshMode::IfVersionChanged,
                git_mode,
            )
            .expect("refresh request");
        manager.schedule_non_curated_plugin_cache_refresh(request);
    }

    {
        let mut state = manager
            .non_curated_cache_refresh_state
            .write()
            .expect("refresh state lock");
        assert!(matches!(
            state.requested.as_ref().map(|request| request.git_mode),
            Some(PluginGitMode::Manual)
        ));
        state.last_refreshed = state.requested.take();
        state.in_flight = false;
    }

    manager.maybe_start_non_curated_plugin_cache_refresh(&context, &marketplaces);

    {
        let mut state = manager
            .non_curated_cache_refresh_state
            .write()
            .expect("refresh state lock");
        assert!(!state.in_flight);
        assert!(state.requested.is_none());
        state.requested = state.last_refreshed.clone();
        state.in_flight = true;
    }

    write_plugin_with_version(
        &marketplace_root,
        "sample-plugin",
        "sample-plugin",
        Some("2.0.0"),
    );
    let marketplaces = manager
        .list_marketplaces_for_context(&context)
        .unwrap()
        .marketplaces;
    manager.maybe_start_non_curated_plugin_cache_refresh(&context, &marketplaces);

    let state = manager
        .non_curated_cache_refresh_state
        .read()
        .expect("refresh state lock");
    let request = state.requested.as_ref().expect("pending refresh");
    assert!(matches!(request.git_mode, PluginGitMode::Automatic));
    assert_eq!(
        request.configured_plugin_sources[0]
            .local_version
            .as_deref(),
        Some("2.0.0")
    );
}

#[test]
fn refresh_non_curated_plugin_cache_reinstalls_missing_configured_plugin_with_manifest_version() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
            &["sample-plugin@debug".to_string()],
        )
        .expect("cache refresh should reinstall missing configured plugin")
    );

    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}

#[test]
fn refresh_non_curated_plugin_cache_refreshes_configured_git_source() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    let remote_repo = tmp.path().join("remote-plugin-repo");
    let remote_repo_url = url::Url::from_directory_path(&remote_repo)
        .unwrap()
        .to_string();
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    write_plugin_with_version(
        &remote_repo,
        "plugins/sample-plugin",
        "sample-plugin",
        Some("1.2.3"),
    );
    init_git_repo(&remote_repo);
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        &format!(
            r#"{{
  "name": "debug",
  "plugins": [
    {{
      "name": "sample-plugin",
      "source": {{
        "source": "git-subdir",
        "url": "{remote_repo_url}",
        "path": "plugins/sample-plugin"
      }}
    }}
  ]
}}"#
        ),
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
            &["sample-plugin@debug".to_string()],
        )
        .expect("cache refresh should materialize configured Git plugin")
    );

    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}

#[test]
fn refresh_non_curated_plugin_cache_returns_false_when_configured_plugins_are_current() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_plugin_with_version(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/1.2.3",
        "sample-plugin",
        Some("1.2.3"),
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        !refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
            &["sample-plugin@debug".to_string()],
        )
        .expect("cache refresh should be a no-op when configured plugins are current")
    );
}

#[test]
fn refresh_non_curated_plugin_cache_force_reinstalls_current_local_version() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin(&repo_root, "sample-plugin", "sample-plugin");
    fs::write(repo_root.join("sample-plugin/skills/SKILL.md"), "new skill").unwrap();
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    }
  ]
}"#,
    );
    write_plugin(
        &tmp.path().join("plugins/cache/debug"),
        "sample-plugin/local",
        "sample-plugin",
    );
    fs::write(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/local/skills/SKILL.md"),
        "old skill",
    )
    .unwrap();
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache_force_reinstall(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
            &["sample-plugin@debug".to_string()],
        )
        .expect("cache refresh should reinstall unchanged local version")
    );

    assert_eq!(
        fs::read_to_string(
            tmp.path()
                .join("plugins/cache/debug/sample-plugin/local/skills/SKILL.md")
        )
        .unwrap(),
        "new skill"
    );
}

#[test]
fn refresh_non_curated_plugin_cache_ignores_invalid_unconfigured_plugin_versions() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "sample-plugin", "sample-plugin", Some("1.2.3"));
    write_plugin_with_version(&repo_root, "broken-plugin", "broken-plugin", Some("   "));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "sample-plugin",
      "source": {
        "source": "local",
        "path": "./sample-plugin"
      }
    },
    {
      "name": "broken-plugin",
      "source": {
        "source": "local",
        "path": "./broken-plugin"
      }
    }
  ]
}"#,
    );
    write_file(
        &tmp.path().join(CONFIG_TOML_FILE),
        r#"[features]
plugins = true

[plugins."sample-plugin@debug"]
enabled = true
"#,
    );

    assert!(
        refresh_non_curated_plugin_cache(
            tmp.path(),
            &[AbsolutePathBuf::try_from(repo_root).unwrap()],
            &["sample-plugin@debug".to_string()],
        )
        .expect("cache refresh should ignore unrelated invalid plugin manifests")
    );

    assert!(
        tmp.path()
            .join("plugins/cache/debug/sample-plugin/1.2.3")
            .is_dir()
    );
}

#[test]
fn refresh_non_curated_plugin_cache_continues_after_plugin_error() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    fs::create_dir_all(repo_root.join(".agents/plugins")).unwrap();
    write_plugin_with_version(&repo_root, "z-good", "z-good", Some("1.2.3"));
    write_file(
        &repo_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "debug",
  "plugins": [
    {
      "name": "a-broken",
      "source": {
        "source": "local",
        "path": "./missing"
      }
    },
    {
      "name": "z-good",
      "source": {
        "source": "local",
        "path": "./z-good"
      }
    }
  ]
}"#,
    );

    let err = refresh_non_curated_plugin_cache(
        tmp.path(),
        &[AbsolutePathBuf::try_from(repo_root).unwrap()],
        &["a-broken@debug".to_string(), "z-good@debug".to_string()],
    )
    .expect_err("broken plugin should be reported after refreshing the remaining plugins");

    assert!(err.contains("a-broken@debug"));
    assert!(tmp.path().join("plugins/cache/debug/z-good/1.2.3").is_dir());
}
