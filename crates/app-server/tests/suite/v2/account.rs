use anyhow::Result;
use anyhow::bail;
use app_test_support::TestAppServer;
use app_test_support::to_response;

use app_test_support::ApiKeyAuthFixture;
use app_test_support::AuthFixtureClaims;
use app_test_support::DEFAULT_CLIENT_NAME;
use app_test_support::encode_fixture_token;
use app_test_support::write_api_key_auth;
use app_test_support::write_models_cache;
use chrono::Duration as ChronoDuration;
use chrono::Utc;
use core_test_support::responses;
use genex_app_server_protocol::Account;
use genex_app_server_protocol::AccountLoginCompletedNotification;
use genex_app_server_protocol::AccountUpdatedNotification;
use genex_app_server_protocol::AuthMode;
use genex_app_server_protocol::CancelLoginAccountParams;
use genex_app_server_protocol::CancelLoginAccountResponse;
use genex_app_server_protocol::CancelLoginAccountStatus;
use genex_app_server_protocol::ClientInfo;
use genex_app_server_protocol::DesktopOnboardingEntrypoint;
use genex_app_server_protocol::GetAccountParams;
use genex_app_server_protocol::GetAccountResponse;
use genex_app_server_protocol::InitializeCapabilities;
use genex_app_server_protocol::JSONRPCError;
use genex_app_server_protocol::JSONRPCErrorError;
use genex_app_server_protocol::JSONRPCMessage;
use genex_app_server_protocol::JSONRPCNotification;
use genex_app_server_protocol::JSONRPCResponse;
use genex_app_server_protocol::LoginAccountResponse;
use genex_app_server_protocol::LogoutAccountResponse;
use genex_app_server_protocol::ProviderAuthStatus;
use genex_app_server_protocol::RequestId;
use genex_app_server_protocol::ServerNotification;
use genex_app_server_protocol::ServerRequest;
use genex_app_server_protocol::TurnCompletedNotification;
use genex_app_server_protocol::TurnStatus;
use genex_config::types::AuthCredentialsStoreMode;
use genex_http_client::HttpClientBuilder;
use genex_login::AuthDotJson;
use genex_login::AuthKeyringBackendKind;
use genex_login::CLIENT_ID_OVERRIDE_ENV_VAR;
use genex_login::REFRESH_TOKEN_URL_OVERRIDE_ENV_VAR;
use genex_login::load_auth_dot_json;
use genex_login::login_with_api_key;
use genex_protocol::account::PlanType as AccountPlanType;
use pretty_assertions::assert_eq;
use serde_json::json;
use serial_test::serial;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::Notify;
use tokio::time::timeout;
use url::Url;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const LOGIN_ISSUER_ENV_VAR: &str = "GENEX_APP_SERVER_LOGIN_ISSUER";
const LOGIN_OPEN_APP_URL_ENV_VAR: &str = "GENEX_APP_SERVER_DEV_OPEN_APP_URL";
const WORKSPACE_ID_ALLOWED: &str = "123e4567-e89b-42d3-a456-426614174000";
const WORKSPACE_ID_SECOND_ALLOWED: &str = "123e4567-e89b-42d3-a456-426614174001";
const WORKSPACE_ID_DISALLOWED: &str = "123e4567-e89b-42d3-a456-426614174002";
const WORKSPACE_ID_EMBEDDED: &str = "123e4567-e89b-42d3-a456-426614174010";
const WORKSPACE_ID_INITIAL: &str = "123e4567-e89b-42d3-a456-426614174011";
const WORKSPACE_ID_REFRESHED: &str = "123e4567-e89b-42d3-a456-426614174012";
const WORKSPACE_ID_DEVICE: &str = "123e4567-e89b-42d3-a456-426614174013";
const WORKSPACE_ID_STALE: &str = "123e4567-e89b-42d3-a456-426614174014";

// Helper to create a minimal config.toml for the app server
#[derive(Default)]
struct CreateConfigTomlParams {
    forced_method: Option<String>,
    forced_workspace_id: Option<String>,
    forced_workspace_ids: Option<Vec<String>>,
    base_url: Option<String>,
    managed_auth_base_url: Option<String>,
    model_provider_id: Option<String>,
    extra_provider_config: Option<String>,
}

fn create_config_toml(genex_home: &Path, params: CreateConfigTomlParams) -> std::io::Result<()> {
    let config_toml = genex_home.join("config.toml");
    let base_url = params
        .base_url
        .unwrap_or_else(|| "http://127.0.0.1:0/v1".to_string());
    let forced_line = if let Some(method) = params.forced_method {
        format!("forced_login_method = \"{method}\"\n")
    } else {
        String::new()
    };
    let forced_workspace_line = if let Some(ws) = params.forced_workspace_id {
        format!("forced_managed_workspace_id = \"{ws}\"\n")
    } else if let Some(workspaces) = params.forced_workspace_ids {
        let workspaces = workspaces
            .into_iter()
            .map(|workspace_id| format!("\"{workspace_id}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("forced_managed_workspace_id = [{workspaces}]\n")
    } else {
        String::new()
    };
    let managed_auth_base_url_line = params
        .managed_auth_base_url
        .map(|url| format!("managed_auth_base_url = \"{url}\"\n"))
        .unwrap_or_default();
    let model_provider_id = params
        .model_provider_id
        .unwrap_or_else(|| "mock_provider".to_string());
    let provider_section = if model_provider_id == "mock_provider" {
        format!(
            r#"[model_provider]
id = "mock_provider"
name = "Mock provider for test"
base_url = "{base_url}"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
"#
        )
    } else {
        params.extra_provider_config.unwrap_or_default()
    };
    let contents = format!(
        r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "danger-full-access"
{managed_auth_base_url_line}
{forced_line}
{forced_workspace_line}


[features]
shell_snapshot = false

{provider_section}
"#
    );
    std::fs::write(config_toml, contents)
}

fn read_config_toml(genex_home: &Path) -> Result<toml::Value> {
    Ok(toml::from_str(&std::fs::read_to_string(
        genex_home.join("config.toml"),
    )?)?)
}

fn load_file_auth(genex_home: &Path) -> Result<Option<AuthDotJson>> {
    Ok(load_auth_dot_json(
        genex_home,
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::default(),
    )?)
}

async fn read_account(mcp: &mut TestAppServer) -> Result<GetAccountResponse> {
    let request_id = mcp
        .send_get_account_request(GetAccountParams {
            refresh_token: false,
        })
        .await?;
    timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(request_id)).await?
}

async fn assert_account_updated(
    mcp: &mut TestAppServer,
    auth_mode: Option<AuthMode>,
) -> Result<()> {
    let payload: AccountUpdatedNotification = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_notification("account/updated"),
    )
    .await??;
    assert_eq!(
        payload,
        AccountUpdatedNotification {
            auth_mode,
            plan_type: None,
        }
    );
    Ok(())
}

async fn mock_device_code_usercode(server: &MockServer, interval_seconds: u64) {
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_auth_id": "device-auth-123",
            "user_code": "CODE-12345",
            "interval": interval_seconds.to_string(),
        })))
        .mount(server)
        .await;
}

async fn mock_device_code_usercode_failure(server: &MockServer, status: u16) {
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/usercode"))
        .respond_with(ResponseTemplate::new(status))
        .mount(server)
        .await;
}

async fn mock_device_code_token_success(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_code": "poll-code-321",
            "code_challenge": "code-challenge-321",
            "code_verifier": "code-verifier-321",
        })))
        .mount(server)
        .await;
}

async fn mock_device_code_token_failure(server: &MockServer, status: u16) {
    Mock::given(method("POST"))
        .and(path("/api/accounts/deviceauth/token"))
        .respond_with(ResponseTemplate::new(status))
        .mount(server)
        .await;
}

async fn mock_oauth_token(server: &MockServer, id_token: &str) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id_token": id_token,
            "access_token": "access-token-123",
            "refresh_token": "refresh-token-123",
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn logout_account_removes_auth_and_notifies() -> Result<()> {
    let genex_home = TempDir::new()?;
    create_config_toml(genex_home.path(), CreateConfigTomlParams::default())?;

    login_with_api_key(
        genex_home.path(),
        "sk-test-key",
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::default(),
    )?;
    assert!(genex_home.path().join("auth.json").exists());

    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .with_env_overrides(&[("PROVIDER_API_KEY", None)])
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let id = mcp.send_logout_account_request().await?;
    let _ok: LogoutAccountResponse = timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(id)).await??;

    let note = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("account/updated"),
    )
    .await??;
    let parsed: ServerNotification = note.try_into()?;
    let ServerNotification::AccountUpdated(payload) = parsed else {
        bail!("unexpected notification: {parsed:?}");
    };
    assert!(
        payload.auth_mode.is_none(),
        "auth_method should be None after logout"
    );
    assert_eq!(payload.plan_type, None);

    assert!(
        !genex_home.path().join("auth.json").exists(),
        "auth.json should be deleted"
    );

    let get_id = mcp
        .send_get_account_request(GetAccountParams {
            refresh_token: false,
        })
        .await?;
    let account: GetAccountResponse =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(get_id)).await??;
    assert_eq!(account.account, None);
    Ok(())
}

#[tokio::test]
async fn logout_account_succeeds_when_config_reload_fails() -> Result<()> {
    let genex_home = TempDir::new()?;
    create_config_toml(genex_home.path(), CreateConfigTomlParams::default())?;
    login_with_api_key(
        genex_home.path(),
        "sk-test-key",
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::default(),
    )?;

    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .with_env_overrides(&[("PROVIDER_API_KEY", None)])
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    std::fs::write(genex_home.path().join("config.toml"), "invalid = [")?;

    let request_id = mcp.send_logout_account_request().await?;
    let response = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_eq!(
        to_response::<LogoutAccountResponse>(response)?,
        LogoutAccountResponse {}
    );
    assert_eq!(load_file_auth(genex_home.path())?, None);
    assert_account_updated(&mut mcp, /*auth_mode*/ None).await?;

    Ok(())
}

#[tokio::test]
async fn startup_enforces_local_auth_requirements_before_cloud_fetch() -> Result<()> {
    let genex_home = TempDir::new()?;
    let mock_server = MockServer::start().await;
    create_config_toml(
        genex_home.path(),
        CreateConfigTomlParams {
            managed_auth_base_url: Some(format!("{}/backend-api", mock_server.uri())),
            ..Default::default()
        },
    )?;
    std::fs::write(
        genex_home.path().join("requirements.toml"),
        "allowed_login_methods = [\"api\"]\n",
    )?;
    write_api_key_auth(
        genex_home.path(),
        ApiKeyAuthFixture::new("test-token")
            .plan_type("enterprise")
            .user_id("user-123")
            .workspace_id("account-123")
            .account_id("account-123"),
        AuthCredentialsStoreMode::File,
    )?;

    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    assert!(
        mock_server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "disallowed managed auth must not fetch cloud requirements"
    );

    assert_eq!(read_account(&mut mcp).await?.account, None);

    Ok(())
}
async fn mount_disabled_attribution_settings(mock_server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/backend-api/wham/settings/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "commit_attribution_enabled": false,
        })))
        .mount(mock_server)
        .await;
}

#[tokio::test]
// 401 response triggers account/managedAuthTokens/refresh and retries with new tokens.
#[tokio::test]
// Client returns JSON-RPC error to refresh; turn fails.
#[tokio::test]
// Refresh returns tokens for the wrong workspace; turn fails.
#[tokio::test]
// Refresh returns a malformed access token; turn fails.
#[tokio::test]
async fn login_account_api_key_succeeds_and_notifies() -> Result<()> {
    let genex_home = TempDir::new()?;
    create_config_toml(genex_home.path(), CreateConfigTomlParams::default())?;

    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let req_id = mcp
        .send_login_account_api_key_request("sk-test-key")
        .await?;
    let login: LoginAccountResponse =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(req_id)).await??;
    assert_eq!(login, LoginAccountResponse::ApiKey {});

    let note = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("account/login/completed"),
    )
    .await??;
    let parsed: ServerNotification = note.try_into()?;
    let ServerNotification::AccountLoginCompleted(payload) = parsed else {
        bail!("unexpected notification: {parsed:?}");
    };
    pretty_assertions::assert_eq!(payload.login_id, None);
    pretty_assertions::assert_eq!(payload.success, true);
    pretty_assertions::assert_eq!(payload.error, None);

    let note = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_notification_message("account/updated"),
    )
    .await??;
    let parsed: ServerNotification = note.try_into()?;
    let ServerNotification::AccountUpdated(payload) = parsed else {
        bail!("unexpected notification: {parsed:?}");
    };
    pretty_assertions::assert_eq!(payload.auth_mode, Some(AuthMode::ApiKey));
    pretty_assertions::assert_eq!(payload.plan_type, None);

    assert!(genex_home.path().join("auth.json").exists());
    Ok(())
}

#[derive(Clone, Copy)]
enum LoginRefreshTrigger {
    AuthStatus,
    UnauthorizedConfig,
}

#[tokio::test]
async fn get_account_no_auth() -> Result<()> {
    let genex_home = TempDir::new()?;
    create_config_toml(
        genex_home.path(),
        CreateConfigTomlParams {
            ..Default::default()
        },
    )?;

    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .with_env_overrides(&[("PROVIDER_API_KEY", None)])
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let params = GetAccountParams {
        refresh_token: false,
    };
    let request_id = mcp.send_get_account_request(params).await?;

    let account: GetAccountResponse =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(request_id)).await??;

    assert_eq!(account.account, None, "expected no account");
    Ok(())
}

#[tokio::test]
async fn get_account_with_api_key() -> Result<()> {
    let genex_home = TempDir::new()?;
    create_config_toml(
        genex_home.path(),
        CreateConfigTomlParams {
            ..Default::default()
        },
    )?;

    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    let req_id = mcp
        .send_login_account_api_key_request("sk-test-key")
        .await?;
    let _login_ok: LoginAccountResponse =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(req_id)).await??;

    let params = GetAccountParams {
        refresh_token: false,
    };
    let request_id = mcp.send_get_account_request(params).await?;

    let received: GetAccountResponse =
        timeout(DEFAULT_READ_TIMEOUT, mcp.read_response(request_id)).await??;

    let expected = GetAccountResponse {
        provider_id: "mock_provider".to_string(),
        account: Some(Account::ApiKey {}),
        account_id: None,
        display_name: None,
        auth_status: ProviderAuthStatus::Authenticated,
        usage: None,
    };
    assert_eq!(received, expected);
    Ok(())
}

#[tokio::test]
async fn account_reads_use_startup_config_when_config_reload_fails() -> Result<()> {
    let genex_home = TempDir::new()?;
    create_config_toml(genex_home.path(), CreateConfigTomlParams::default())?;

    let mut mcp = TestAppServer::builder()
        .with_genex_home(genex_home.path())
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_READ_TIMEOUT)
        .await?;

    std::fs::write(genex_home.path().join("config.toml"), "invalid = [")?;

    assert_eq!(
        read_account(&mut mcp).await?,
        GetAccountResponse {
            provider_id: "mock_provider".to_string(),
            account: None,
            account_id: None,
            display_name: None,
            auth_status: ProviderAuthStatus::Unauthenticated,
            usage: None,
        }
    );

    Ok(())
}
