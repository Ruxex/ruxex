use super::shared::v2_enum_from_core;
use super::turn::CyberAccessProgram;
use crate::JsonSchema;
use crate::TS;
use genex_protocol::account::ProviderAuthStatus;
use genex_protocol::model_types::InputModality;
use genex_protocol::model_types::ModelAccessPrograms as CoreModelAccessPrograms;
use genex_protocol::model_types::ModelAvailabilityNux as CoreModelAvailabilityNux;
use genex_protocol::model_types::ReasoningEffort;
use genex_protocol::model_types::default_input_modalities;
use genex_protocol::protocol::ModelRerouteReason as CoreModelRerouteReason;
use genex_protocol::protocol::ModelVerification as CoreModelVerification;
use genex_protocol::protocol::MultiAgentVersion as CoreMultiAgentVersion;
use serde::Deserialize;
use serde::Serialize;

v2_enum_from_core!(
    pub enum ModelRerouteReason from CoreModelRerouteReason {
        HighRiskCyberActivity
    }
);

v2_enum_from_core!(
    pub enum ModelVerification from CoreModelVerification {
        TrustedAccessForCyber
    }
);

v2_enum_from_core!(
    /// Multi-agent runtime supported by a model.
    pub enum MultiAgentVersion from CoreMultiAgentVersion {
        Disabled,
        V1,
        V2
    }
);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelProviderCapabilitiesReadParams {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub enum RemoteCompactionSupport {
    Unsupported,
    V2,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelProviderCapabilitiesReadResponse {
    pub model_listing: bool,
    pub api_key_model_discovery: bool,
    pub command_auth: bool,
    pub streaming: bool,
    pub websocket: bool,
    pub usage: bool,
    pub interactive_login: bool,
    pub namespace_tools: bool,
    pub web_search: bool,
    pub standalone_web_search: bool,
    pub external_web_access: bool,
    pub history_notes: bool,
    pub remote_compaction: RemoteCompactionSupport,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelProviderReadParams {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelProviderReadResponse {
    pub id: String,
    pub name: String,
    pub auth_status: ProviderAuthStatus,
    pub auth_fields: Vec<ModelProviderAuthField>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelProviderAuthField {
    pub id: String,
    pub label: String,
    pub secret: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelListParams {
    /// Opaque pagination cursor returned by a previous call.
    #[ts(optional = nullable)]
    pub cursor: Option<String>,
    /// Optional page size; defaults to a reasonable server-side value.
    #[ts(optional = nullable)]
    pub limit: Option<u32>,
    /// When true, include models that are hidden from the default picker list.
    #[ts(optional = nullable)]
    pub include_hidden: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelAvailabilityNux {
    pub message: String,
}

impl From<CoreModelAvailabilityNux> for ModelAvailabilityNux {
    fn from(value: CoreModelAvailabilityNux) -> Self {
        Self {
            message: value.message,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelServiceTier {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Caller-specific explicit access programs advertised by model discovery.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelAccessPrograms {
    /// Accepted explicit selections.
    pub cyber: Vec<CyberAccessProgram>,
}

impl From<CoreModelAccessPrograms> for ModelAccessPrograms {
    fn from(value: CoreModelAccessPrograms) -> Self {
        Self {
            cyber: value.cyber.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ModelAccessPrograms> for CoreModelAccessPrograms {
    fn from(value: ModelAccessPrograms) -> Self {
        Self {
            cyber: value.cyber.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct Model {
    pub id: String,
    pub model: String,
    pub upgrade: Option<String>,
    pub upgrade_info: Option<ModelUpgradeInfo>,
    pub availability_nux: Option<ModelAvailabilityNux>,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub model_specialty: Option<String>,
    pub hidden: bool,
    pub supported_reasoning_efforts: Vec<ReasoningEffortOption>,
    pub default_reasoning_effort: ReasoningEffort,
    #[serde(default = "default_input_modalities")]
    pub input_modalities: Vec<InputModality>,
    /// Multi-agent runtime declared by this model, when available.
    pub multi_agent_version: Option<MultiAgentVersion>,
    /// Deprecated: use `serviceTiers` instead.
    #[serde(default)]
    pub additional_speed_tiers: Vec<String>,
    #[serde(default)]
    pub service_tiers: Vec<ModelServiceTier>,
    /// Catalog default service tier id for this model, when one is configured.
    #[serde(default)]
    pub default_service_tier: Option<String>,
    /// Null when the catalog does not provide access-program metadata.
    #[serde(default)]
    pub available_access_programs: Option<ModelAccessPrograms>,
    // Only one model should be marked as default.
    pub is_default: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelUpgradeInfo {
    pub model: String,
    pub upgrade_copy: Option<String>,
    pub model_link: Option<String>,
    pub migration_markdown: Option<String>,
    /// Informational Unix timestamp for this upgrade's scheduled retirement, if known.
    #[ts(type = "number | null")]
    pub retirement_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ReasoningEffortOption {
    pub reasoning_effort: ReasoningEffort,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelListResponse {
    pub data: Vec<Model>,
    /// Opaque cursor to pass to the next call to continue after the last item.
    /// If None, there are no more items to return.
    pub next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelReroutedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub from_model: String,
    pub to_model: String,
    pub reason: ModelRerouteReason,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelVerificationNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub verifications: Vec<ModelVerification>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ModelSafetyBufferingUpdatedNotification {
    pub thread_id: String,
    pub turn_id: String,
    pub model: String,
    pub use_cases: Vec<String>,
    pub reasons: Vec<String>,
    pub show_buffering_ui: bool,
    pub faster_model: Option<String>,
}
