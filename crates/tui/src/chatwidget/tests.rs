//! Exercises `ChatWidget` event handling and rendering invariants.
//!
//! These tests cover both app-server-native inputs and focused widget helpers. Many assertions are
//! snapshot-based so that layout regressions and status/header changes show up as stable,
//! reviewable diffs.

pub(super) use super::*;
pub(super) use crate::app_command::AppCommand as Op;
pub(super) use crate::app_event::AppEvent;
pub(super) use crate::app_event::ExitMode;
pub(super) use crate::app_event_sender::AppEventSender;
pub(super) use crate::approval_events::ApplyPatchApprovalRequestEvent;
pub(super) use crate::approval_events::ExecApprovalRequestEvent;
pub(super) use crate::bottom_pane::LocalImageAttachment;
pub(super) use crate::bottom_pane::MentionBinding;
pub(super) use crate::bottom_pane::QueuedInputAction;
pub(super) use crate::diff_model::FileChange;
pub(super) use crate::history_cell::UserHistoryCell;
pub(super) use crate::legacy_core::config::Config;
pub(super) use crate::legacy_core::config::ConfigBuilder;
pub(super) use crate::model_catalog::ModelCatalog;
pub(super) use crate::test_backend::VT100Backend;
pub(super) use crate::test_support::PathBufExt;
pub(super) use crate::test_support::test_path_buf;
pub(super) use crate::test_support::test_path_display;
pub(super) use crate::token_usage::TokenUsage;
pub(super) use crate::token_usage::TokenUsageInfo;
pub(super) use crate::tui::FrameRequester;
pub(super) use assert_matches::assert_matches;
pub(super) use genex_app_server_protocol::AddCreditsNudgeCreditType;
pub(super) use genex_app_server_protocol::AddCreditsNudgeEmailStatus;
pub(super) use genex_app_server_protocol::AdditionalFileSystemPermissions as AppServerAdditionalFileSystemPermissions;
pub(super) use genex_app_server_protocol::AdditionalNetworkPermissions as AppServerAdditionalNetworkPermissions;
pub(super) use genex_app_server_protocol::AdditionalPermissionProfile as AppServerAdditionalPermissionProfile;
pub(super) use genex_app_server_protocol::AppSummary;
pub(super) use genex_app_server_protocol::AutoReviewDecisionSource as AppServerGuardianApprovalReviewDecisionSource;
pub(super) use genex_app_server_protocol::GenexErrorInfo;
pub(super) use genex_app_server_protocol::CollabAgentState as AppServerCollabAgentState;
pub(super) use genex_app_server_protocol::CollabAgentStatus as AppServerCollabAgentStatus;
pub(super) use genex_app_server_protocol::CollabAgentTool as AppServerCollabAgentTool;
pub(super) use genex_app_server_protocol::CollabAgentToolCallStatus as AppServerCollabAgentToolCallStatus;
pub(super) use genex_app_server_protocol::CommandAction as AppServerCommandAction;
pub(super) use genex_app_server_protocol::CommandExecutionRequestApprovalParams as AppServerCommandExecutionRequestApprovalParams;
pub(super) use genex_app_server_protocol::CommandExecutionSource as ExecCommandSource;
pub(super) use genex_app_server_protocol::CommandExecutionSource as AppServerCommandExecutionSource;
pub(super) use genex_app_server_protocol::CommandExecutionStatus as AppServerCommandExecutionStatus;
pub(super) use genex_app_server_protocol::ConfigWarningNotification;
pub(super) use genex_app_server_protocol::CreditsSnapshot;
pub(super) use genex_app_server_protocol::ErrorNotification;
pub(super) use genex_app_server_protocol::ExecPolicyAmendment;
pub(super) use genex_app_server_protocol::FileUpdateChange;
pub(super) use genex_app_server_protocol::GuardianApprovalReview;
pub(super) use genex_app_server_protocol::GuardianApprovalReviewAction as AppServerGuardianApprovalReviewAction;
pub(super) use genex_app_server_protocol::GuardianApprovalReviewStatus;
pub(super) use genex_app_server_protocol::GuardianCommandSource as AppServerGuardianCommandSource;
pub(super) use genex_app_server_protocol::GuardianRiskLevel as AppServerGuardianRiskLevel;
pub(super) use genex_app_server_protocol::GuardianUserAuthorization as AppServerGuardianUserAuthorization;
pub(super) use genex_app_server_protocol::GuardianWarningNotification;
pub(super) use genex_app_server_protocol::HookCompletedNotification as AppServerHookCompletedNotification;
pub(super) use genex_app_server_protocol::HookEventName as AppServerHookEventName;
pub(super) use genex_app_server_protocol::HookExecutionMode as AppServerHookExecutionMode;
pub(super) use genex_app_server_protocol::HookHandlerType as AppServerHookHandlerType;
pub(super) use genex_app_server_protocol::HookOutputEntry as AppServerHookOutputEntry;
pub(super) use genex_app_server_protocol::HookOutputEntryKind as AppServerHookOutputEntryKind;
pub(super) use genex_app_server_protocol::HookRunStatus as AppServerHookRunStatus;
pub(super) use genex_app_server_protocol::HookRunSummary as AppServerHookRunSummary;
pub(super) use genex_app_server_protocol::HookScope as AppServerHookScope;
pub(super) use genex_app_server_protocol::HookStartedNotification as AppServerHookStartedNotification;
pub(super) use genex_app_server_protocol::ItemCompletedNotification;
pub(super) use genex_app_server_protocol::ItemGuardianApprovalReviewCompletedNotification;
pub(super) use genex_app_server_protocol::ItemGuardianApprovalReviewStartedNotification;
pub(super) use genex_app_server_protocol::ItemStartedNotification;
pub(super) use genex_app_server_protocol::MarketplaceAddResponse;
pub(super) use genex_app_server_protocol::MarketplaceInterface;
pub(super) use genex_app_server_protocol::MarketplaceUpgradeErrorInfo;
pub(super) use genex_app_server_protocol::MarketplaceUpgradeResponse;
pub(super) use genex_app_server_protocol::McpServerStartupState;
pub(super) use genex_app_server_protocol::McpServerStatusDetail;
pub(super) use genex_app_server_protocol::McpServerStatusUpdatedNotification;
pub(super) use genex_app_server_protocol::ModelSafetyBufferingUpdatedNotification;
pub(super) use genex_app_server_protocol::ModelVerification as AppServerModelVerification;
pub(super) use genex_app_server_protocol::ModelVerificationNotification;
pub(super) use genex_app_server_protocol::NonSteerableTurnKind;
pub(super) use genex_app_server_protocol::PatchApplyStatus as AppServerPatchApplyStatus;
pub(super) use genex_app_server_protocol::PatchChangeKind;
pub(super) use genex_app_server_protocol::PermissionsRequestApprovalParams as AppServerPermissionsRequestApprovalParams;
pub(super) use genex_app_server_protocol::PluginAuthPolicy;
pub(super) use genex_app_server_protocol::PluginDetail;
pub(super) use genex_app_server_protocol::PluginInstallPolicy;
pub(super) use genex_app_server_protocol::PluginInterface;
pub(super) use genex_app_server_protocol::PluginListResponse;
pub(super) use genex_app_server_protocol::PluginMarketplaceEntry;
pub(super) use genex_app_server_protocol::PluginReadResponse;
pub(super) use genex_app_server_protocol::PluginSource;
pub(super) use genex_app_server_protocol::PluginSummary;
pub(super) use genex_app_server_protocol::RateLimitReachedType;
pub(super) use genex_app_server_protocol::RateLimitSnapshot;
pub(super) use genex_app_server_protocol::RateLimitWindow;
pub(super) use genex_app_server_protocol::ReasoningSummaryTextDeltaNotification;
pub(super) use genex_app_server_protocol::ReviewTarget;
pub(super) use genex_app_server_protocol::ServerNotification;
pub(super) use genex_app_server_protocol::SkillMetadata;
pub(super) use genex_app_server_protocol::SkillSummary;
pub(super) use genex_app_server_protocol::ThreadClosedNotification;
pub(super) use genex_app_server_protocol::ThreadItem as AppServerThreadItem;
pub(super) use genex_app_server_protocol::ToolRequestUserInputOption;
pub(super) use genex_app_server_protocol::ToolRequestUserInputParams;
pub(super) use genex_app_server_protocol::ToolRequestUserInputQuestion;
pub(super) use genex_app_server_protocol::Turn as AppServerTurn;
pub(super) use genex_app_server_protocol::TurnCompletedNotification;
pub(super) use genex_app_server_protocol::TurnError as AppServerTurnError;
pub(super) use genex_app_server_protocol::TurnStartedNotification;
pub(super) use genex_app_server_protocol::TurnStatus as AppServerTurnStatus;
pub(super) use genex_app_server_protocol::UserInput;
pub(super) use genex_app_server_protocol::UserInput as AppServerUserInput;
pub(super) use genex_app_server_protocol::WarningNotification;
pub(super) use genex_app_server_protocol::WindowsSandboxSetupMode;
pub(super) use genex_config::ConfigLayerStack;
pub(super) use genex_config::Constrained;
pub(super) use genex_config::ConstraintError;
pub(super) use genex_config::RequirementSource;
pub(super) use genex_config::types::ApprovalsReviewer;
pub(super) use genex_config::types::Notifications;
pub(super) use genex_features::Feature;
pub(super) use genex_git_utils::CommitLogEntry;
pub(super) use genex_models_manager::test_support::construct_model_info_offline_for_tests;
pub(super) use genex_models_manager::test_support::get_model_offline_for_tests;
pub(super) use genex_otel::RuntimeMetricsSummary;
pub(super) use genex_otel::SessionTelemetry;
pub(super) use genex_protocol::ThreadId;
pub(super) use genex_protocol::account::PlanType;
pub(super) use genex_protocol::approvals::GuardianAssessmentAction;
pub(super) use genex_protocol::approvals::GuardianAssessmentDecisionSource;
pub(super) use genex_protocol::approvals::GuardianAssessmentEvent;
pub(super) use genex_protocol::approvals::GuardianAssessmentStatus;
pub(super) use genex_protocol::approvals::GuardianCommandSource;
pub(super) use genex_protocol::approvals::GuardianRiskLevel;
pub(super) use genex_protocol::approvals::GuardianUserAuthorization;
pub(super) use genex_protocol::config_types::CollaborationMode;
pub(super) use genex_protocol::config_types::ModeKind;
pub(super) use genex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
pub(super) use genex_protocol::config_types::ServiceTier;
pub(super) use genex_protocol::models::ActivePermissionProfile;
pub(super) use genex_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
pub(super) use genex_protocol::models::FileSystemPermissions;
pub(super) use genex_protocol::models::MessagePhase;
pub(super) use genex_protocol::models::NetworkPermissions;
pub(super) use genex_protocol::models::PermissionProfile;
pub(super) use genex_protocol::model_types::ModelInfo;
pub(super) use genex_protocol::model_types::ModelPreset;
pub(super) use genex_protocol::model_types::ModelsResponse;
pub(super) use genex_protocol::model_types::ReasoningEffortPreset;
pub(super) use genex_protocol::model_types::default_input_modalities;
pub(super) use genex_protocol::parse_command::ParsedCommand;
pub(super) use genex_protocol::plan_tool::PlanItemArg;
pub(super) use genex_protocol::plan_tool::StepStatus;
pub(super) use genex_protocol::plan_tool::UpdatePlanArgs;
pub(super) use genex_protocol::request_permissions::RequestPermissionProfile;
pub(super) use genex_protocol::user_input::TextElement;
pub(super) use genex_terminal_detection::Multiplexer;
pub(super) use genex_terminal_detection::TerminalInfo;
pub(super) use genex_terminal_detection::TerminalName;
pub(super) use genex_utils_absolute_path::AbsolutePathBuf;
pub(super) use genex_utils_approval_presets::builtin_approval_presets;
pub(super) use genex_utils_path_uri::LegacyAppPathString;
pub(super) use crossterm::event::KeyCode;
pub(super) use crossterm::event::KeyEvent;
pub(super) use crossterm::event::KeyModifiers;
pub(super) use serde_json::json;
#[cfg(target_os = "windows")]
pub(super) use serial_test::serial;
pub(super) use std::collections::HashMap;
pub(super) use std::path::PathBuf;
pub(super) use tempfile::NamedTempFile;
pub(super) use tempfile::tempdir;
pub(super) use tokio::sync::mpsc::error::TryRecvError;
pub(super) use tokio::sync::mpsc::unbounded_channel;
pub(super) use toml::Value as TomlValue;

fn next_goal_draft(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
    expected_thread_id: ThreadId,
) -> crate::goal_files::GoalDraft {
    loop {
        let event = rx.try_recv().expect("expected goal draft event");
        if let AppEvent::SetThreadGoalDraft {
            thread_id, draft, ..
        } = event
        {
            assert_eq!(thread_id, expected_thread_id);
            return draft;
        }
    }
}

mod app_server;
mod approval_requests;
#[path = "tests/collaboration_catalog_tests.rs"]
mod collaboration_catalog_tests;
#[path = "tests/compaction_tests.rs"]
mod compaction_tests;
#[path = "tests/completion_styling_tests.rs"]
mod completion_styling;
mod composer_submission;
#[path = "tests/computer_activity_tests.rs"]
mod computer_activity_tests;
mod exec_flow;
mod goal_menu;
mod goal_validation;
pub(crate) mod helpers;
mod history_replay;
mod mcp_startup;
#[path = "tests/permission_shortcuts_tests.rs"]
mod permission_shortcuts_tests;
mod permissions;
mod plan_mode;
#[path = "tests/plugin_catalog_tests.rs"]
mod plugin_catalog;
mod popups_and_settings;
#[path = "tests/rate_limit_recovery_tests.rs"]
mod rate_limit_recovery_tests;
#[path = "tests/reasoning_status_tests.rs"]
mod reasoning_status_tests;
mod review_mode;
#[path = "tests/session_model_selection_tests.rs"]
mod session_model_selection_tests;
mod side;
mod slash_commands;
#[path = "tests/sparkle_submission_tests.rs"]
mod sparkle_submission_tests;
mod status_and_layout;
mod status_command_tests;
mod status_surface_previews;
mod terminal_title;
mod usage;

pub(crate) use helpers::make_chatwidget_manual_with_sender;
pub(crate) use helpers::set_managed_auth;
pub(crate) use helpers::set_service_tier_test_catalog;
pub(super) use helpers::*;
