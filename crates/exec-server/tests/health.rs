#![cfg(unix)]

mod common;

use genex_exec_server::EnvironmentStatus;
use genex_exec_server::EnvironmentStatusKind;
use genex_exec_server::InitializeParams;
use genex_exec_server::InitializeResponse;
use genex_exec_server_protocol::JSONRPCMessage;
use genex_exec_server_protocol::JSONRPCResponse;
use common::exec_server::exec_server;
use pretty_assertions::assert_eq;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_server_reports_environment_status_over_stdio() -> anyhow::Result<()> {
    let mut server = exec_server().await?;
    let initialize_id = server
        .send_request(
            "initialize",
            serde_json::to_value(InitializeParams {
                client_name: "exec-server-health-test".to_string(),
                resume_session_id: None,
            })?,
        )
        .await?;
    let JSONRPCMessage::Response(JSONRPCResponse {
        id,
        result: initialize_result,
    }) = server.next_event().await?
    else {
        panic!("expected initialize response");
    };
    assert_eq!(id, initialize_id);
    let _: InitializeResponse = serde_json::from_value(initialize_result)?;
    server
        .send_notification("initialized", serde_json::json!({}))
        .await?;

    let status_id = server
        .send_request("environment/status", serde_json::json!({}))
        .await?;
    let JSONRPCMessage::Response(JSONRPCResponse { id, result }) = server.next_event().await?
    else {
        panic!("expected environment status response");
    };
    assert_eq!(id, status_id);
    assert_eq!(
        serde_json::from_value::<EnvironmentStatus>(result)?,
        EnvironmentStatus {
            status: EnvironmentStatusKind::Ready,
        }
    );

    server.shutdown().await?;
    Ok(())
}
