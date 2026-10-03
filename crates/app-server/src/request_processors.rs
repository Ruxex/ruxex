use crate::bespoke_event_handling::apply_bespoke_event_handling;
use crate::command_exec::CommandExecManager;
use crate::command_exec::StartCommandExecParams;
use crate::config_manager::ConfigManager;
use crate::error_code::INPUT_TOO_LARGE_ERROR_CODE;
use crate::error_code::invalid_params;
use crate::models::supported_models;
use crate::outgoing_message::ConnectionId;
use crate::outgoing_message::ConnectionRequestId;
use crate::outgoing_message::OutgoingMessageSender;
use crate::outgoing_message::RequestContext;
use crate::outgoing_message::ThreadScopedOutgoingMessageSender;
use crate::skills_watcher::SkillsWatcher;
use crate::thread_status::ThreadWatchManager;
use crate::thread_status::resolve_thread_status;
use chrono::Duration as ChronoDuration;
use chrono::SecondsFormat;
use genex_app_server_protocol::AccountLoginCompletedNotification;
use genex_app_server_protocol::AccountUpdatedNotification;
use genex_app_server_protocol::AdditionalContextEntry;
use genex_app_server_protocol::AdditionalContextKind;
use genex_app_server_protocol::AskForApproval;
use genex_app_server_protocol::AuthMode;
use genex_app_server_protocol::CancelLoginAccountParams;
use genex_app_server_protocol::CancelLoginAccountResponse;
use genex_app_server_protocol::CancelLoginAccountStatus;
use genex_app_server_protocol::ClientInfo;
use genex_app_server_protocol::ClientResponsePayload;
use genex_app_server_protocol::GenexErrorInfo;
use genex_app_server_protocol::CollaborationModeListParams;
use genex_app_server_protocol::CollaborationModeListResponse;
use genex_app_server_protocol::CommandExecParams;
use genex_app_server_protocol::CommandExecResizeParams;
use genex_app_server_protocol::CommandExecTerminateParams;
use genex_app_server_protocol::CommandExecWriteParams;
use genex_app_server_protocol::ConfigWarningNotification;
use genex_app_server_protocol::ConversationGitInfo;
use genex_app_server_protocol::ConversationSummary;
use genex_app_server_protocol::DeprecationNoticeNotification;
use genex_app_server_protocol::DynamicToolFunctionSpec;
use genex_app_server_protocol::DynamicToolNamespaceTool;
use genex_app_server_protocol::DynamicToolSpec;
use genex_app_server_protocol::EnvironmentInfoParams;
use genex_app_server_protocol::EnvironmentInfoResponse;
use genex_app_server_protocol::EnvironmentShellInfo;
use genex_app_server_protocol::EnvironmentStatusKind;
use genex_app_server_protocol::EnvironmentStatusParams;
use genex_app_server_protocol::EnvironmentStatusResponse;
use genex_app_server_protocol::GetAccountParams;
use genex_app_server_protocol::GetAuthStatusParams;
use genex_app_server_protocol::GetAuthStatusResponse;
use genex_app_server_protocol::GetConversationSummaryParams;
use genex_app_server_protocol::GetConversationSummaryResponse;
use genex_app_server_protocol::GitDiffToRemoteParams;
use genex_app_server_protocol::GitDiffToRemoteResponse;
use genex_app_server_protocol::GitInfo as ApiGitInfo;
use genex_app_server_protocol::HookHandlerMetadata;
use genex_app_server_protocol::HookMetadata;
use genex_app_server_protocol::HooksListParams;
use genex_app_server_protocol::HooksListResponse;
use genex_app_server_protocol::InitializeParams;
use genex_app_server_protocol::InitializeResponse;
use genex_app_server_protocol::JSONRPCErrorError;
use genex_app_server_protocol::ListMcpServerStatusParams;
use genex_app_server_protocol::ListMcpServerStatusResponse;
use genex_app_server_protocol::LoginAccountParams;
use genex_app_server_protocol::LoginAccountResponse;
use genex_app_server_protocol::LoginApiKeyParams;
use genex_app_server_protocol::LogoutAccountResponse;
use genex_app_server_protocol::MarketplaceAddParams;
use genex_app_server_protocol::MarketplaceAddResponse;
use genex_app_server_protocol::MarketplaceRemoveParams;
use genex_app_server_protocol::MarketplaceRemoveResponse;
use genex_app_server_protocol::MarketplaceUpgradeErrorInfo;
use genex_app_server_protocol::MarketplaceUpgradeParams;
use genex_app_server_protocol::MarketplaceUpgradeResponse;
use genex_app_server_protocol::McpResourceReadParams;
use genex_app_server_protocol::McpResourceReadResponse;
use genex_app_server_protocol::McpServerOauthClientRegistration;
use genex_app_server_protocol::McpServerOauthLoginCompletedNotification;
use genex_app_server_protocol::McpServerOauthLoginParams;
use genex_app_server_protocol::McpServerOauthLoginResponse;
use genex_app_server_protocol::McpServerRefreshParams;
use genex_app_server_protocol::McpServerRefreshResponse;
use genex_app_server_protocol::McpServerStatus;
use genex_app_server_protocol::McpServerStatusDetail;
use genex_app_server_protocol::McpServerToolCallParams;
use genex_app_server_protocol::McpServerToolCallResponse;
use genex_app_server_protocol::MockExperimentalMethodParams;
use genex_app_server_protocol::MockExperimentalMethodResponse;
use genex_app_server_protocol::ModelListParams;
use genex_app_server_protocol::ModelListResponse;
use genex_app_server_protocol::PermissionProfileListParams;
use genex_app_server_protocol::PermissionProfileListResponse;
use genex_app_server_protocol::PermissionProfileSummary;
use genex_app_server_protocol::PluginInterface;
use genex_app_server_protocol::RequestId;
use genex_app_server_protocol::ReviewDelivery as ApiReviewDelivery;
use genex_app_server_protocol::ReviewStartParams;
use genex_app_server_protocol::ReviewStartResponse;
use genex_app_server_protocol::ReviewTarget as ApiReviewTarget;
use genex_app_server_protocol::SandboxMode;
use genex_app_server_protocol::ServerNotification;
use genex_app_server_protocol::ServerRequestResolvedNotification;
use genex_app_server_protocol::SkillsConfigWriteParams;
use genex_app_server_protocol::SkillsConfigWriteResponse;
use genex_app_server_protocol::SkillsExtraRootsSetParams;
use genex_app_server_protocol::SkillsExtraRootsSetResponse;
use genex_app_server_protocol::SkillsListParams;
use genex_app_server_protocol::SkillsListResponse;
use genex_app_server_protocol::SortDirection;
use genex_app_server_protocol::Thread;
use genex_app_server_protocol::ThreadArchiveParams;
use genex_app_server_protocol::ThreadArchiveResponse;
use genex_app_server_protocol::ThreadArchivedNotification;
use genex_app_server_protocol::ThreadBackgroundTerminal;
use genex_app_server_protocol::ThreadBackgroundTerminalsCleanParams;
use genex_app_server_protocol::ThreadBackgroundTerminalsCleanResponse;
use genex_app_server_protocol::ThreadBackgroundTerminalsListParams;
use genex_app_server_protocol::ThreadBackgroundTerminalsListResponse;
use genex_app_server_protocol::ThreadBackgroundTerminalsTerminateParams;
use genex_app_server_protocol::ThreadBackgroundTerminalsTerminateResponse;
use genex_app_server_protocol::ThreadClosedNotification;
use genex_app_server_protocol::ThreadCompactStartParams;
use genex_app_server_protocol::ThreadCompactStartResponse;
use genex_app_server_protocol::ThreadDecrementElicitationParams;
use genex_app_server_protocol::ThreadDecrementElicitationResponse;
use genex_app_server_protocol::ThreadDeleteParams;
use genex_app_server_protocol::ThreadDeleteResponse;
use genex_app_server_protocol::ThreadDeletedNotification;
use genex_app_server_protocol::ThreadForkParams;
use genex_app_server_protocol::ThreadForkResponse;
use genex_app_server_protocol::ThreadGoal;
use genex_app_server_protocol::ThreadGoalClearParams;
use genex_app_server_protocol::ThreadGoalClearResponse;
use genex_app_server_protocol::ThreadGoalClearedNotification;
use genex_app_server_protocol::ThreadGoalGetParams;
use genex_app_server_protocol::ThreadGoalGetResponse;
use genex_app_server_protocol::ThreadGoalSetParams;
use genex_app_server_protocol::ThreadGoalSetResponse;
use genex_app_server_protocol::ThreadGoalStatus;
use genex_app_server_protocol::ThreadGoalUpdatedNotification;
use genex_app_server_protocol::ThreadHistoryBuilder;
#[cfg(test)]
use genex_app_server_protocol::ThreadHistoryMode;
use genex_app_server_protocol::ThreadIncrementElicitationParams;
use genex_app_server_protocol::ThreadIncrementElicitationResponse;
use genex_app_server_protocol::ThreadInjectItemsParams;
use genex_app_server_protocol::ThreadInjectItemsResponse;
use genex_app_server_protocol::ThreadItem;
use genex_app_server_protocol::ThreadItemEntry;
use genex_app_server_protocol::ThreadItemsListParams;
use genex_app_server_protocol::ThreadItemsListResponse;
use genex_app_server_protocol::ThreadListCwdFilter;
use genex_app_server_protocol::ThreadListParams;
use genex_app_server_protocol::ThreadListResponse;
use genex_app_server_protocol::ThreadLoadedListParams;
use genex_app_server_protocol::ThreadLoadedListResponse;
use genex_app_server_protocol::ThreadMetadataGitInfoUpdateParams;
use genex_app_server_protocol::ThreadMetadataUpdateParams;
use genex_app_server_protocol::ThreadMetadataUpdateResponse;
use genex_app_server_protocol::ThreadNameUpdatedNotification;
use genex_app_server_protocol::ThreadProjectUpdatedNotification;
use genex_app_server_protocol::ThreadReadParams;
use genex_app_server_protocol::ThreadReadResponse;
use genex_app_server_protocol::ThreadResumeInitialTurnsPageParams;
use genex_app_server_protocol::ThreadResumeParams;
use genex_app_server_protocol::ThreadResumeResponse;
use genex_app_server_protocol::ThreadSearchOccurrence;
use genex_app_server_protocol::ThreadSearchOccurrencesParams;
use genex_app_server_protocol::ThreadSearchOccurrencesResponse;
use genex_app_server_protocol::ThreadSearchParams;
use genex_app_server_protocol::ThreadSearchResponse;
use genex_app_server_protocol::ThreadSearchResult;
use genex_app_server_protocol::ThreadSearchSortKey;
use genex_app_server_protocol::ThreadSearchTextRange;
use genex_app_server_protocol::ThreadSetNameParams;
use genex_app_server_protocol::ThreadSetNameResponse;
use genex_app_server_protocol::ThreadSettings;
use genex_app_server_protocol::ThreadSettingsUpdateParams;
use genex_app_server_protocol::ThreadSettingsUpdateResponse;
use genex_app_server_protocol::ThreadShellCommandParams;
use genex_app_server_protocol::ThreadShellCommandResponse;
use genex_app_server_protocol::ThreadSortKey;
use genex_app_server_protocol::ThreadSourceKind;
use genex_app_server_protocol::ThreadStartParams;
use genex_app_server_protocol::ThreadStartResponse;
use genex_app_server_protocol::ThreadStartedNotification;
use genex_app_server_protocol::ThreadStatus;
use genex_app_server_protocol::ThreadTimelineListParams;
use genex_app_server_protocol::ThreadTimelineListResponse;
use genex_app_server_protocol::ThreadTurnsListParams;
use genex_app_server_protocol::ThreadTurnsListResponse;
use genex_app_server_protocol::ThreadUnarchiveParams;
use genex_app_server_protocol::ThreadUnarchiveResponse;
use genex_app_server_protocol::ThreadUnarchivedNotification;
use genex_app_server_protocol::ThreadUnsubscribeParams;
use genex_app_server_protocol::ThreadUnsubscribeResponse;
use genex_app_server_protocol::ThreadUnsubscribeStatus;
use genex_app_server_protocol::Turn;
use genex_app_server_protocol::TurnEnvironmentParams;
use genex_app_server_protocol::TurnError;
use genex_app_server_protocol::TurnInterruptParams;
use genex_app_server_protocol::TurnInterruptResponse;
use genex_app_server_protocol::TurnItemsView;
use genex_app_server_protocol::TurnSettingsUpdateParams;
use genex_app_server_protocol::TurnSettingsUpdateResponse;
use genex_app_server_protocol::TurnSettingsUpdateStatus;
use genex_app_server_protocol::TurnStartParams;
use genex_app_server_protocol::TurnStartResponse;
use genex_app_server_protocol::TurnStatus;
use genex_app_server_protocol::TurnSteerParams;
use genex_app_server_protocol::TurnSteerResponse;
use genex_app_server_protocol::UserInput as V2UserInput;
use genex_app_server_protocol::WindowsSandboxReadiness;
use genex_app_server_protocol::WindowsSandboxReadinessResponse;
use genex_app_server_protocol::WindowsSandboxSetupCompletedNotification;
use genex_app_server_protocol::WindowsSandboxSetupMode;
use genex_app_server_protocol::WindowsSandboxSetupStartParams;
use genex_app_server_protocol::WindowsSandboxSetupStartResponse;
use genex_arg0::Arg0DispatchPaths;
use genex_config::ConfigLayerStack;
use genex_config::loader::project_trust_key;
use genex_config::types::McpServerTransportConfig;
use genex_core::GenexThread;
use genex_core::GenexThreadSettingsOverrides;
use genex_core::ForkSnapshot;
use genex_core::NewThread;
use genex_core::NotSubmittedReason;
#[cfg(test)]
use genex_core::SessionMeta;
use genex_core::StartThreadOptions;
use genex_core::SteerSubmission;
use genex_core::ThreadConfigSnapshot;
use genex_core::ThreadManager;
use genex_core::TurnInput;
use genex_core::TurnInputRequest;
use genex_core::TurnInputSubmission;
use genex_core::TurnStartOptions;
use genex_core::config::Config;
use genex_core::config::ConfigOverrides;
use genex_core::config::NetworkProxyAuditMetadata;
use genex_core::config::edit::ConfigEdit;
use genex_core::config::edit::ConfigEditsBuilder;
use genex_core::exec::ExecCapturePolicy;
use genex_core::exec::ExecExpiration;
use genex_core::exec::ExecParams;
use genex_core::exec_env::create_env;
use genex_core::path_utils;
#[cfg(test)]
use genex_core::read_head_for_summary;
use genex_core::sandboxing::SandboxPermissions;
use genex_core::truncate_rollout_after_turn_id;
use genex_core::truncate_rollout_before_turn_id;
use genex_core::windows_sandbox::WindowsSandboxLevelExt;
use genex_core::windows_sandbox::WindowsSandboxSetupMode as CoreWindowsSandboxSetupMode;
use genex_core::windows_sandbox::WindowsSandboxSetupRequest;
use genex_core::windows_sandbox::sandbox_setup_is_complete;
use genex_core_plugins::marketplace_add::MarketplaceAddError;
use genex_core_plugins::marketplace_add::MarketplaceAddRequest;
use genex_core_plugins::marketplace_add::add_marketplace as add_marketplace_to_genex_home;
use genex_core_plugins::marketplace_remove::MarketplaceRemoveError;
use genex_core_plugins::marketplace_remove::MarketplaceRemoveRequest as CoreMarketplaceRemoveRequest;
use genex_core_plugins::marketplace_remove::remove_marketplace;
use genex_exec_server::EnvironmentManager;
use genex_exec_server::EnvironmentObservedStatus;
use genex_exec_server::LOCAL_ENVIRONMENT_ID;
use genex_exec_server::LOCAL_FS;
use genex_features::Feature;
use genex_git_utils::git_diff_to_remote;
use genex_git_utils::resolve_root_git_project_for_trust;
use genex_login::AuthManager;
use genex_login::GenexAuth;
use genex_login::login_with_api_key;
use genex_login::login_with_bedrock_api_key;
use genex_mcp::McpRuntimeContext;
use genex_mcp::McpServerStatusSnapshot;
use genex_mcp::McpSnapshotDetail;
use genex_mcp::collect_mcp_server_status_snapshot_with_detail;
use genex_mcp::discover_supported_scopes;
use genex_mcp::read_mcp_resource as read_mcp_resource_without_thread;
use genex_mcp::resolve_oauth_scopes;
use genex_models_manager::collaboration_mode_presets::builtin_collaboration_mode_presets;
use genex_protocol::ThreadId;
use genex_protocol::config_types::CollaborationMode;
use genex_protocol::config_types::ForcedLoginMethod;
use genex_protocol::config_types::ReasoningSummary;
use genex_protocol::config_types::TrustLevel;
use genex_protocol::config_types::WindowsSandboxLevel;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result as GenexResult;
#[cfg(test)]
use genex_protocol::items::TurnItem;
use genex_protocol::model_types::ReasoningEffort;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::AgentStatus;
use genex_protocol::protocol::EnvironmentConfigState;
use genex_protocol::protocol::EventMsg;
#[cfg(test)]
use genex_protocol::protocol::GitInfo as CoreGitInfo;
use genex_protocol::protocol::McpAuthStatus as CoreMcpAuthStatus;
use genex_protocol::protocol::Op;
use genex_protocol::protocol::ReviewDelivery as CoreReviewDelivery;
use genex_protocol::protocol::ReviewRequest;
use genex_protocol::protocol::ReviewTarget as CoreReviewTarget;
use genex_protocol::protocol::SessionConfiguredEvent;
#[cfg(test)]
use genex_protocol::protocol::SessionMetaLine;
use genex_protocol::protocol::TurnEnvironmentSelection;
use genex_protocol::protocol::TurnEnvironmentSelections;
use genex_protocol::protocol::W3cTraceContext;
use genex_protocol::protocol::strip_user_message_prefix;
use genex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;
use genex_protocol::user_input::UserInput as CoreInputItem;
use genex_rmcp_client::McpOAuthClientRegistration;
use genex_rmcp_client::StreamableHttpRedirectMode;
use genex_rmcp_client::perform_oauth_login_return_url;
use genex_rollout::InitialHistory;
use genex_rollout::ResumedHistory;
use genex_rollout::RolloutItem;
use genex_rollout::is_persisted_rollout_item;
use genex_rollout::state_db::StateDbHandle;
use genex_rollout::state_db::reconcile_rollout;
use genex_state::ThreadMetadata;
use genex_state::log_db::LogDbLayer;
use genex_thread_store::ArchiveThreadParams as StoreArchiveThreadParams;
use genex_thread_store::ArchiveThreadsParams as StoreArchiveThreadsParams;
use genex_thread_store::ClearableField as StoreClearableField;
use genex_thread_store::DeleteThreadsParams as StoreDeleteThreadsParams;
use genex_thread_store::GitInfoPatch as StoreGitInfoPatch;
use genex_thread_store::ItemSortKey as StoreItemSortKey;
use genex_thread_store::ListItemsParams as StoreListItemsParams;
use genex_thread_store::ListThreadsParams as StoreListThreadsParams;
use genex_thread_store::ListTimelineParams as StoreListTimelineParams;
use genex_thread_store::ListTurnsParams as StoreListTurnsParams;
use genex_thread_store::LoadThreadHistoryParams as StoreLoadThreadHistoryParams;
use genex_thread_store::LocalThreadStore;
use genex_thread_store::ReadThreadByRolloutPathParams as StoreReadThreadByRolloutPathParams;
use genex_thread_store::ReadThreadParams as StoreReadThreadParams;
use genex_thread_store::SearchThreadOccurrencesParams as StoreSearchThreadOccurrencesParams;
use genex_thread_store::SearchThreadsParams as StoreSearchThreadsParams;
use genex_thread_store::SortDirection as StoreSortDirection;
use genex_thread_store::StoredThread;
use genex_thread_store::StoredTurn;
use genex_thread_store::StoredTurnItemsView;
use genex_thread_store::StoredTurnStatus;
use genex_thread_store::ThreadMetadataPatch as StoreThreadMetadataPatch;
use genex_thread_store::ThreadRelationFilter as StoreThreadRelationFilter;
use genex_thread_store::ThreadSortKey as StoreThreadSortKey;
use genex_thread_store::ThreadStore;
use genex_thread_store::ThreadStoreError;
use genex_utils_absolute_path::AbsolutePathBuf;
use genex_utils_pty::DEFAULT_OUTPUT_BYTES_CAP;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
#[cfg(test)]
use std::io::Error as IoError;
use std::path::Path;
use std::path::PathBuf;
use std::result::Result;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::sync::Semaphore;
use tokio::sync::SemaphorePermit;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use toml::Value as TomlValue;
use tracing::Instrument;
use tracing::error;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

#[cfg(test)]
use genex_app_server_protocol::ServerRequest;

mod account_processor;
mod bedrock_auth;
mod catalog_processor;
mod command_exec_processor;
mod config_processor;
mod diagnostics;
mod environment_processor;
mod fs_processor;
mod git_processor;
mod initialize_processor;
mod marketplace_processor;
mod mcp_processor;
mod persisted_resume_settings;
mod plugins;
mod process_exec_processor;
mod projects;
mod rollout;
mod search;
mod thread_attachments;
mod thread_enrichment;
mod thread_fork_goal;
mod thread_input;
mod thread_processor;
mod thread_queue_processor;
mod thread_sections;
mod token_usage_replay;
mod turn_processor;
mod windows_sandbox_processor;

pub(crate) use account_processor::AccountRequestProcessor;
pub(crate) use catalog_processor::CatalogRequestProcessor;
pub(crate) use command_exec_processor::CommandExecRequestProcessor;
pub(crate) use config_processor::ConfigRequestProcessor;
pub(crate) use diagnostics::read_server_diagnostics;
pub(crate) use environment_processor::EnvironmentRequestProcessor;
pub(crate) use fs_processor::FsRequestProcessor;
pub(crate) use git_processor::GitRequestProcessor;
pub(crate) use initialize_processor::InitializeRequestProcessor;
pub(crate) use marketplace_processor::MarketplaceRequestProcessor;
pub(crate) use mcp_processor::McpRequestProcessor;
pub(crate) use plugins::PluginRequestProcessor;
pub(crate) use process_exec_processor::ProcessExecRequestProcessor;
pub(crate) use projects::ProjectRequestProcessor;
pub(crate) use search::SearchRequestProcessor;
pub(crate) use thread_goal_processor::ThreadGoalRequestProcessor;
pub(crate) use thread_processor::ThreadRequestProcessor;
pub(crate) use thread_processor::ThreadResumeTarget;
pub(crate) use thread_queue_processor::ThreadQueueRequestProcessor;
pub(crate) use turn_processor::TurnRequestProcessor;
pub(crate) use windows_sandbox_processor::WindowsSandboxRequestProcessor;

use crate::error_code::internal_error;
use crate::error_code::invalid_request;
use crate::filters::compute_source_filters;
use crate::filters::source_kind_matches;
use crate::thread_state::ConnectionCapabilities;
use crate::thread_state::ThreadListenerCommand;
use crate::thread_state::ThreadState;
use crate::thread_state::ThreadStateManager;
use token_usage_replay::restored_token_usage_turn_id;
use token_usage_replay::send_thread_token_usage_update_to_connection;

pub(crate) fn apply_live_thread_settings(
    thread: &mut Thread,
    config_snapshot: &ThreadConfigSnapshot,
) {
    thread.model = Some(config_snapshot.model.clone());
    thread.reasoning_effort = config_snapshot.reasoning_effort.clone();
    thread.environments = Some(
        config_snapshot
            .environment_selections()
            .iter()
            .map(Into::into)
            .collect(),
    );
}

fn resolve_request_cwd(cwd: Option<PathBuf>) -> Result<Option<AbsolutePathBuf>, JSONRPCErrorError> {
    cwd.map(|cwd| {
        AbsolutePathBuf::relative_to_current_dir(path_utils::normalize_for_native_workdir(cwd))
            .map_err(|err| invalid_request(format!("invalid cwd: {err}")))
    })
    .transpose()
}

fn resolve_turn_environment_selections(
    thread_manager: &ThreadManager,
    environments: Option<Vec<TurnEnvironmentParams>>,
) -> Result<Option<Vec<TurnEnvironmentSelection>>, JSONRPCErrorError> {
    let Some(environments) = environments else {
        return Ok(None);
    };
    let mut selections = Vec::with_capacity(environments.len());
    for environment in environments {
        let environment_id = environment.environment_id;
        let cwd = environment
            .cwd
            .to_inferred_path_uri()
            .ok_or_else(|| {
                invalid_request(format!(
                    "invalid cwd for environment `{environment_id}`: path `{}` does not use absolute POSIX or Windows path syntax",
                    environment.cwd
                ))
            })?;
        let workspace_roots = environment
            .runtime_workspace_roots
            .map(|roots| {
                let mut resolved_roots = Vec::new();
                for root in roots {
                    let root = root.to_inferred_path_uri().ok_or_else(|| {
                        invalid_request(format!(
                            "invalid runtime workspace root for environment `{environment_id}`: path `{root}` does not use absolute POSIX or Windows path syntax"
                        ))
                    })?;
                    if !resolved_roots.contains(&root) {
                        resolved_roots.push(root);
                    }
                }
                Ok::<_, JSONRPCErrorError>(resolved_roots)
            })
            .transpose()?
            .unwrap_or_else(|| vec![cwd.clone()]);
        selections.push(TurnEnvironmentSelection {
            environment_id,
            cwd,
            workspace_roots,
            config: EnvironmentConfigState::FromThread,
        });
    }
    thread_manager
        .validate_environment_selections(&selections)
        .map_err(environment_selection_error)?;
    Ok(Some(selections))
}

fn resolve_runtime_workspace_roots(workspace_roots: Vec<AbsolutePathBuf>) -> Vec<AbsolutePathBuf> {
    let mut resolved_roots = Vec::new();
    for root in workspace_roots {
        if !resolved_roots.iter().any(|existing| existing == &root) {
            resolved_roots.push(root);
        }
    }
    resolved_roots
}

mod config_errors;
mod request_errors;
mod thread_delete;
mod thread_goal_processor;
mod thread_lifecycle;
mod thread_summary;

use self::config_errors::*;
use self::request_errors::*;
use self::thread_goal_processor::api_thread_goal_from_state;
use self::thread_lifecycle::*;
use self::thread_summary::*;

pub(crate) use self::thread_lifecycle::populate_thread_turns_from_history;
pub(crate) use self::thread_processor::thread_from_stored_thread;
#[cfg(test)]
pub(crate) use self::thread_summary::read_summary_from_rollout;
#[cfg(test)]
pub(crate) use self::thread_summary::summary_to_thread;
pub(crate) use self::thread_summary::thread_settings_from_config_snapshot;

pub(crate) fn build_legacy_api_turns_from_rollout_items(items: &[RolloutItem]) -> Vec<Turn> {
    let mut builder = ThreadHistoryBuilder::new();
    for item in items {
        if is_persisted_rollout_item(item) {
            builder.handle_rollout_item(item);
        }
    }
    builder.finish()
}
