use super::*;
use tempfile::TempDir;

#[test]
fn mock_responses_config_composes_model_provider_features_and_extra_tables() {
    let home = TempDir::new().expect("temporary GENEX_HOME");
    MockResponsesConfig::new("http://127.0.0.1:1234")
        .with_model("custom-model")
        .with_model_provider("provider-custom")
        .with_provider_name("provider")
        .with_provider_base_url("http://127.0.0.1:1234/api/genex")
        .with_approval_policy("on-request")
        .with_sandbox_mode("workspace-write")
        .enable_feature(Feature::NetworkProxy)
        .disable_feature(Feature::ShellSnapshot)
        .with_root_config("managed_auth_base_url = \"http://127.0.0.1:1234\"")
        .with_provider_config("requires_auth = true")
        .with_extra_config("[extra]\nenabled = true")
        .write(home.path())
        .expect("write composable mock Responses config");

    let config =
        std::fs::read_to_string(home.path().join("config.toml")).expect("read config.toml");
    for expected in [
        "model = \"custom-model\"",
        "approval_policy = \"on-request\"",
        "sandbox_mode = \"workspace-write\"",
        "managed_auth_base_url = \"http://127.0.0.1:1234\"",
        "model_provider = \"provider-custom\"",
        "shell_snapshot = false",
        "network_proxy = true",
        "[model_providers.provider-custom]\nname = \"provider\"",
        "base_url = \"http://127.0.0.1:1234/api/genex\"",
        "requires_auth = true",
        "[extra]\nenabled = true",
    ] {
        assert!(config.contains(expected), "config is missing {expected}");
    }
}

#[test]
fn mock_responses_writer_preserves_provider_auth_and_feature_overrides() {
    let home = TempDir::new().expect("temporary GENEX_HOME");
    write_mock_responses_config_toml(
        home.path(),
        "http://127.0.0.1:1234",
        &BTreeMap::from([
            (Feature::NetworkProxy, true),
            (Feature::ShellSnapshot, false),
        ]),
        /*auto_compact_limit*/ 321,
        Some(true),
        "provider",
        "compact this",
    )
    .expect("write mock Responses config");

    let config =
        std::fs::read_to_string(home.path().join("config.toml")).expect("read config.toml");
    for expected in [
        "compact_prompt = \"compact this\"",
        "model_auto_compact_token_limit = 321",
        "shell_snapshot = false",
        "network_proxy = true",
        "[model_providers.provider]\nname = \"provider\"",
        "supports_websockets = false",
        "requires_auth = true",
    ] {
        assert!(config.contains(expected), "config is missing {expected}");
    }
}
