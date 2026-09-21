//! Central approval policy-stage execution and reviewer routing.

use crate::command_canonicalization::canonicalize_command_for_approval;
use crate::exec_policy::prompt_is_rejected_by_policy;
use crate::hook_runtime::run_permission_request_hooks;
use crate::mcp_tool_call::request_mcp_tool_user_approval;
use crate::sandboxing::SandboxPermissions;
use crate::session::session::Session;
use crate::tools::hook_names::HookToolName;
use crate::tools::runtimes::apply_patch::ApplyPatchApprovalKey;
use crate::tools::runtimes::unified_exec::UnifiedExecApprovalKey;
use crate::tools::sandboxing::ApprovalRequestReasons;
use crate::tools::sandboxing::PermissionRequestPayload;
use crate::tools::sandboxing::ToolError;
use crate::tools::sandboxing::with_cached_approval;
use codex_config::types::AppToolApproval;
use codex_hooks::PermissionRequestDecision;
use codex_otel::ToolDecisionSource;
use codex_protocol::approvals::ExecApprovalKind;
use codex_protocol::approvals::ExecPolicyAmendment;
#[cfg(unix)]
use codex_protocol::approvals::NetworkApprovalContext;
use codex_protocol::approvals::NetworkApprovalProtocol;
use codex_protocol::error::CodexErr;
use codex_protocol::models::AdditionalPermissionProfile;
use codex_protocol::items::ModelInvocationContext;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::FileChange;
use codex_protocol::protocol::NetworkPolicyRuleAction;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::request_permissions::RequestPermissionProfile;
use codex_tools::ToolName;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::PathUri;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::warn;

#[derive(Clone)]
pub(crate) struct ApprovalReviewContext {
    turn: Arc<crate::session::turn_context::TurnContext>,
    environments: crate::environment_selection::TurnEnvironmentSnapshot,
    pub(crate) model_info: Arc<ModelInfo>,
    pub(crate) approval_policy: AskForApproval,
}

impl ApprovalReviewContext {
    pub(crate) fn turn(&self) -> &Arc<crate::session::turn_context::TurnContext> {
        &self.turn
    }

    pub(crate) fn environments(&self) -> &crate::environment_selection::TurnEnvironmentSnapshot {
        &self.environments
    }

    pub(crate) fn model_context(&self) -> ModelInvocationContext {
        ModelInvocationContext {
            model_slug: self.model_info.slug.clone(),
            reasoning_effort: self.model_info.default_reasoning_level.as_ref().map(ToString::to_string),
        }
    }
}

impl From<&Arc<crate::session::step_context::StepContext>> for ApprovalReviewContext {
    fn from(step: &Arc<crate::session::step_context::StepContext>) -> Self {
        Self {
            turn: Arc::clone(&step.turn),
            environments: step.environments.clone(),
            model_info: Arc::clone(&step.settings.model_info),
            approval_policy: step.settings.approval_policy(),
        }
    }
}

impl From<&Arc<crate::session::turn_context::TurnContext>> for ApprovalReviewContext {
    fn from(turn: &Arc<crate::session::turn_context::TurnContext>) -> Self {
        Self {
            turn: Arc::clone(turn),
            environments: turn.environments.clone(),
            model_info: turn.model_info().clone(),
            approval_policy: turn.approval_policy(),
        }
    }
}

impl ApprovalReviewContext {
    pub(crate) fn from_resolved_settings(
        turn: Arc<crate::session::turn_context::TurnContext>,
        settings: &crate::session::step_settings::ResolvedStepSettings,
        environments: &crate::environment_selection::TurnEnvironmentSnapshot,
    ) -> Self {
        Self {
            turn,
            environments: environments.clone(),
            model_info: Arc::clone(&settings.model_info),
            approval_policy: settings.approval_policy(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ApprovalContext {
    pub(crate) review_context: ApprovalReviewContext,
    pub(crate) cancellation_token: Option<CancellationToken>,
    pub(crate) call_id: String,
    pub(crate) tool_name: ToolName,
    pub(crate) approval_reason: Option<String>,
    pub(crate) retry_reason: Option<String>,
    pub(crate) network_approval_context: Option<NetworkApprovalContext>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub(crate) enum ApprovalAction {
    ExecCommand {
        id: String,
        environment_id: String,
        command: Vec<String>,
        #[serde(skip_serializing)]
        hook_command: String,
        cwd: PathUri,
        sandbox_permissions: SandboxPermissions,
        additional_permissions: Option<AdditionalPermissionProfile>,
        justification: Option<String>,
        tty: bool,
        proposed_execpolicy_amendment: Option<ExecPolicyAmendment>,
    },
    WriteStdin {
        id: String,
        approval_id: String,
        environment_id: String,
        process_id: i32,
        input: String,
        cwd: PathUri,
        tty: bool,
        sandbox_permissions: SandboxPermissions,
        additional_permissions: Option<AdditionalPermissionProfile>,
    },
    #[cfg(unix)]
    Execve {
        id: String,
        approval_id: String,
        environment_id: String,
        program: AbsolutePathBuf,
        argv: Vec<String>,
        command: Vec<String>,
        cwd: AbsolutePathBuf,
        additional_permissions: Option<AdditionalPermissionProfile>,
    },
    ApplyPatch {
        id: String,
        environment_id: String,
        cwd: PathUri,
        files: Vec<PathUri>,
        patch: String,
        #[serde(skip_serializing)]
        changes: Arc<HashMap<PathBuf, FileChange>>,
        permissions_preapproved: bool,
    },
    McpToolCall {
        id: String,
        server: String,
        tool_name: String,
        arguments: Option<serde_json::Value>,
        connector_id: Option<String>,
        connector_name: Option<String>,
        connector_description: Option<String>,
        connected_account_email: Option<String>,
        tool_title: Option<String>,
        tool_description: Option<String>,
        #[serde(skip_serializing)]
        hook_tool_name: HookToolName,
        approval_policy: AskForApproval,
        approval_mode: AppToolApproval,
        allow_session_remember: bool,
        allow_persistent_approval: bool,
    },
    NetworkAccess {
        id: String,
        turn_id: String,
        environment_id: String,
        target: String,
        host: String,
        protocol: NetworkApprovalProtocol,
        port: u16,
        #[serde(skip_serializing)]
        hook_command: String,
        #[serde(skip_serializing)]
        hook_run_id: String,
        command: Vec<String>,
        cwd: AbsolutePathBuf,
    },
    RequestPermissions {
        id: String,
        turn_id: String,
        reason: Option<String>,
        permissions: RequestPermissionProfile,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Serialize)]
#[serde(untagged)]
pub(crate) enum ApprovalCacheKey {
    ExecCommand(UnifiedExecApprovalKey),
    ApplyPatch(ApplyPatchApprovalKey),
}

impl ApprovalAction {
    pub(crate) fn permission_request_payload(&self) -> PermissionRequestPayload {
        match self {
            Self::ExecCommand {
                hook_command,
                justification,
                ..
            } => PermissionRequestPayload::bash(hook_command.clone(), justification.clone()),
            Self::WriteStdin {
                id,
                approval_id,
                environment_id,
                process_id,
                input,
                cwd,
                tty,
                sandbox_permissions,
                additional_permissions,
            } => PermissionRequestPayload {
                tool_name: HookToolName::new("write_stdin"),
                tool_input: serde_json::json!({
                    "session_id": process_id,
                    "chars": input,
                    "parent_call_id": id,
                    "approval_id": approval_id,
                    "environment_id": environment_id,
                    "cwd": cwd,
                    "tty": tty,
                    "sandbox_permissions": sandbox_permissions,
                    "additional_permissions": additional_permissions,
                }),
            },
            #[cfg(unix)]
            Self::Execve { command, .. } => PermissionRequestPayload::bash(
                codex_shell_command::parse_command::shlex_join(command),
                /*description*/ None,
            ),
            Self::ApplyPatch { patch, .. } => PermissionRequestPayload {
                tool_name: HookToolName::apply_patch(),
                tool_input: serde_json::json!({ "command": patch }),
            },
            Self::McpToolCall {
                hook_tool_name,
                arguments,
                ..
            } => PermissionRequestPayload {
                tool_name: hook_tool_name.clone(),
                tool_input: arguments
                    .clone()
                    .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
            },
            Self::NetworkAccess {
                hook_command,
                target,
                ..
            } => PermissionRequestPayload::bash(
                hook_command.clone(),
                Some(format!("network-access {target}")),
            ),
            Self::RequestPermissions {
                reason,
                permissions,
                ..
            } => PermissionRequestPayload {
                tool_name: HookToolName::new("request_permissions"),
                tool_input: serde_json::json!({
                    "reason": reason,
                    "permissions": permissions,
                }),
            },
        }
    }

    pub(crate) fn cache_keys(&self) -> Vec<ApprovalCacheKey> {
        match self {
            Self::ExecCommand {
                environment_id,
                command,
                cwd,
                tty,
                sandbox_permissions,
                additional_permissions,
                ..
            } => vec![ApprovalCacheKey::ExecCommand(UnifiedExecApprovalKey {
                environment_id: environment_id.clone(),
                executable: command.first().cloned(),
                command: canonicalize_command_for_approval(command),
                cwd: cwd.clone(),
                tty: *tty,
                sandbox_permissions: *sandbox_permissions,
                additional_permissions: additional_permissions.clone(),
            })],
            #[cfg(unix)]
            Self::Execve { .. } => Vec::new(),
            Self::McpToolCall { .. }
            | Self::NetworkAccess { .. }
            | Self::RequestPermissions { .. }
            | Self::WriteStdin { .. } => Vec::new(),
            Self::ApplyPatch {
                environment_id,
                files,
                ..
            } => files
                .iter()
                .cloned()
                .map(|path| {
                    ApprovalCacheKey::ApplyPatch(ApplyPatchApprovalKey {
                        environment_id: environment_id.clone(),
                        path,
                    })
                })
                .collect(),
        }
    }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ApprovalResolutionSource {
    Hook,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ApprovalResolution {
    decision: ReviewDecision,
    source: ApprovalResolutionSource,
}

impl ApprovalResolution {
    fn into_tool_result(self, model_info: &ModelInfo) -> Result<ReviewDecision, ToolError> {
        let source = self.source;
        match self.decision {
            ReviewDecision::ApprovedMcpPolicyAmendment => {
                error!("Tool approval received ApprovedMcpPolicyAmendment");
                Err(ToolError::Rejected(
                    "Error while requesting approval".to_string(),
                ))
            }
            ReviewDecision::NetworkPolicyAmendment {
                network_policy_amendment,
            } if network_policy_amendment.action == NetworkPolicyRuleAction::Deny => {
                let rejection = match source {
                    ApprovalResolutionSource::Hook => "rejected by configuration",
                    ApprovalResolutionSource::User => "rejected by user",
                };
                Err(ToolError::Rejected(rejection.to_string()))
            }
            ReviewDecision::Denied { rejection } => Err(ToolError::Rejected(rejection)),
            ReviewDecision::TimedOut => Err(ToolError::Rejected("approval request timed out".to_string())),
            ReviewDecision::Abort => Err(ToolError::Codex(CodexErr::TurnAborted)),
            decision => Ok(decision),
        }
    }
}

impl Session {
    pub(crate) async fn request_approval(
        self: &Arc<Self>,
        action: ApprovalAction,
        ctx: ApprovalContext,
    ) -> Result<ReviewDecision, ToolError> {
        // Stdin that exceeds current permissions needs a fresh sandbox approval.
        // Strict review of ordinary input follows the same routing as ordinary exec.
        let policy = ctx.review_context.turn().approval_policy();
        if matches!(&action, ApprovalAction::WriteStdin { sandbox_permissions, .. }
            if sandbox_permissions.requests_sandbox_override())
            && let Some(reason) =
                prompt_is_rejected_by_policy(policy, /*prompt_is_rule*/ false)
        {
            return Err(ToolError::Rejected(reason.to_string()));
        }
        let is_mcp_tool_call = matches!(&action, ApprovalAction::McpToolCall { .. });
        let is_network_approval = matches!(&action, ApprovalAction::NetworkAccess { .. });
        let permission_request_run_id = match &action {
            #[cfg(unix)]
            ApprovalAction::Execve { approval_id, .. } => approval_id.clone(),
            ApprovalAction::NetworkAccess { hook_run_id, .. } => hook_run_id.clone(),
            _ if ctx.retry_reason.is_some() => format!("{}:retry", ctx.call_id),
            _ => ctx.call_id.clone(),
        };

        // Approval precedence is hooks, then the normal user approval flow.
        let resolution = match run_permission_request_hooks(
            self,
            &ctx.review_context,
            &permission_request_run_id,
            action.permission_request_payload(),
        )
        .await
        {
            Some(PermissionRequestDecision::Allow) => ApprovalResolution {
                decision: ReviewDecision::Approved,
                source: ApprovalResolutionSource::Hook,
            },
            Some(PermissionRequestDecision::Deny { message }) => ApprovalResolution {
                decision: ReviewDecision::denied(message),
                source: ApprovalResolutionSource::Hook,
            },
            None => ApprovalResolution {
                decision: self.request_user_approval(&action, &ctx).await,
                source: ApprovalResolutionSource::User,
            },
        };
        // Network approvals record their final telemetry after validation and persistence.
        if !is_network_approval {
            record_resolution(&ctx, &resolution);
        }
        if is_mcp_tool_call && resolution.decision == ReviewDecision::ApprovedMcpPolicyAmendment {
            return Ok(resolution.decision);
        }
        if is_network_approval {
            match (&resolution.decision, resolution.source) {
                (
                    ReviewDecision::NetworkPolicyAmendment {
                        network_policy_amendment,
                    },
                    _,
                ) if network_policy_amendment.action == NetworkPolicyRuleAction::Deny => {
                    return Ok(resolution.decision);
                }
                _ => {}
            }
        }
        resolution.into_tool_result(ctx.review_context.turn().model_info())
    }

    async fn request_user_approval(
        &self,
        action: &ApprovalAction,
        ctx: &ApprovalContext,
    ) -> ReviewDecision {
        match action {
            ApprovalAction::ExecCommand {
                environment_id,
                command,
                cwd,
                additional_permissions,
                justification,
                proposed_execpolicy_amendment,
                ..
            } => {
                let tool_name = "unified_exec";
                let reason = ctx
                    .retry_reason
                    .clone()
                    .or_else(|| ctx.approval_reason.clone())
                    .or_else(|| justification.clone());
                let policy_fingerprint = ctx
                    .review_context
                    .environments()
                    .turn_environments()
                    .find(|environment| environment.selection.environment_id == *environment_id)
                    .and_then(|environment| environment.config().exec_policy.as_ref())
                    .map(codex_execpolicy::RequirementsExecPolicy::fingerprint);
                let cache_keys = action
                    .cache_keys()
                    .into_iter()
                    .map(|key| (key, &policy_fingerprint))
                    .collect();
                with_cached_approval(&self.services, tool_name, cache_keys, || async {
                    self.request_command_approval(
                        ctx.review_context.turn(),
                        ExecApprovalKind::Command,
                        ctx.review_context.model_context(),
                        ctx.call_id.clone(),
                        /*approval_id*/ None,
                        Some(environment_id.clone()),
                        command.clone(),
                        cwd.clone(),
                        reason,
                        ctx.network_approval_context.clone(),
                        proposed_execpolicy_amendment.clone(),
                        additional_permissions.clone(),
                        /*available_decisions*/ None,
                        /*plugin_attribution_override*/ None,
                    )
                    .await
                })
                .await
            }
            ApprovalAction::WriteStdin {
                id,
                approval_id,
                environment_id,
                process_id,
                input,
                cwd,
                additional_permissions,
                ..
            } => {
                self.request_command_approval(
                    ctx.review_context.turn(),
                    ExecApprovalKind::WriteStdin,
                    ctx.review_context.model_context(),
                    id.clone(),
                    Some(approval_id.clone()),
                    Some(environment_id.clone()),
                    vec![
                        "write_stdin".to_string(),
                        "--session-id".to_string(),
                        process_id.to_string(),
                        input.clone(),
                    ],
                    cwd.clone(),
                    ctx.approval_reason.clone(),
                    /*network_approval_context*/ None,
                    /*proposed_execpolicy_amendment*/ None,
                    additional_permissions.clone(),
                    Some(vec![ReviewDecision::Approved, ReviewDecision::Abort]),
                    /*plugin_attribution_override*/ None,
                )
                .await
            }
            #[cfg(unix)]
            ApprovalAction::Execve {
                approval_id,
                environment_id,
                command,
                cwd,
                additional_permissions,
                ..
            } => {
                self.request_command_approval(
                    ctx.review_context.turn(),
                    ExecApprovalKind::Command,
                    ctx.review_context.model_context(),
                    ctx.call_id.clone(),
                    Some(approval_id.clone()),
                    Some(environment_id.clone()),
                    command.clone(),
                    cwd.clone().into(),
                    /*reason*/ None,
                    /*network_approval_context*/ None,
                    /*proposed_execpolicy_amendment*/ None,
                    additional_permissions.clone(),
                    Some(vec![ReviewDecision::Approved, ReviewDecision::Abort]),
                    /*plugin_attribution_override*/ None,
                )
                .await
            }
            ApprovalAction::ApplyPatch {
                changes,
                permissions_preapproved,
                ..
            } => {
                let reason = ctx
                    .retry_reason
                    .clone()
                    .or_else(|| ctx.approval_reason.clone());
                if *permissions_preapproved && reason.is_none() {
                    return ReviewDecision::Approved;
                }
                if reason.is_some() {
                    return self
                        .request_patch_approval(
                            ctx.review_context.turn(),
                            ctx.call_id.clone(),
                            changes.as_ref().clone(),
                            reason,
                            /*grant_root*/ None,
                        )
                        .await;
                }
                with_cached_approval(
                    &self.services,
                    "apply_patch",
                    action.cache_keys(),
                    || async {
                        self.request_patch_approval(
                            ctx.review_context.turn(),
                            ctx.call_id.clone(),
                            changes.as_ref().clone(),
                            /*reason*/ None,
                            /*grant_root*/ None,
                        )
                        .await
                    },
                )
                .await
            }
            ApprovalAction::McpToolCall { .. } => {
                request_mcp_tool_user_approval(
                    self,
                    ctx.review_context.turn(),
                    &ctx.call_id,
                    action,
                )
                .await
            }
            ApprovalAction::NetworkAccess {
                environment_id,
                command,
                cwd,
                ..
            } => {
                self.request_command_approval(
                    ctx.review_context.turn(),
                    ExecApprovalKind::Command,
                    ctx.review_context.model_context(),
                    ctx.call_id.clone(),
                    /*approval_id*/ None,
                    Some(environment_id.clone()),
                    command.clone(),
                    cwd.clone().into(),
                    ctx.approval_reason.clone(),
                    ctx.network_approval_context.clone(),
                    /*proposed_execpolicy_amendment*/ None,
                    /*additional_permissions*/ None,
                    /*available_decisions*/ None,
                    /*plugin_attribution_override*/ None,
                )
                .await
            }
            ApprovalAction::RequestPermissions { .. } => {
                unreachable!("permission requests are handled by the session event flow")
            }
        }
    }
}

fn record_resolution(ctx: &ApprovalContext, resolution: &ApprovalResolution) {
    let source = match resolution.source {
        ApprovalResolutionSource::Hook => ToolDecisionSource::Config,
        ApprovalResolutionSource::User => ToolDecisionSource::User,
    };
    ctx.review_context.turn().session_telemetry.tool_decision(
        &ctx.tool_name,
        &ctx.call_id,
        &resolution.decision,
        Some(source),
    );
}

#[cfg(all(test, unix))]
#[path = "approvals_tests.rs"]
mod tests;
