use super::ApplicationRequirements;
use super::AskForApproval;
use super::BrowserUseConfig;
use super::ComputerUseConfig;
use super::SandboxMode;
use crate::JsonSchema;
use crate::TS;
use genex_experimental_api_macros::ExperimentalApi;
use genex_protocol::config_types::AutoCompactTokenLimitScope;
use genex_protocol::config_types::ForcedLoginMethod;
use genex_protocol::config_types::ReasoningSummary;
use genex_protocol::config_types::Verbosity;
use genex_protocol::config_types::WebSearchMode;
use genex_protocol::config_types::WebSearchToolConfig;
use genex_protocol::model_types::ReasoningEffort;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_path_uri::PathUri;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type")]
#[ts(export_to = "v2/")]
pub enum ConfigLayerSource {
    /// Default configuration supplied with the installed Genex package.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    PackagedDefaults {
        /// Path to the packaged default configuration file.
        file: AbsolutePathBuf,
    },

    /// Managed preferences layer delivered by MDM (macOS only).
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Mdm { domain: String, key: String },

    /// Host-wide configuration loaded from a file.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    System {
        /// This is the path to the system config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,
    },

    /// Enterprise-managed config layer delivered by the cloud config bundle.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    EnterpriseManaged {
        /// Stable identifier for the delivered layer.
        id: String,

        /// Admin-facing name for the delivered layer. This is surfaced in
        /// diagnostics so users know which cloud layer needs administrator
        /// attention.
        name: String,
    },

    /// User config layer from $GENEX_HOME/config.toml. This layer is special
    /// in that it is expected to be:
    /// - writable by the user
    /// - generally outside the workspace directory
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    User {
        /// This is the path to the user's config.toml file, though it is not
        /// guaranteed to exist.
        file: AbsolutePathBuf,

        /// Name of the selected profile-v2 config layered on top of the base
        /// user config, when this layer represents one.
        profile: Option<String>,
    },

    /// Path to a .genex/ folder within a project. There could be multiple of
    /// these between `cwd` and the project/repo root.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Project { dot_genex_folder: AbsolutePathBuf },

    /// Session-layer overrides supplied via `-c`/`--config`.
    SessionFlags,
}

impl ConfigLayerSource {
    /// A settings from a layer with a higher precedence will override a setting
    /// from a layer with a lower precedence.
    pub fn precedence(&self) -> i16 {
        match self {
            ConfigLayerSource::PackagedDefaults { .. } => -10,
            ConfigLayerSource::Mdm { .. } => 50,
            ConfigLayerSource::System { .. } => 10,
            ConfigLayerSource::EnterpriseManaged { .. } => 15,
            ConfigLayerSource::User { profile, .. } => {
                if profile.is_some() {
                    21
                } else {
                    20
                }
            }
            ConfigLayerSource::Project { .. } => 25,
            ConfigLayerSource::SessionFlags => 30,
        }
    }
}

/// Compares [ConfigLayerSource] by precedence, so `A < B` means settings from
/// layer `A` will be overridden by settings from layer `B`.
impl PartialOrd for ConfigLayerSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.precedence().cmp(&other.precedence()))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct SandboxWorkspaceWrite {
    #[serde(default)]
    pub writable_roots: Vec<PathBuf>,
    #[serde(default)]
    pub network_access: bool,
    #[serde(default)]
    pub exclude_tmpdir_env_var: bool,
    #[serde(default)]
    pub exclude_slash_tmp: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct ToolsV2 {
    pub web_search: Option<WebSearchToolConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct MetricsConfig {
    pub enabled: Option<bool>,
    #[serde(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

/// API shape for managed-auth workspace login restrictions.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(untagged)]
#[ts(export_to = "v2/")]
pub enum ForcedManagedWorkspaceIds {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "v2/")]
pub struct Config {
    pub model: Option<String>,
    pub model_context_window: Option<i64>,
    pub model_auto_compact_token_limit: Option<i64>,
    pub model_auto_compact_token_limit_scope: Option<AutoCompactTokenLimitScope>,
    pub model_provider: Option<String>,
    #[experimental(nested)]
    pub approval_policy: Option<AskForApproval>,
    pub sandbox_mode: Option<SandboxMode>,
    pub sandbox_workspace_write: Option<SandboxWorkspaceWrite>,
    pub forced_managed_workspace_id: Option<ForcedManagedWorkspaceIds>,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub web_search: Option<WebSearchMode>,
    pub tools: Option<ToolsV2>,
    pub instructions: Option<String>,
    pub developer_instructions: Option<String>,
    pub compact_prompt: Option<String>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub model_reasoning_summary: Option<ReasoningSummary>,
    pub model_verbosity: Option<Verbosity>,
    pub service_tier: Option<String>,
    pub metrics: Option<MetricsConfig>,
    pub browser_use: Option<BrowserUseConfig>,
    pub computer_use: Option<ComputerUseConfig>,
    pub desktop: Option<HashMap<String, JsonValue>>,
    #[serde(default, flatten)]
    pub additional: HashMap<String, JsonValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigLayerMetadata {
    pub name: ConfigLayerSource,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigLayer {
    pub name: ConfigLayerSource,
    pub version: String,
    pub config: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum MergeStrategy {
    Replace,
    Upsert,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum WriteStatus {
    Ok,
    OkOverridden,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct OverriddenMetadata {
    pub message: String,
    pub overriding_layer: ConfigLayerMetadata,
    pub effective_value: JsonValue,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigWriteResponse {
    pub status: WriteStatus,
    pub version: String,
    /// Canonical path to the config file that was written.
    pub file_path: AbsolutePathBuf,
    pub overridden_metadata: Option<OverriddenMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ConfigWriteErrorCode {
    ConfigLayerReadonly,
    ConfigRequirementReadonly,
    ConfigVersionConflict,
    ConfigValidationError,
    ConfigPathNotFound,
    ConfigSchemaUnknownKey,
    UserLayerNotFound,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigReadParams {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_layers: bool,
    /// Optional working directory to resolve project config layers. If specified,
    /// return the effective config as seen from that directory (i.e., including any
    /// project layers between `cwd` and the project/repo root).
    #[ts(optional = nullable)]
    pub cwd: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigReadResponse {
    #[experimental(nested)]
    pub config: Config,
    pub origins: HashMap<String, ConfigLayerMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<ConfigLayer>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigRequirements {
    /// Exact provider selection required by managed policy.
    pub model_provider: Option<String>,
    /// Complete required provider definitions, using config.toml field names.
    pub model_providers: Option<HashMap<String, JsonValue>>,
    /// Effective login methods after managed, forced-login, and workspace restrictions.
    /// An empty list permits no login method. Older servers may omit this field.
    pub allowed_login_methods: Option<Vec<ForcedLoginMethod>>,
    pub cli_auth_credentials_store: Option<CliAuthCredentialsStoreMode>,
    pub managed_auth_base_url: Option<String>,
    pub additional_developer_instructions: Option<String>,
    #[experimental(nested)]
    pub allowed_approval_policies: Option<Vec<AskForApproval>>,
    pub allowed_sandbox_modes: Option<Vec<SandboxMode>>,
    pub allowed_permission_profiles: Option<BTreeMap<String, bool>>,
    pub default_permissions: Option<String>,
    pub allowed_web_search_modes: Option<Vec<WebSearchMode>>,
    pub allow_managed_hooks_only: Option<bool>,
    pub allow_browser_and_computer_use: Option<bool>,
    pub allow_appshots: Option<bool>,
    pub computer_use: Option<ComputerUseRequirements>,
    pub browser_use: Option<BrowserUseRequirements>,
    pub in_app_browser: Option<InAppBrowserRequirements>,
    pub feature_requirements: Option<BTreeMap<String, bool>>,
    #[experimental("configRequirements/read.hooks")]
    pub hooks: Option<ManagedHooksRequirements>,
    pub enforce_residency: Option<ResidencyRequirement>,
    #[experimental("configRequirements/read.network")]
    pub network: Option<NetworkRequirements>,
    #[experimental("configRequirements/read.application")]
    pub application: Option<ApplicationRequirements>,
    pub models: Option<ModelsRequirements>,
    #[schemars(with = "Option<String>")]
    pub sqlite_home: Option<PathUri>,
    #[schemars(with = "Option<String>")]
    pub log_dir: Option<PathUri>,
    #[schemars(with = "Option<String>")]
    pub model_catalog_json: Option<PathUri>,
    pub allow_login_shell: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/", rename_all = "camelCase")]
pub enum CliAuthCredentialsStoreMode {
    File,
    Keyring,
    Auto,
    Ephemeral,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelsRequirements {
    pub new_thread: Option<NewThreadModelDefaults>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct NewThreadModelDefaults {
    pub model: Option<String>,
    pub model_reasoning_effort: Option<ReasoningEffort>,
    pub service_tier: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ComputerUseRequirements {
    pub allow_locked_computer_use: Option<bool>,
    pub allow_persistent_approval: Option<bool>,
    pub default_app_access: Option<AllowDenyRequirement>,
    pub macos: Option<ComputerUseMacosRequirements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct BrowserUseRequirements {
    pub allow_webmcp: Option<bool>,
    pub allow_history_access: Option<bool>,
    pub disable_auto_review: Option<bool>,
    pub allow_global_persistent_approval: Option<bool>,
    pub default_origin_policy: Option<BrowserUseOriginPolicy>,
    pub origins: Option<BTreeMap<String, BrowserUseOriginPolicy>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct BrowserUseOriginPolicy {
    pub access: Option<AllowDenyRequirement>,
    pub downloads: Option<AllowDenyRequirement>,
    pub uploads: Option<AllowDenyRequirement>,
    pub full_cdp_access: Option<AllowDenyRequirement>,
    pub auto_review: Option<AllowDenyRequirement>,
    pub persistent_approval: Option<bool>,
    pub access_approval_lifetime: Option<BrowserUseAccessApprovalLifetime>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase", export_to = "v2/")]
pub enum AllowDenyRequirement {
    Allow,
    Deny,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase", export_to = "v2/")]
pub enum BrowserUseAccessApprovalLifetime {
    Turn,
    Thread,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ComputerUseMacosRequirements {
    pub bundle_ids: Option<BTreeMap<String, AllowDenyRequirement>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct InAppBrowserRequirements {
    pub allow_external_browser_settings_import: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ManagedHooksRequirements {
    pub managed_dir: Option<PathBuf>,
    #[serde(rename = "PreToolUse")]
    #[ts(rename = "PreToolUse")]
    pub pre_tool_use: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "PermissionRequest")]
    #[ts(rename = "PermissionRequest")]
    pub permission_request: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "PostToolUse")]
    #[ts(rename = "PostToolUse")]
    pub post_tool_use: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "PreCompact")]
    #[ts(rename = "PreCompact")]
    pub pre_compact: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "PostCompact")]
    #[ts(rename = "PostCompact")]
    pub post_compact: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "SessionStart")]
    #[ts(rename = "SessionStart")]
    pub session_start: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "SessionEnd", default)]
    #[ts(rename = "SessionEnd")]
    pub session_end: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "UserPromptSubmit")]
    #[ts(rename = "UserPromptSubmit")]
    pub user_prompt_submit: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "SubagentStart")]
    #[ts(rename = "SubagentStart")]
    pub subagent_start: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "SubagentStop")]
    #[ts(rename = "SubagentStop")]
    pub subagent_stop: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "Stop")]
    #[ts(rename = "Stop")]
    pub stop: Vec<ConfiguredHookMatcherGroup>,
    #[serde(rename = "Interrupt", default)]
    #[ts(rename = "Interrupt")]
    pub interrupt: Vec<ConfiguredHookMatcherGroup>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfiguredHookMatcherGroup {
    pub matcher: Option<String>,
    pub hooks: Vec<ConfiguredHookHandler>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(tag = "type")]
#[ts(tag = "type", export_to = "v2/")]
pub enum ConfiguredHookHandler {
    #[serde(rename = "command")]
    #[ts(rename = "command")]
    Command {
        command: String,
        #[serde(rename = "timeoutSec")]
        #[ts(rename = "timeoutSec")]
        timeout_sec: Option<u64>,
        r#async: bool,
        #[serde(rename = "statusMessage")]
        #[ts(rename = "statusMessage")]
        status_message: Option<String>,
        /// Approximate token threshold for spilling this hook's `additionalContext` to disk.
        /// `null` uses 2,500 tokens; `0` disables spilling for this hook. The threshold is
        /// evaluated against the original context; a spilled preview also includes recovery
        /// metadata.
        #[serde(rename = "additionalContextLimit")]
        #[ts(rename = "additionalContextLimit")]
        additional_context_limit: Option<usize>,
    },
    #[serde(rename = "mcp_tool")]
    #[ts(rename = "mcp_tool")]
    McpTool {
        server: String,
        tool: String,
        input: serde_json::Map<String, serde_json::Value>,
        #[serde(rename = "timeoutSec")]
        #[ts(rename = "timeoutSec")]
        timeout_sec: Option<u64>,
        #[serde(rename = "statusMessage")]
        #[ts(rename = "statusMessage")]
        status_message: Option<String>,
    },
    #[serde(rename = "prompt")]
    #[ts(rename = "prompt")]
    Prompt {},
    #[serde(rename = "agent")]
    #[ts(rename = "agent")]
    Agent {},
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct NetworkRequirements {
    pub enabled: Option<bool>,
    pub http_port: Option<u16>,
    pub socks_port: Option<u16>,
    pub allow_upstream_proxy: Option<bool>,
    pub dangerously_allow_non_loopback_proxy: Option<bool>,
    pub dangerously_allow_all_unix_sockets: Option<bool>,
    /// Canonical network permission map for `experimental_network`.
    pub domains: Option<BTreeMap<String, NetworkDomainPermission>>,
    /// When true, only managed allowlist entries are respected while managed
    /// network enforcement is active.
    pub managed_allowed_domains_only: Option<bool>,
    /// Legacy compatibility view derived from `domains`.
    pub allowed_domains: Option<Vec<String>>,
    /// Legacy compatibility view derived from `domains`.
    pub denied_domains: Option<Vec<String>>,
    /// Canonical unix socket permission map for `experimental_network`.
    pub unix_sockets: Option<BTreeMap<String, NetworkUnixSocketPermission>>,
    /// Legacy compatibility view derived from `unix_sockets`.
    pub allow_unix_sockets: Option<Vec<String>>,
    pub allow_local_binding: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum NetworkDomainPermission {
    Allow,
    Deny,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "v2/")]
pub enum NetworkUnixSocketPermission {
    Allow,
    Deny,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum ResidencyRequirement {
    Us,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS, ExperimentalApi)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigRequirementsReadResponse {
    /// Null if no requirements are configured (e.g. no requirements.toml/MDM entries).
    #[experimental(nested)]
    pub requirements: Option<ConfigRequirements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigValueWriteParams {
    pub key_path: String,
    pub value: JsonValue,
    pub merge_strategy: MergeStrategy,
    /// Path to the config file to write; defaults to the user's `config.toml` when omitted.
    #[ts(optional = nullable)]
    pub file_path: Option<String>,
    #[ts(optional = nullable)]
    pub expected_version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigBatchWriteParams {
    pub edits: Vec<ConfigEdit>,
    /// Path to the config file to write; defaults to the user's `config.toml` when omitted.
    #[ts(optional = nullable)]
    pub file_path: Option<String>,
    #[ts(optional = nullable)]
    pub expected_version: Option<String>,
    /// When true, hot-reload updated runtime settings into loaded threads after writing.
    /// Session-static model, reasoning-effort, Plan-mode reasoning-effort, and service-tier
    /// defaults are not reloaded.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reload_user_config: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigEdit {
    pub key_path: String,
    pub value: JsonValue,
    pub merge_strategy: MergeStrategy,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TextPosition {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (in Unicode scalar values).
    pub column: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ConfigWarningNotification {
    /// Concise summary of the warning.
    pub summary: String,
    /// Optional extra guidance or error details.
    pub details: Option<String>,
    /// Optional path to the config file that triggered the warning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub path: Option<String>,
    /// Optional range for the error location inside the config file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub range: Option<TextRange>,
}
