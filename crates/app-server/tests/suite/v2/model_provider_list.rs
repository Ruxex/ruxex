use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use pretty_assertions::assert_eq;
use serde_json::Value;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[tokio::test]
async fn lists_the_mounted_provider_and_auth_state() -> Result<()> {
    let genex_home = TempDir::new()?;
    MockResponsesConfig::new("http://127.0.0.1:1/v1")
        .with_model_provider("provider_a")
        .with_provider_name("Provider A")
        .write(genex_home.path())?;

    let mut app_server = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_TIMEOUT)
        .await?;

    let request_id = app_server
        .send_request("modelProvider/list", Some(serde_json::json!({})))
        .await?;
    let received: Value = timeout(DEFAULT_TIMEOUT, app_server.read_response(request_id)).await??;

    let providers = received["providers"]
        .as_array()
        .expect("providers should be an array");
    assert_eq!(providers.len(), 1);
    let provider_a = &providers[0];
    assert_eq!(provider_a["id"], "provider_a");
    assert_eq!(provider_a["name"], "Provider A");
    assert_eq!(provider_a["authStatus"], "unauthenticated");
    Ok(())
}
