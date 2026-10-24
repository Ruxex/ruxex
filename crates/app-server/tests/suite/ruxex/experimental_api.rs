use anyhow::Result;
use app_test_support::DEFAULT_CLIENT_NAME;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_sequence_unchecked;
use app_test_support::to_response;
use ruxex_app_server_protocol::AskForApproval;
use ruxex_app_server_protocol::ClientInfo;
use ruxex_app_server_protocol::InitializeCapabilities;
use ruxex_app_server_protocol::JSONRPCError;
use ruxex_app_server_protocol::JSONRPCMessage;
use ruxex_app_server_protocol::JSONRPCResponse;
use ruxex_app_server_protocol::RequestId;
use ruxex_app_server_protocol::ThreadStartParams;
use ruxex_app_server_protocol::ThreadStartResponse;
use pretty_assertions::assert_eq;
use std::time::Duration;
use tempfile::TempDir;
use test_case::test_case;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[test_case("thread/settings/update"; "future settings")]
#[test_case("turn/settings/update"; "turn settings")]
#[tokio::test]
async fn settings_update_requires_experimental_api_capability(method: &str) -> Result<()> {
    let ruxex_home = TempDir::new()?;
    let mut mcp = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .without_auto_env()
        .build()
        .await?;

    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                extensions: None,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let mut params = serde_json::json!({"threadId": "thr_123"});
    if method == "turn/settings/update" {
        params["turnId"] = serde_json::json!("turn_123");
    }
    let request_id = mcp.send_request(method, Some(params)).await?;
    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, method);
    Ok(())
}

#[tokio::test]
async fn thread_start_mock_field_requires_experimental_api_capability() -> Result<()> {
    let server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let ruxex_home = TempDir::new()?;
    MockResponsesConfig::new(&server.uri()).write(ruxex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .build()
        .await?;
    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                extensions: None,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_thread_start_request(ThreadStartParams {
            mock_experimental_field: Some("mock".to_string()),
            ..Default::default()
        })
        .await?;

    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "thread/start.mockExperimentalField");
    Ok(())
}

#[tokio::test]
async fn thread_start_without_dynamic_tools_allows_without_experimental_api_capability()
-> Result<()> {
    let server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let ruxex_home = TempDir::new()?;
    MockResponsesConfig::new(&server.uri()).write(ruxex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .build()
        .await?;
    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                extensions: None,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_thread_start_request(ThreadStartParams {
            model: Some("mock-model".to_string()),
            ..Default::default()
        })
        .await?;
    let response: JSONRPCResponse = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    let _: ThreadStartResponse = to_response(response)?;
    Ok(())
}

#[tokio::test]
async fn thread_start_granular_approval_policy_requires_experimental_api_capability() -> Result<()>
{
    let server = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let ruxex_home = TempDir::new()?;
    MockResponsesConfig::new(&server.uri()).write(ruxex_home.path())?;

    let mut mcp = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .build()
        .await?;
    let init = mcp
        .initialize_with_capabilities(
            default_client_info(),
            Some(InitializeCapabilities {
                experimental_api: false,
                request_attestation: false,
                opt_out_notification_methods: None,
                extensions: None,
            }),
        )
        .await?;
    let JSONRPCMessage::Response(_) = init else {
        anyhow::bail!("expected initialize response, got {init:?}");
    };

    let request_id = mcp
        .send_thread_start_request(ThreadStartParams {
            approval_policy: Some(AskForApproval::Granular {
                sandbox_approval: true,
                rules: false,
                skill_approval: false,
                request_permissions: true,
                mcp_elicitations: false,
            }),
            ..Default::default()
        })
        .await?;

    let error = timeout(
        DEFAULT_TIMEOUT,
        mcp.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_experimental_capability_error(error, "askForApproval.granular");
    Ok(())
}

fn default_client_info() -> ClientInfo {
    ClientInfo {
        name: DEFAULT_CLIENT_NAME.to_string(),
        title: None,
        version: "0.1.0".to_string(),
    }
}

fn assert_experimental_capability_error(error: JSONRPCError, reason: &str) {
    assert_eq!(error.error.code, -32600);
    assert_eq!(
        error.error.message,
        format!("{reason} requires experimentalApi capability")
    );
    assert_eq!(error.error.data, None);
}
