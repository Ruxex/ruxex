use super::*;
use pretty_assertions::assert_eq;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_absolute_path::AbsolutePathBufGuard;
use std::num::NonZeroU64;
use tempfile::tempdir;

#[test]
fn test_api_provider_applies_current_managed_residency() {
    let info = ModelProviderInfo {
        http_headers: Some(maplit::hashmap! {
            "X-Codex-Residency".to_string() => "eu".into(),
            "x-provider-header".to_string() => "preserved".into(),
        }),
        ..ModelProviderInfo::create_responses_provider(/*base_url*/ None)
    };
    let original_info = info.clone();
    let previous_requirement = read_managed_residency_requirement();
    set_managed_residency_requirement(Some(ResidencyRequirement::Us));
    let managed = info.to_api_provider(/*auth_mode*/ None);
    set_managed_residency_requirement(/*enforce_residency*/ None);
    let unmanaged = info.to_api_provider(/*auth_mode*/ None);
    set_managed_residency_requirement(previous_requirement);

    assert_eq!(
        managed.expect("managed provider should resolve").headers,
        HeaderMap::from_iter([
            (
                HeaderName::from_static(RESIDENCY_HEADER_NAME),
                HeaderValue::from_static("us")
            ),
            (
                HeaderName::from_static("x-provider-header"),
                HeaderValue::from_static("preserved")
            ),
        ])
    );
    assert_eq!(
        unmanaged
            .expect("unmanaged provider should resolve")
            .headers,
        HeaderMap::from_iter([
            (
                HeaderName::from_static(RESIDENCY_HEADER_NAME),
                HeaderValue::from_static("eu")
            ),
            (
                HeaderName::from_static("x-provider-header"),
                HeaderValue::from_static("preserved")
            ),
        ])
    );
    assert_eq!(info, original_info);
}

#[test]
fn test_deserialize_ollama_model_provider_toml() {
    let azure_provider_toml = r#"
name = "Ollama"
base_url = "http://localhost:11434/v1"
        "#;
    let expected_provider = ModelProviderInfo {
        id: None,
        name: "Ollama".into(),
        base_url: Some("http://localhost:11434/v1".into()),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        supports_websockets: false,
        supports_standalone_web_search: false,
        supports_tool_strict_mode: false,
    };

    let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
    assert_eq!(expected_provider, provider);
}

#[test]
fn test_deserialize_azure_model_provider_toml() {
    let azure_provider_toml = r#"
name = "Azure"
base_url = "https://xxxxx.provider.azure.com/provider"
env_key = "AZURE_PROVIDER_API_KEY"
query_params = { api-version = "2025-04-01-preview" }
        "#;
    let expected_provider = ModelProviderInfo {
        id: None,
        name: "Azure".into(),
        base_url: Some("https://xxxxx.provider.azure.com/provider".into()),
        env_key: Some("AZURE_PROVIDER_API_KEY".into()),
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        wire_api: WireApi::Responses,
        query_params: Some(maplit::hashmap! {
            "api-version".to_string() => "2025-04-01-preview".into(),
        }),
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        supports_websockets: false,
        supports_standalone_web_search: false,
        supports_tool_strict_mode: false,
    };

    let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
    assert_eq!(expected_provider, provider);
}

#[test]
fn test_deserialize_example_model_provider_toml() {
    let azure_provider_toml = r#"
name = "Example"
base_url = "https://example.com"
env_key = "API_KEY"
http_headers = { "X-Example-Header" = "example-value" }
env_http_headers = { "X-Example-Env-Header" = "EXAMPLE_ENV_VAR" }
supports_standalone_web_search = true
        "#;
    let expected_provider = ModelProviderInfo {
        id: None,
        name: "Example".into(),
        base_url: Some("https://example.com".into()),
        env_key: Some("API_KEY".into()),
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: Some(maplit::hashmap! {
            "X-Example-Header".to_string() => "example-value".into(),
        }),
        env_http_headers: Some(maplit::hashmap! {
            "X-Example-Env-Header".to_string() => "EXAMPLE_ENV_VAR".to_string(),
        }),
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        supports_websockets: false,
        supports_standalone_web_search: true,
        supports_tool_strict_mode: false,
    };

    let provider: ModelProviderInfo = toml::from_str(azure_provider_toml).unwrap();
    assert_eq!(expected_provider, provider);
}

#[test]
fn test_deserialize_chat_wire_api() {
    let provider_toml = r#"
name = "Example provider using Chat Completions"
base_url = "https://api.example.test/v1"
env_key = "PROVIDER_API_KEY"
wire_api = "chat"
        "#;

    let provider = toml::from_str::<ModelProviderInfo>(provider_toml).unwrap();
    assert_eq!(provider.wire_api, WireApi::Chat);
}

#[test]
fn test_deserialize_tool_strict_mode_support() {
    let provider = toml::from_str::<ModelProviderInfo>(
        r#"
name = "Strict provider"
supports_tool_strict_mode = true
"#,
    )
    .expect("provider config should deserialize");

    assert!(provider.supports_tool_strict_mode);
}

#[test]
fn test_deserialize_websocket_connect_timeout() {
    let provider_toml = r#"
name = "Example provider"
base_url = "https://api.example.test/v1"
websocket_connect_timeout_ms = 15000
supports_websockets = true
        "#;

    let provider: ModelProviderInfo = toml::from_str(provider_toml).unwrap();
    assert_eq!(provider.websocket_connect_timeout_ms, Some(15_000));
}

#[test]
fn test_deserialize_provider_auth_config_defaults() {
    let base_dir = tempdir().unwrap();
    let provider_toml = r#"
name = "Corp"

[auth]
command = "./scripts/print-token"
args = ["--format=text"]
        "#;

    let provider: ModelProviderInfo = {
        let _guard = AbsolutePathBufGuard::new(base_dir.path());
        toml::from_str(provider_toml).unwrap()
    };

    assert_eq!(
        provider.auth,
        Some(ModelProviderAuthInfo {
            command: "./scripts/print-token".to_string(),
            args: vec!["--format=text".into()],
            timeout_ms: NonZeroU64::new(5_000).unwrap(),
            refresh_interval_ms: 300_000,
            cwd: AbsolutePathBuf::resolve_path_against_base(".", base_dir.path()),
        })
    );
}

#[test]
fn test_resolve_model_provider_returns_one_custom_provider() {
    let custom_provider = ModelProviderInfo {
        id: Some("custom".to_string()),
        name: "Custom".to_string(),
        base_url: Some("https://example.com/v1".to_string()),
        ..ModelProviderInfo::default()
    };
    let resolved = resolve_model_provider(Some(custom_provider.clone()))
        .expect("custom provider should resolve");

    assert_eq!(resolved.id, "custom");
    assert_eq!(resolved.info, custom_provider);
}

#[test]
fn test_resolve_model_provider_uses_responses_default() {
    let resolved = resolve_model_provider(None).expect("built-in provider should resolve");
    let mut expected = ModelProviderInfo::create_responses_provider(None);
    expected.id = Some("responses".to_string());

    assert_eq!(resolved.id, "responses");
    assert_eq!(resolved.info.id.as_deref(), Some("responses"));
    assert_eq!(resolved.info, expected);
}

#[test]
fn test_resolve_model_provider_rejects_unknown_provider() {
    let error = resolve_model_provider(Some(ModelProviderInfo {
        id: Some("missing".to_string()),
        ..ModelProviderInfo::default()
    }))
    .expect_err("empty provider should fail");

    assert_eq!(error, "model_provider.name must not be empty");
}

#[test]
fn test_deserialize_provider_auth_config_allows_zero_refresh_interval() {
    let base_dir = tempdir().unwrap();
    let provider_toml = r#"
name = "Corp"

[auth]
command = "./scripts/print-token"
refresh_interval_ms = 0
        "#;

    let provider: ModelProviderInfo = {
        let _guard = AbsolutePathBufGuard::new(base_dir.path());
        toml::from_str(provider_toml).unwrap()
    };

    let auth = provider.auth.expect("auth config should deserialize");
    assert_eq!(auth.refresh_interval_ms, 0);
    assert_eq!(auth.refresh_interval(), None);
}
