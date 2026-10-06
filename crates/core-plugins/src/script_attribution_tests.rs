use super::*;
use crate::LoadedPlugin;
use crate::TEST_API_CURATED_MARKETPLACE_NAME;
use crate::TEST_CURATED_MARKETPLACE_NAME;
use crate::store::PluginStore;
use crate::test_support::TEST_CURATED_PLUGIN_CACHE_VERSION;
use crate::test_support::TEST_CURATED_PLUGIN_SHA;
use crate::test_support::curated_plugins_repo_path;
use crate::test_support::write_curated_plugin_sha_with;
use crate::test_support::write_test_api_curated_marketplace;
use crate::test_support::write_test_curated_marketplace;
use genex_plugin::PluginLoadOutcome;
use genex_utils_path_uri::PathUri;
use genex_utils_plugins::SkillDiscoveryMode;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;
const ENABLED: bool = true;
fn path(path: &Path) -> AbsolutePathBuf {
    AbsolutePathBuf::from_absolute_path_checked(path).expect("absolute path")
}
fn loaded_plugin(config_name: &str, root: &Path, enabled: bool) -> LoadedPlugin {
    LoadedPlugin {
        config_name: config_name.to_string(),
        remote_plugin_id: None,
        manifest_name: None,
        plugin_namespace: None,
        manifest_description: None,
        root: path(root),
        enabled,
        skill_roots: Vec::new(),
        skill_discovery_mode: SkillDiscoveryMode::Recursive,
        disabled_skill_paths: HashSet::new(),
        has_enabled_skills: false,
        mcp_servers: HashMap::new(),
        hook_sources: Vec::new(),
        hook_load_warnings: Vec::new(),
        error: None,
    }
}
fn synced_plugin_root(genex_home: &Path, marketplace: &str, plugin_name: &str) -> AbsolutePathBuf {
    let synced_root = curated_plugins_repo_path(genex_home);
    match marketplace {
        TEST_CURATED_MARKETPLACE_NAME => {
            write_test_curated_marketplace(&synced_root, &[plugin_name])
        }
        TEST_API_CURATED_MARKETPLACE_NAME => {
            write_test_api_curated_marketplace(&synced_root, &[plugin_name])
        }
        _ => panic!("unsupported test marketplace"),
    }
    let plugin_id =
        PluginId::new(plugin_name.to_string(), marketplace.to_string()).expect("plugin id");
    let root = PluginStore::new(genex_home.to_path_buf())
        .plugin_root(&plugin_id, TEST_CURATED_PLUGIN_CACHE_VERSION);
    fs::create_dir_all(root.as_path()).expect("create cached plugin root");
    root
}
fn script_fixture() -> (TempDir, AbsolutePathBuf, AbsolutePathBuf) {
    let temp = TempDir::new().expect("temp dir");
    write_curated_plugin_sha_with(temp.path(), TEST_CURATED_PLUGIN_SHA);
    let root = synced_plugin_root(temp.path(), TEST_CURATED_MARKETPLACE_NAME, "sample");
    let script = root.join("scripts/run.py");
    fs::create_dir_all(script.as_path().parent().expect("script parent")).expect("create scripts");
    fs::write(script.as_path(), "#!/usr/bin/env python3\n").expect("write script");
    let script = script.canonicalize().expect("canonical script");
    (temp, root, script)
}

#[test]
fn resolves_primary_runtime_scripts_from_the_installed_plugin_cache() {
    let temp = TempDir::new().expect("temp dir");
    let marketplace_root = temp.path().join("genex-primary-runtime");
    let source_root = marketplace_root.join("plugins/presentations");
    fs::create_dir_all(source_root.join(".genex-plugin")).expect("create manifest directory");
    fs::write(
        source_root.join(".genex-plugin/plugin.json"),
        r#"{"name":"presentations","version":"0.1.29"}"#,
    )
    .expect("write plugin manifest");
    let relative_script_path =
        "skills/presentations/container_tools/mark_artifact_operation_started.mjs";
    let source_script = source_root.join(relative_script_path);
    fs::create_dir_all(source_script.parent().expect("script parent")).expect("create scripts");
    fs::write(&source_script, "#!/usr/bin/env node\n").expect("write script");
    fs::create_dir_all(marketplace_root.join(".agents/plugins"))
        .expect("create marketplace directory");
    fs::write(
        marketplace_root.join(".agents/plugins/marketplace.json"),
        r#"{
  "name": "genex-primary-runtime",
  "plugins": [
    {
      "name": "presentations",
      "source": {
        "source": "local",
        "path": "./plugins/presentations"
      }
    }
  ]
}"#,
    )
    .expect("write marketplace manifest");
    let plugin_id = PluginId::parse("presentations@genex-primary-runtime").expect("plugin id");
    let plugin_root = PluginStore::new(temp.path().to_path_buf())
        .install(path(&source_root), plugin_id.clone())
        .expect("install plugin")
        .installed_path;
    let script = plugin_root.join(relative_script_path);
    let store = PluginStore::new(temp.path().to_path_buf());
    assert_eq!(
        TrustedPluginRoots::expected_plugin_root(
            &store,
            temp.path(),
            &plugin_id,
            Some(&marketplace_root),
        ),
        Some(plugin_root.clone())
    );
    let roots = TrustedPluginRoots {
        roots: vec![TrustedPluginRoot {
            plugin_id: plugin_id.clone(),
            metrics_operations_by_path: BTreeMap::new(),
            root: plugin_root.canonicalize().expect("canonical plugin root"),
        }],
    };

    assert_eq!(
        roots.resolve_attribution(
            &command(&[
                "node",
                script.to_string_lossy().as_ref(),
                "--operation-kind",
                "create",
            ]),
            &path(temp.path()),
        ),
        Some(PluginCommandAttribution {
            plugin_id,
            normalized_relative_path: relative_script_path.to_string(),
        })
    );
}
fn roots_for(genex_home: &Path, plugins: Vec<LoadedPlugin>) -> TrustedPluginRoots {
    TrustedPluginRoots::from_plugin_load_outcome(
        &PluginLoadOutcome::from_plugins(plugins),
        genex_home,
    )
}

#[test]
fn recognizes_windows_executor_plugin_cache_root() {
    let attribution = PluginCommandAttribution {
        plugin_id: PluginId::parse("presentations@genex-primary-runtime").expect("plugin id"),
        normalized_relative_path:
            "skills/presentations/container_tools/mark_artifact_operation_started.mjs".to_string(),
    };
    let script = PathUri::parse(
        "file:///C:/Users/user/.genex/plugins/cache/genex-primary-runtime/presentations/0.1.29/skills/presentations/container_tools/mark_artifact_operation_started.mjs",
    )
    .expect("Windows script URI");

    assert!(executor_plugin_root_matches(&script, &attribution));
}
fn command(parts: &[&str]) -> Vec<String> {
    parts.iter().map(ToString::to_string).collect()
}

#[test]
fn only_emits_safe_normalized_relative_script_paths() {
    assert_eq!(
        normalized_relative_script_path(Path::new("scripts/run.py")),
        Some("scripts/run.py".to_string())
    );
    assert_eq!(
        normalized_relative_script_path(Path::new(
            "/home/user/.genex/plugins/cache/test-curated/sample/scripts/run.py"
        )),
        None
    );
}

#[test]
fn rejects_ambiguous_commands_overlaps_and_symlink_escapes() {
    let (temp, root, script) = script_fixture();
    let roots = roots_for(
        temp.path(),
        vec![loaded_plugin(
            "sample@test-curated",
            root.as_path(),
            ENABLED,
        )],
    );
    let script = script.to_string_lossy().to_string();
    let complex = format!("python {script} && echo done");
    for command in [
        command(&["bash", "-lc", &complex]),
        command(&["node", "--require", "scripts/bootstrap.js", &script]),
        command(&["python", "-m", "scripts.run"]),
        command(&[
            "pwsh.exe",
            "-NoProfile",
            "-Command",
            "scripts/run.py; echo done",
        ]),
        command(&["python", "scripts/missing.py"]),
    ] {
        assert_eq!(roots.resolve_attribution(&command, &root), None);
    }
    let overlapping = TrustedPluginRoots {
        roots: vec![
            TrustedPluginRoot {
                plugin_id: PluginId::parse("sample@test-curated").expect("plugin id"),
                metrics_operations_by_path: BTreeMap::new(),
                root: root.canonicalize().expect("canonical root"),
            },
            TrustedPluginRoot {
                plugin_id: PluginId::parse("nested@test-curated").expect("plugin id"),
                metrics_operations_by_path: BTreeMap::new(),
                root: root.join("scripts").canonicalize().expect("nested root"),
            },
        ],
    };
    assert_eq!(
        overlapping.resolve_attribution(&command(&["scripts/run.py"]), &root),
        None
    );
    #[cfg(unix)]
    {
        let outside = temp.path().join("outside.py");
        fs::write(&outside, "print('outside')\n").expect("write outside script");
        std::os::unix::fs::symlink(&outside, root.join("scripts/escape.py")).expect("symlink");
        assert_eq!(
            roots.resolve_attribution(&command(&["python", "scripts/escape.py"]), &root),
            None
        );

        for unsafe_name in [r"scripts\run.py", "C:run.py"] {
            let unsafe_script = root.join(unsafe_name);
            fs::write(unsafe_script.as_path(), "print('unsafe')\n").expect("write unsafe script");
            assert_eq!(
                roots.resolve_attribution(
                    &command(&["python", &unsafe_script.to_string_lossy()]),
                    &root,
                ),
                None
            );
        }
    }
}
