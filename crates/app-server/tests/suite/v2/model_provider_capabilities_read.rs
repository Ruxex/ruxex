use std::time::Duration;

use anyhow::Result;
use app_test_support::TestAppServer;
use genex_app_server_protocol::ModelProviderCapabilitiesReadParams;
use genex_app_server_protocol::ModelProviderCapabilitiesReadResponse;
use genex_app_server_protocol::RemoteCompactionSupport;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn read_default_provider_capabilities() -> Result<()> {
    let genex_home = TempDir::new()?;
    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_TIMEOUT)
        .await?;

    let request_id = mcp
        .send_model_provider_capabilities_read_request(ModelProviderCapabilitiesReadParams {})
        .await?;
    let received: ModelProviderCapabilitiesReadResponse =
        timeout(DEFAULT_TIMEOUT, mcp.read_response(request_id)).await??;

    let expected = ModelProviderCapabilitiesReadResponse {
        model_listing: true,
        api_key_model_discovery: false,
        command_auth: false,
        streaming: true,
        websocket: false,
        usage: false,
        interactive_login: false,
        namespace_tools: true,
        web_search: true,
        standalone_web_search: false,
        external_web_access: true,
        history_notes: true,
        remote_compaction: RemoteCompactionSupport::V2,
    };
    assert_eq!(received, expected);
    Ok(())
}
