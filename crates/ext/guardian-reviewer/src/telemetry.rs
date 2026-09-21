use codex_protocol::approvals::GuardianCommandSource;
use codex_protocol::approvals::NetworkApprovalProtocol;
use codex_protocol::models::AdditionalPermissionProfile;
use codex_protocol::protocol::GuardianAssessmentOutcome;
use codex_protocol::protocol::GuardianRiskLevel;
use codex_protocol::protocol::GuardianUserAuthorization;
use codex_protocol::models::SandboxPermissions;
use codex_protocol::protocol::TokenUsage;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardianReviewDecision {
    Approved,
    Denied,
    Aborted,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardianReviewTerminalStatus {
    Approved,
    Denied,
    Aborted,
    TimedOut,
    FailedClosed,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardianReviewFailureReason {
    Timeout,
    Cancelled,
    PromptBuildError,
    SessionError,
    ParseError,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardianReviewSessionKind {
    TrunkNew,
    TrunkReused,
    EphemeralForked,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardianApprovalRequestSource {
    MainTurn,
    DelegatedSubagent,
}

#[derive(Clone, Debug, Serialize)]
pub struct GuardianAdditionalPermissions {
    network: Option<GuardianNetworkPermissions>,
}

#[derive(Clone, Debug, Serialize)]
struct GuardianNetworkPermissions {
    enabled: Option<bool>,
}

impl From<&AdditionalPermissionProfile> for GuardianAdditionalPermissions {
    fn from(permissions: &AdditionalPermissionProfile) -> Self {
        Self {
            network: permissions
                .network
                .as_ref()
                .map(|network| GuardianNetworkPermissions {
                    enabled: network.enabled,
                }),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GuardianReviewedAction {
    Shell {
        sandbox_permissions: SandboxPermissions,
        additional_permissions: Option<GuardianAdditionalPermissions>,
    },
    UnifiedExec {
        sandbox_permissions: SandboxPermissions,
        additional_permissions: Option<GuardianAdditionalPermissions>,
        tty: bool,
    },
    WriteStdin { tty: bool },
    Execve {
        source: GuardianCommandSource,
        additional_permissions: Option<GuardianAdditionalPermissions>,
    },
    ApplyPatch {},
    NetworkAccess {
        protocol: NetworkApprovalProtocol,
        port: u16,
    },
    McpToolCall {
        server: String,
        tool_name: String,
        connector_id: Option<String>,
        connector_name: Option<String>,
        tool_title: Option<String>,
    },
    RequestPermissions {},
}

#[derive(Debug)]
pub struct GuardianReviewAnalyticsResult {
    pub decision: GuardianReviewDecision,
    pub terminal_status: GuardianReviewTerminalStatus,
    pub failure_reason: Option<GuardianReviewFailureReason>,
    pub attempt_count: i64,
    pub risk_level: Option<GuardianRiskLevel>,
    pub user_authorization: Option<GuardianUserAuthorization>,
    pub outcome: Option<GuardianAssessmentOutcome>,
    pub guardian_thread_id: Option<String>,
    pub guardian_session_kind: Option<GuardianReviewSessionKind>,
    pub guardian_model: Option<String>,
    pub guardian_reasoning_effort: Option<String>,
    pub guardian_default_review_model_id: Option<String>,
    pub guardian_catalog_contains_auto_review: Option<bool>,
    pub guardian_review_model_overridden: Option<bool>,
    pub guardian_review_model_override: Option<String>,
    pub guardian_model_provider_id: Option<String>,
    pub had_prior_review_context: Option<bool>,
    pub reviewed_action_truncated: bool,
    pub token_usage: Option<TokenUsage>,
    pub time_to_first_token_ms: Option<u64>,
}

impl GuardianReviewAnalyticsResult {
    pub fn without_session() -> Self {
        Self {
            decision: GuardianReviewDecision::Denied,
            terminal_status: GuardianReviewTerminalStatus::FailedClosed,
            failure_reason: None,
            attempt_count: 1,
            risk_level: None,
            user_authorization: None,
            outcome: None,
            guardian_thread_id: None,
            guardian_session_kind: None,
            guardian_model: None,
            guardian_reasoning_effort: None,
            guardian_default_review_model_id: None,
            guardian_catalog_contains_auto_review: None,
            guardian_review_model_overridden: None,
            guardian_review_model_override: None,
            guardian_model_provider_id: None,
            had_prior_review_context: None,
            reviewed_action_truncated: false,
            token_usage: None,
            time_to_first_token_ms: None,
        }
    }

    pub fn from_session(params: GuardianReviewSessionAnalyticsParams) -> Self {
        Self {
            guardian_thread_id: Some(params.guardian_thread_id),
            guardian_session_kind: Some(params.guardian_session_kind),
            guardian_model: Some(params.guardian_model),
            guardian_reasoning_effort: params.guardian_reasoning_effort,
            guardian_default_review_model_id: Some(params.guardian_default_review_model_id),
            guardian_catalog_contains_auto_review: Some(params.guardian_catalog_contains_auto_review),
            guardian_review_model_overridden: Some(params.guardian_review_model_overridden),
            guardian_review_model_override: params.guardian_review_model_override,
            guardian_model_provider_id: Some(params.guardian_model_provider_id),
            had_prior_review_context: Some(params.had_prior_review_context),
            ..Self::without_session()
        }
    }
}

pub struct GuardianReviewSessionAnalyticsParams {
    pub guardian_thread_id: String,
    pub guardian_session_kind: GuardianReviewSessionKind,
    pub guardian_model: String,
    pub guardian_reasoning_effort: Option<String>,
    pub guardian_default_review_model_id: String,
    pub guardian_catalog_contains_auto_review: bool,
    pub guardian_review_model_overridden: bool,
    pub guardian_review_model_override: Option<String>,
    pub guardian_model_provider_id: String,
    pub had_prior_review_context: bool,
}
