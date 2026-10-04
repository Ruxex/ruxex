use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use app_test_support::ApiKeyAuthFixture;
use app_test_support::write_api_key_auth;
use genex_config::ConfigLoadOptions;
use genex_config::types::AuthCredentialsStoreMode;
use genex_core::config::load_config_toml_with_layer_stack;
use genex_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

const CLOUD_MANAGED_PERMISSION_PROFILE_REQUIREMENTS: &str = r#"
default_permissions = "managed-cloud"

[allowed_permission_profiles]
managed-cloud = true

[permissions.managed-cloud]
extends = ":workspace"

[permissions.managed-cloud.network]
enabled = true
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sandbox_fetches_and_enforces_cloud_managed_permission_profile() -> Result<()> {
    let server = MockServer::start().await;
    let managed_auth_base_url = format!("{}/backend-api", server.uri());
    let expected_requirements = json!([{
        "id": "req-managed-cloud",
        "name": "Managed permissions",
        "contents": CLOUD_MANAGED_PERMISSION_PROFILE_REQUIREMENTS,
    }]);

    let genex_home = TempDir::new()?;
    std::fs::write(
        genex_home.path().join("config.toml"),
        format!(
            "cli_auth_credentials_store = \"file\"\nmanaged_auth_base_url = \"{managed_auth_base_url}\"\n",
        ),
    )?;
    let bootstrap_config = load_config_toml_with_layer_stack(
        genex_home.path(),
        Some(&AbsolutePathBuf::from_absolute_path(genex_home.path())?),
        vec![
            (
                "cli_auth_credentials_store".to_string(),
                toml::Value::String("file".to_string()),
            ),
            (
                "managed_auth_base_url".to_string(),
                toml::Value::String(managed_auth_base_url.clone()),
            ),
        ],
        ConfigLoadOptions::default(),
    )
    .await?;
    if bootstrap_config.config_toml.cli_auth_credentials_store
        != Some(AuthCredentialsStoreMode::File)
        || bootstrap_config
            .config_toml
            .managed_auth_base_url
            .as_deref()
            != Some(managed_auth_base_url.as_str())
    {
        eprintln!(
            "skipping cloud-managed sandbox subprocess: host-managed authentication or backend routing prevents isolated mock credentials"
        );
        return Ok(());
    }

    write_api_key_auth(
        genex_home.path(),
        ApiKeyAuthFixture::new("test-token")
            .account_id("workspace-123")
            .workspace_id("workspace-123")
            .user_id("user-123")
            .plan_type("enterprise"),
        AuthCredentialsStoreMode::File,
    )?;
    Mock::given(method("GET"))
        .and(path("/backend-api/wham/config/bundle"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "requirements_toml": {
                "enterprise_managed": expected_requirements.clone(),
            },
        })))
        .expect(1)
        .mount(&server)
        .await;

    let genex = genex_utils_cargo_bin::cargo_bin("genex")?;
    let managed_auth_base_url_override =
        format!("managed_auth_base_url=\"{managed_auth_base_url}\"");
    let output = Command::new(&genex)
        .current_dir(genex_home.path())
        .env("GENEX_HOME", genex_home.path())
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .env_remove("GENEX_ACCESS_TOKEN")
        .env_remove("PROVIDER_API_KEY")
        .args(["-c", "cli_auth_credentials_store=\"file\""])
        .args(["-c", managed_auth_base_url_override.as_str()])
        .args([
            "sandbox",
            "-P",
            "managed-cloud",
            "--include-managed-config",
            "--",
        ])
        .arg(&genex)
        .arg("--version")
        .output()?;
    let cloud_bundle_request_paths: Vec<_> = server
        .received_requests()
        .await
        .context("failed to read mock cloud configuration requests")?
        .into_iter()
        .map(|request| request.url.path().to_string())
        .collect();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let nested_macos_sandbox_unavailable = cfg!(target_os = "macos")
        && output.status.code() == Some(71)
        && stderr.contains("sandbox-exec: sandbox_apply: Operation not permitted");

    assert!(
        output.status.success() || nested_macos_sandbox_unavailable,
        "cloud-managed sandbox profile was not enforced: status={:?}; stdout={}; stderr={}; cloud bundle requests={cloud_bundle_request_paths:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        stderr,
    );
    if !nested_macos_sandbox_unavailable {
        assert!(
            String::from_utf8(output.stdout)?.starts_with("genex"),
            "expected the sandboxed Genex version command to run",
        );
    }

    let cache: Value = serde_json::from_slice(&std::fs::read(
        genex_home.path().join("cloud-config-bundle-cache.json"),
    )?)?;
    assert_eq!(
        json!({
            "user_id": cache["signed_payload"]["user_id"],
            "account_id": cache["signed_payload"]["account_id"],
            "requirements_toml": cache["signed_payload"]["bundle"]["requirements_toml"],
        }),
        json!({
            "user_id": "user-123",
            "account_id": "workspace-123",
            "requirements_toml": {
                "enterprise_managed": expected_requirements,
            },
        }),
    );
    server.verify().await;

    Ok(())
}
