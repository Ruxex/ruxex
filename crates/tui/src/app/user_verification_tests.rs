//! Covers the controller's RPC boundary, cancellation generations, and proof redaction.

use super::*;
use ruxex_app_server_protocol::McpServerElicitationRequest;
use ruxex_app_server_protocol::McpServerElicitationRequestParams;
use ruxex_app_server_protocol::ServerRequest;
use futures::SinkExt;
use futures::StreamExt;
use pretty_assertions::assert_eq;

fn verification_request() -> ServerRequest {
    ServerRequest::McpServerElicitationRequest {
        request_id: RequestId::Integer(7),
        params: McpServerElicitationRequestParams {
            thread_id: ThreadId::new().to_string(),
            turn_id: None,
            server_name: "deployments".to_string(),
            request: McpServerElicitationRequest::UserVerification {
                title: "Approve deployment?".to_string(),
                description: "Deploy the reviewed change.".to_string(),
                challenge: "AQID".to_string(),
            },
        },
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RpcScenario {
    Success,
    MethodNotFound,
    CancelWhilePending,
    RemoteWorkspace,
}

#[tokio::test]
async fn user_verification_controller_uses_the_local_binary_rpc_and_original_response()
-> color_eyre::Result<()> {
    run_verification_rpc_scenario(RpcScenario::Success).await
}

#[tokio::test]
async fn user_verification_controller_cancels_for_an_older_binary() -> color_eyre::Result<()> {
    run_verification_rpc_scenario(RpcScenario::MethodNotFound).await
}

#[tokio::test]
async fn user_verification_controller_suppresses_a_late_proof_after_cancellation()
-> color_eyre::Result<()> {
    run_verification_rpc_scenario(RpcScenario::CancelWhilePending).await
}

#[tokio::test]
async fn user_verification_remote_workspace_dismisses_the_waiting_prompt() -> color_eyre::Result<()>
{
    run_verification_rpc_scenario(RpcScenario::RemoteWorkspace).await
}
