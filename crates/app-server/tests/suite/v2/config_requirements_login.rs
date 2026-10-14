//! Requirements reads report the authentication policy enforced by the running server.

use anyhow::Result;
use app_test_support::TestAppServer;
use ruxex_app_server_protocol::Account;
use ruxex_app_server_protocol::GetAccountParams;
use ruxex_app_server_protocol::GetAccountResponse;
use ruxex_app_server_protocol::RequestId;
use ruxex_protocol::config_types::ForcedLoginMethod;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::time::Duration;
use tempfile::TempDir;
use test_case::test_case;
use tokio::time::timeout;
use wiremock::MockServer;

const READ_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 60);

async fn start_server(
    config: &str,
    requirements: Option<&str>,
) -> Result<(TempDir, TestAppServer)> {
    let home = TempDir::new()?;
    std::fs::write(home.path().join("config.toml"), config)?;
    if let Some(requirements) = requirements {
        std::fs::write(home.path().join("requirements.toml"), requirements)?;
    }
    let server = TestAppServer::builder()
        .with_ruxex_home(home.path())
        .build_initialized_with_timeout(READ_TIMEOUT)
        .await?;
    Ok((home, server))
}

async fn read_requirements(server: &mut TestAppServer) -> Result<Value> {
    let id = server.send_config_requirements_read_request().await?;
    timeout(READ_TIMEOUT, server.read_response(id)).await?
}

#[test_case(""; "no_requirements")]
#[test_case("forced_managed_workspace_id = []"; "empty_forced_workspaces_are_unrestricted")]
#[test_case("forced_managed_workspace_id = ['managed']"; "forced_workspace_does_not_exclude_api")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_requirements_read_preserves_unrestricted_default(config: &str) -> Result<()> {
    let (_home, mut server) = start_server(config, /*requirements*/ None).await?;
    assert_eq!(
        read_requirements(&mut server).await?,
        json!({"requirements": null})
    );
    Ok(())
}





#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_requirements_read_rejects_invalid_login_method() -> Result<()> {
    let (home, mut server) = start_server("", /*requirements*/ None).await?;
    std::fs::write(
        home.path().join("requirements.toml"),
        "allowed_login_methods = ['saml']",
    )?;
    let id = server.send_config_requirements_read_request().await?;
    let error = timeout(
        READ_TIMEOUT,
        server.read_stream_until_error_message(RequestId::Integer(id)),
    )
    .await??;
    assert!(
        error.error.message.contains("allowed_login_methods"),
        "{error:?}"
    );
    Ok(())
}
