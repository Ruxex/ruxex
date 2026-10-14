use super::EffectiveMcpServer;
use super::McpServerConnectionIdentity;
use super::referenced_environment_variables;
use crate::runtime::McpRuntimeContext;
use ruxex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID;
use ruxex_config::McpServerConfig;
use ruxex_config::types::AuthKeyringBackendKind;
use ruxex_config::types::OAuthCredentialsStoreMode;
use ruxex_exec_server::Environment;
use ruxex_exec_server_test_support::environment_manager_without_environments;
use ruxex_protocol::mcp::ClientMcpExtensions;
use ruxex_rmcp_client::McpOAuthRefreshMode;
use ruxex_utils_path_uri::PathUri;
use pretty_assertions::assert_eq;
use rmcp::model::ElicitationCapability;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn remote_http_connections_track_host_headers_but_not_executor_bearer_tokens() {
    let mut config: McpServerConfig = serde_json::from_value(serde_json::json!({
        "url": "https://example.com/mcp",
        "environment_id": "executor-1",
        "bearer_token_env_var": "NODE_REPL_AUTH_TOKEN",
        "env_http_headers": {"X-Api-Key": "PATH"},
    }))
    .expect("remote MCP configuration should deserialize");

    assert_eq!(
        referenced_environment_variables(&config),
        vec![("PATH".to_string(), std::env::var_os("PATH"))],
    );

    let remote_host_bearer: McpServerConfig = serde_json::from_value(serde_json::json!({
        "url": "https://example.com/mcp",
        "environment_id": "executor-1",
        "bearer_token_env_var": "PATH",
    }))
    .expect("host-resolved remote MCP configuration should deserialize");
    assert_eq!(
        referenced_environment_variables(&remote_host_bearer),
        vec![("PATH".to_string(), std::env::var_os("PATH"))],
    );

    config.environment_id = DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string();
    assert_eq!(
        referenced_environment_variables(&config),
        vec![
            (
                "NODE_REPL_AUTH_TOKEN".to_string(),
                std::env::var_os("NODE_REPL_AUTH_TOKEN"),
            ),
            ("PATH".to_string(), std::env::var_os("PATH")),
        ],
    );
}
#[test]
fn connection_identity_constructor_accepts_only_mcp_owned_inputs() {
    let _constructor: fn(
        &str,
        &EffectiveMcpServer,
        Option<&PathUri>,
        OAuthCredentialsStoreMode,
        AuthKeyringBackendKind,
        McpOAuthRefreshMode,
        &Result<Option<Arc<Environment>>, String>,
        &McpRuntimeContext,
        ElicitationCapability,
        ClientMcpExtensions,
        Option<&McpServerConnectionIdentity>,
    ) -> McpServerConnectionIdentity = McpServerConnectionIdentity::new;
}

fn connection_identity_for_config(value: serde_json::Value) -> McpServerConnectionIdentity {
    let config: McpServerConfig = serde_json::from_value(value).expect("valid MCP config");
    let server = EffectiveMcpServer::configured(config);
    let runtime_context = McpRuntimeContext::new(
        Arc::new(environment_manager_without_environments()),
        PathBuf::from("/tmp"),
    );
    let resolved_environment = Ok(None);
    McpServerConnectionIdentity::new(
        "test-server",
        &server,
        None,
        OAuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::default(),
        McpOAuthRefreshMode::Legacy,
        &resolved_environment,
        &runtime_context,
        ElicitationCapability::default(),
        ClientMcpExtensions::default(),
        None,
    )
}

#[test]
fn connection_identity_depends_on_mcp_owned_configuration() {
    let first = connection_identity_for_config(serde_json::json!({
        "url": "https://example.test/mcp",
        "http_headers": {"X-MCP-Key": "one"},
    }));
    let same = connection_identity_for_config(serde_json::json!({
        "url": "https://example.test/mcp",
        "http_headers": {"X-MCP-Key": "one"},
    }));
    let changed = connection_identity_for_config(serde_json::json!({
        "url": "https://example.test/mcp",
        "http_headers": {"X-MCP-Key": "two"},
    }));

    assert!(first.has_same_connection_config(&same));
    assert!(!first.has_same_connection_config(&changed));
}
