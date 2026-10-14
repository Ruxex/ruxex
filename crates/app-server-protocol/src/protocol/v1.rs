use std::collections::HashMap;
use std::path::PathBuf;

use crate::JsonSchema;
use crate::TS;
use ruxex_protocol::ThreadId;
use ruxex_protocol::config_types::ForcedLoginMethod;
use ruxex_protocol::config_types::ReasoningSummary;
use ruxex_protocol::config_types::SandboxMode;
use ruxex_protocol::config_types::Verbosity;
use ruxex_protocol::model_types::ReasoningEffort;
use ruxex_protocol::protocol::AskForApproval;
use ruxex_protocol::protocol::SandboxPolicy;
use ruxex_protocol::protocol::SessionSource;
use ruxex_protocol::protocol::TurnAbortReason;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use serde::Deserialize;
use serde::Serialize;

use crate::protocol::v2::ForcedManagedWorkspaceIds;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub client_info: ClientInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<InitializeCapabilities>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub title: Option<String>,
    pub version: String,
}

/// Client-declared capabilities negotiated during initialize.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeCapabilities {
    /// Opt into receiving experimental API methods and fields.
    #[serde(default)]
    pub experimental_api: bool,
    /// Opt into `attestation/generate` requests for upstream `x-oai-attestation`.
    #[serde(default)]
    pub request_attestation: bool,
    /// Exact notification method names that should be suppressed for this
    /// connection (for example `thread/started`).
    #[ts(optional = nullable)]
    pub opt_out_notification_methods: Option<Vec<String>>,
    /// MCP extension settings declared by the app-server client.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResponse {
    pub user_agent: String,
    /// Absolute path to the server's $RUXEX_HOME directory.
    pub ruxex_home: AbsolutePathBuf,
    /// Platform family for the running app-server target, for example
    /// `"unix"`.
    pub platform_family: String,
    /// Operating system for the running app-server target, for example
    /// `"macos"` or `"linux"`.
    pub platform_os: String,
    pub app_server_identity: AppServerIdentity,
}

pub const APP_SERVER_IDENTITY_VERSION: &str = "app-server/1";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppServerIdentity {
    pub signature: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_version: Option<String>,
}

impl AppServerIdentity {
    pub fn from_provider_home(
        signature: impl Into<String>,
        provider_protocol_version: &str,
        build_version: Option<String>,
    ) -> Self {
        Self {
            signature: signature.into(),
            version: format!("{APP_SERVER_IDENTITY_VERSION};provider/{provider_protocol_version}"),
            build_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_server_identity_round_trips_with_a_diagnostic_build_version() {
        let response = InitializeResponse {
            user_agent: "ruxex-app-server/1.0".to_string(),
            ruxex_home: AbsolutePathBuf::from_absolute_path("/server/.ruxex")
                .expect("absolute path"),
            platform_family: "unix".to_string(),
            platform_os: "linux".to_string(),
            app_server_identity: AppServerIdentity {
                signature: "opaque-signature".to_string(),
                version: "app-server/1;provider/1".to_string(),
                build_version: Some("1.2.3".to_string()),
            },
        };

        let decoded: InitializeResponse =
            serde_json::from_value(serde_json::to_value(response.clone()).expect("serialize"))
                .expect("deserialize");

        assert_eq!(decoded, response);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub conversation_id: ThreadId,
    pub path: PathBuf,
    pub preview: String,
    pub timestamp: Option<String>,
    pub updated_at: Option<String>,
    pub cwd: PathBuf,
    pub cli_version: String,
    pub source: SessionSource,
    pub git_info: Option<ConversationGitInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub struct ConversationGitInfo {
    pub sha: Option<String>,
    pub branch: Option<String>,
    pub origin_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct LoginApiKeyParams {
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecOneOffCommandParams {
    pub command: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub cwd: Option<PathBuf>,
    pub sandbox_policy: Option<SandboxPolicy>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct UserSavedConfig {
    pub approval_policy: Option<AskForApproval>,
    pub sandbox_mode: Option<SandboxMode>,
    pub sandbox_settings: Option<SandboxSettings>,
    pub forced_managed_workspace_id: Option<ForcedManagedWorkspaceIds>,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub model: Option<String>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub tools: Option<Tools>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    pub web_search: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    #[serde(default)]
    pub writable_roots: Vec<AbsolutePathBuf>,
    pub network_access: Option<bool>,
    pub exclude_tmpdir_env_var: Option<bool>,
    pub exclude_slash_tmp: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct InterruptConversationResponse {
    pub abort_reason: TurnAbortReason,
}
