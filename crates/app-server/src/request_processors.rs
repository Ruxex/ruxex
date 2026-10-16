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
use ruxex_app_server_protocol::AccountLoginCompletedNotification;
use ruxex_app_server_protocol::AccountUpdatedNotification;
use ruxex_app_server_protocol::AdditionalContextEntry;
use ruxex_app_server_protocol::AdditionalContextKind;
use ruxex_app_server_protocol::AppServerIdentity;
use ruxex_app_server_protocol::AskForApproval;
use ruxex_app_server_protocol::AuthMode;
use ruxex_app_server_protocol::CancelLoginAccountParams;
use ruxex_app_server_protocol::CancelLoginAccountResponse;
use ruxex_app_server_protocol::CancelLoginAccountStatus;
use ruxex_app_server_protocol::ClientInfo;
use ruxex_app_server_protocol::ClientResponsePayload;
use ruxex_app_server_protocol::CollaborationModeListParams;
use ruxex_app_server_protocol::CollaborationModeListResponse;
use ruxex_app_server_protocol::CommandExecParams;
use ruxex_app_server_protocol::CommandExecResizeParams;
use ruxex_app_server_protocol::CommandExecTerminateParams;
use ruxex_app_server_protocol::CommandExecWriteParams;
use ruxex_app_server_protocol::ConfigWarningNotification;
#[cfg(test)]
use ruxex_app_server_protocol::ConversationGitInfo;
#[cfg(test)]
use ruxex_app_server_protocol::ConversationSummary;
use ruxex_app_server_protocol::DeprecationNoticeNotification;
use ruxex_app_server_protocol::DynamicToolFunctionSpec;
use ruxex_app_server_protocol::DynamicToolNamespaceTool;
use ruxex_app_server_protocol::DynamicToolSpec;
use ruxex_app_server_protocol::EnvironmentInfoParams;
use ruxex_app_server_protocol::EnvironmentInfoResponse;
use ruxex_app_server_protocol::EnvironmentShellInfo;
use ruxex_app_server_protocol::EnvironmentStatusKind;
use ruxex_app_server_protocol::EnvironmentStatusParams;
use ruxex_app_server_protocol::EnvironmentStatusResponse;
use ruxex_app_server_protocol::GetAccountParams;
use ruxex_app_server_protocol::GitInfo as ApiGitInfo;
use ruxex_app_server_protocol::HookHandlerMetadata;
use ruxex_app_server_protocol::HookMetadata;
use ruxex_app_server_protocol::HooksListParams;
use ruxex_app_server_protocol::HooksListResponse;
use ruxex_app_server_protocol::InitializeParams;
use ruxex_app_server_protocol::InitializeResponse;
use ruxex_app_server_protocol::JSONRPCErrorError;
use ruxex_app_server_protocol::ListMcpServerStatusParams;
use ruxex_app_server_protocol::ListMcpServerStatusResponse;
use ruxex_app_server_protocol::LoginAccountParams;
use ruxex_app_server_protocol::LoginAccountResponse;
use ruxex_app_server_protocol::LogoutAccountResponse;
use ruxex_app_server_protocol::MarketplaceAddParams;
use ruxex_app_server_protocol::MarketplaceAddResponse;
use ruxex_app_server_protocol::MarketplaceRemoveParams;
use ruxex_app_server_protocol::MarketplaceRemoveResponse;
use ruxex_app_server_protocol::MarketplaceUpgradeErrorInfo;
use ruxex_app_server_protocol::MarketplaceUpgradeParams;
use ruxex_app_server_protocol::MarketplaceUpgradeResponse;
use ruxex_app_server_protocol::McpResourceReadParams;
use ruxex_app_server_protocol::McpResourceReadResponse;
use ruxex_app_server_protocol::McpServerOauthClientRegistration;
use ruxex_app_server_protocol::McpServerOauthLoginCompletedNotification;
use ruxex_app_server_protocol::McpServerOauthLoginParams;
use ruxex_app_server_protocol::McpServerOauthLoginResponse;
use ruxex_app_server_protocol::McpServerRefreshParams;
use ruxex_app_server_protocol::McpServerRefreshResponse;
use ruxex_app_server_protocol::McpServerStatus;
use ruxex_app_server_protocol::McpServerStatusDetail;
use ruxex_app_server_protocol::McpServerToolCallParams;
use ruxex_app_server_protocol::McpServerToolCallResponse;
use ruxex_app_server_protocol::ModelListParams;
use ruxex_app_server_protocol::ModelListResponse;
use ruxex_app_server_protocol::PermissionProfileListParams;
use ruxex_app_server_protocol::PermissionProfileListResponse;
use ruxex_app_server_protocol::PermissionProfileSummary;
use ruxex_app_server_protocol::PluginInterface;
use ruxex_app_server_protocol::RequestId;
use ruxex_app_server_protocol::ReviewDelivery as ApiReviewDelivery;
use ruxex_app_server_protocol::ReviewStartParams;
use ruxex_app_server_protocol::ReviewStartResponse;
use ruxex_app_server_protocol::ReviewTarget as ApiReviewTarget;
use ruxex_app_server_protocol::RuxexErrorInfo;
use ruxex_app_server_protocol::SandboxMode;
use ruxex_app_server_protocol::ServerNotification;
use ruxex_app_server_protocol::ServerRequestResolvedNotification;
use ruxex_app_server_protocol::SkillsConfigWriteParams;
use ruxex_app_server_protocol::SkillsConfigWriteResponse;
use ruxex_app_server_protocol::SkillsExtraRootsSetParams;
use ruxex_app_server_protocol::SkillsExtraRootsSetResponse;
use ruxex_app_server_protocol::SkillsListParams;
use ruxex_app_server_protocol::SkillsListResponse;
use ruxex_app_server_protocol::SortDirection;
use ruxex_app_server_protocol::Thread;
use ruxex_app_server_protocol::ThreadArchiveParams;
use ruxex_app_server_protocol::ThreadArchiveResponse;
use ruxex_app_server_protocol::ThreadArchivedNotification;
use ruxex_app_server_protocol::ThreadBackgroundTerminal;
use ruxex_app_server_protocol::ThreadBackgroundTerminalsCleanParams;
use ruxex_app_server_protocol::ThreadBackgroundTerminalsCleanResponse;
use ruxex_app_server_protocol::ThreadBackgroundTerminalsListParams;
use ruxex_app_server_protocol::ThreadBackgroundTerminalsListResponse;
use ruxex_app_server_protocol::ThreadBackgroundTerminalsTerminateParams;
use ruxex_app_server_protocol::ThreadBackgroundTerminalsTerminateResponse;
use ruxex_app_server_protocol::ThreadClosedNotification;
use ruxex_app_server_protocol::ThreadCompactStartParams;
use ruxex_app_server_protocol::ThreadCompactStartResponse;
use ruxex_app_server_protocol::ThreadDecrementElicitationParams;
use ruxex_app_server_protocol::ThreadDecrementElicitationResponse;
use ruxex_app_server_protocol::ThreadDeleteParams;
use ruxex_app_server_protocol::ThreadDeleteResponse;
use ruxex_app_server_protocol::ThreadDeletedNotification;
use ruxex_app_server_protocol::ThreadForkParams;
use ruxex_app_server_protocol::ThreadForkResponse;
use ruxex_app_server_protocol::ThreadGoal;
use ruxex_app_server_protocol::ThreadGoalClearParams;
use ruxex_app_server_protocol::ThreadGoalClearResponse;
use ruxex_app_server_protocol::ThreadGoalClearedNotification;
use ruxex_app_server_protocol::ThreadGoalGetParams;
use ruxex_app_server_protocol::ThreadGoalGetResponse;
use ruxex_app_server_protocol::ThreadGoalSetParams;
use ruxex_app_server_protocol::ThreadGoalSetResponse;
use ruxex_app_server_protocol::ThreadGoalStatus;
use ruxex_app_server_protocol::ThreadGoalUpdatedNotification;
use ruxex_app_server_protocol::ThreadHistoryBuilder;
use ruxex_app_server_protocol::ThreadIncrementElicitationParams;
use ruxex_app_server_protocol::ThreadIncrementElicitationResponse;
use ruxex_app_server_protocol::ThreadInjectItemsParams;
use ruxex_app_server_protocol::ThreadInjectItemsResponse;
use ruxex_app_server_protocol::ThreadItem;
use ruxex_app_server_protocol::ThreadItemEntry;
use ruxex_app_server_protocol::ThreadItemsListParams;
use ruxex_app_server_protocol::ThreadItemsListResponse;
use ruxex_app_server_protocol::ThreadListCwdFilter;
use ruxex_app_server_protocol::ThreadListParams;
use ruxex_app_server_protocol::ThreadListResponse;
use ruxex_app_server_protocol::ThreadLoadedListParams;
use ruxex_app_server_protocol::ThreadLoadedListResponse;
use ruxex_app_server_protocol::ThreadMetadataGitInfoUpdateParams;
use ruxex_app_server_protocol::ThreadMetadataUpdateParams;
use ruxex_app_server_protocol::ThreadMetadataUpdateResponse;
use ruxex_app_server_protocol::ThreadNameUpdatedNotification;
use ruxex_app_server_protocol::ThreadProjectUpdatedNotification;
use ruxex_app_server_protocol::ThreadReadParams;
use ruxex_app_server_protocol::ThreadReadResponse;
use ruxex_app_server_protocol::ThreadResumeInitialTurnsPageParams;
use ruxex_app_server_protocol::ThreadResumeParams;
use ruxex_app_server_protocol::ThreadResumeResponse;
use ruxex_app_server_protocol::ThreadSearchOccurrence;
use ruxex_app_server_protocol::ThreadSearchOccurrencesParams;
use ruxex_app_server_protocol::ThreadSearchOccurrencesResponse;
use ruxex_app_server_protocol::ThreadSearchParams;
use ruxex_app_server_protocol::ThreadSearchResponse;
use ruxex_app_server_protocol::ThreadSearchResult;
use ruxex_app_server_protocol::ThreadSearchSortKey;
use ruxex_app_server_protocol::ThreadSearchTextRange;
use ruxex_app_server_protocol::ThreadSetNameParams;
use ruxex_app_server_protocol::ThreadSetNameResponse;
use ruxex_app_server_protocol::ThreadSettings;
use ruxex_app_server_protocol::ThreadSettingsUpdateParams;
use ruxex_app_server_protocol::ThreadSettingsUpdateResponse;
use ruxex_app_server_protocol::ThreadShellCommandParams;
use ruxex_app_server_protocol::ThreadShellCommandResponse;
use ruxex_app_server_protocol::ThreadSortKey;
use ruxex_app_server_protocol::ThreadSourceKind;
use ruxex_app_server_protocol::ThreadStartParams;
use ruxex_app_server_protocol::ThreadStartResponse;
use ruxex_app_server_protocol::ThreadStartedNotification;
use ruxex_app_server_protocol::ThreadStatus;
use ruxex_app_server_protocol::ThreadTimelineListParams;
use ruxex_app_server_protocol::ThreadTimelineListResponse;
use ruxex_app_server_protocol::ThreadTurnsListParams;
use ruxex_app_server_protocol::ThreadTurnsListResponse;
use ruxex_app_server_protocol::ThreadUnarchiveParams;
use ruxex_app_server_protocol::ThreadUnarchiveResponse;
use ruxex_app_server_protocol::ThreadUnarchivedNotification;
use ruxex_app_server_protocol::ThreadUnsubscribeParams;
use ruxex_app_server_protocol::ThreadUnsubscribeResponse;
use ruxex_app_server_protocol::ThreadUnsubscribeStatus;
use ruxex_app_server_protocol::Turn;
use ruxex_app_server_protocol::TurnEnvironmentParams;
use ruxex_app_server_protocol::TurnError;
use ruxex_app_server_protocol::TurnInterruptParams;
use ruxex_app_server_protocol::TurnInterruptResponse;
use ruxex_app_server_protocol::TurnItemsView;
use ruxex_app_server_protocol::TurnSettingsUpdateParams;
use ruxex_app_server_protocol::TurnSettingsUpdateResponse;
use ruxex_app_server_protocol::TurnSettingsUpdateStatus;
use ruxex_app_server_protocol::TurnStartParams;
use ruxex_app_server_protocol::TurnStartResponse;
use ruxex_app_server_protocol::TurnStatus;
use ruxex_app_server_protocol::TurnSteerParams;
use ruxex_app_server_protocol::TurnSteerResponse;
use ruxex_app_server_protocol::UserInput as V2UserInput;
use ruxex_arg0::Arg0DispatchPaths;
use ruxex_config::ConfigLayerStack;
use ruxex_config::loader::project_trust_key;
use ruxex_config::types::McpServerTransportConfig;
use ruxex_core::ForkSnapshot;
use ruxex_core::NewThread;
use ruxex_core::NotSubmittedReason;
use ruxex_core::RuxexThread;
use ruxex_core::RuxexThreadSettingsOverrides;
#[cfg(test)]
use ruxex_core::SessionMeta;
use ruxex_core::StartThreadOptions;
use ruxex_core::SteerSubmission;
use ruxex_core::ThreadConfigSnapshot;
use ruxex_core::ThreadManager;
use ruxex_core::TurnInput;
use ruxex_core::TurnInputRequest;
use ruxex_core::TurnInputSubmission;
use ruxex_core::TurnStartOptions;
use ruxex_core::config::Config;
use ruxex_core::config::ConfigOverrides;
use ruxex_core::config::NetworkProxyAuditMetadata;
use ruxex_core::config::edit::ConfigEdit;
use ruxex_core::config::edit::ConfigEditsBuilder;
use ruxex_core::exec::ExecCapturePolicy;
use ruxex_core::exec::ExecExpiration;
use ruxex_core::exec::ExecParams;
use ruxex_core::exec_env::create_env;
use ruxex_core::path_utils;
use ruxex_core::sandboxing::SandboxPermissions;
use ruxex_core::truncate_rollout_after_turn_id;
use ruxex_core::truncate_rollout_before_turn_id;
use ruxex_core_plugins::marketplace_add::MarketplaceAddError;
use ruxex_core_plugins::marketplace_add::MarketplaceAddRequest;
use ruxex_core_plugins::marketplace_add::add_marketplace as add_marketplace_to_ruxex_home;
use ruxex_core_plugins::marketplace_remove::MarketplaceRemoveError;
use ruxex_core_plugins::marketplace_remove::MarketplaceRemoveRequest as CoreMarketplaceRemoveRequest;
use ruxex_core_plugins::marketplace_remove::remove_marketplace;
use ruxex_exec_server::EnvironmentManager;
use ruxex_exec_server::EnvironmentObservedStatus;
use ruxex_exec_server::LOCAL_ENVIRONMENT_ID;
use ruxex_exec_server::LOCAL_FS;
use ruxex_features::Feature;
use ruxex_git_utils::resolve_root_git_project_for_trust;
use ruxex_mcp::McpRuntimeContext;
use ruxex_mcp::McpServerStatusSnapshot;
use ruxex_mcp::McpSnapshotDetail;
use ruxex_mcp::collect_mcp_server_status_snapshot_with_detail;
use ruxex_mcp::discover_supported_scopes;
use ruxex_mcp::read_mcp_resource as read_mcp_resource_without_thread;
use ruxex_mcp::resolve_oauth_scopes;
use ruxex_models_manager::collaboration_mode_presets::builtin_collaboration_mode_presets;
use ruxex_protocol::ThreadId;
use ruxex_protocol::config_types::CollaborationMode;
use ruxex_protocol::config_types::ReasoningSummary;
use ruxex_protocol::config_types::TrustLevel;
use ruxex_protocol::error::Result as RuxexResult;
use ruxex_protocol::error::RuxexErr;
#[cfg(test)]
use ruxex_protocol::items::TurnItem;
use ruxex_protocol::model_types::ReasoningEffort;
use ruxex_protocol::models::ResponseItem;
use ruxex_protocol::protocol::AgentStatus;
use ruxex_protocol::protocol::EnvironmentConfigState;
use ruxex_protocol::protocol::EventMsg;
#[cfg(test)]
use ruxex_protocol::protocol::GitInfo as CoreGitInfo;
use ruxex_protocol::protocol::McpAuthStatus as CoreMcpAuthStatus;
use ruxex_protocol::protocol::Op;
use ruxex_protocol::protocol::ReviewDelivery as CoreReviewDelivery;
use ruxex_protocol::protocol::ReviewRequest;
use ruxex_protocol::protocol::ReviewTarget as CoreReviewTarget;
use ruxex_protocol::protocol::SessionConfiguredEvent;
use ruxex_protocol::protocol::TurnEnvironmentSelection;
use ruxex_protocol::protocol::TurnEnvironmentSelections;
use ruxex_protocol::protocol::W3cTraceContext;
use ruxex_protocol::protocol::strip_user_message_prefix;
use ruxex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;
use ruxex_protocol::user_input::UserInput as CoreInputItem;
use ruxex_rmcp_client::McpOAuthClientRegistration;
use ruxex_rmcp_client::StreamableHttpRedirectMode;
use ruxex_rmcp_client::perform_oauth_login_return_url;
use ruxex_rollout::InitialHistory;
use ruxex_rollout::ResumedHistory;
use ruxex_rollout::RolloutItem;
use ruxex_rollout::is_persisted_rollout_item;
use ruxex_rollout::state_db::StateDbHandle;
use ruxex_rollout::state_db::reconcile_rollout;
use ruxex_state::ThreadMetadata;
use ruxex_state::log_db::LogDbLayer;
use ruxex_thread_store::ArchiveThreadParams as StoreArchiveThreadParams;
use ruxex_thread_store::ArchiveThreadsParams as StoreArchiveThreadsParams;
use ruxex_thread_store::ClearableField as StoreClearableField;
use ruxex_thread_store::DeleteThreadsParams as StoreDeleteThreadsParams;
use ruxex_thread_store::GitInfoPatch as StoreGitInfoPatch;
use ruxex_thread_store::ItemSortKey as StoreItemSortKey;
use ruxex_thread_store::ListItemsParams as StoreListItemsParams;
use ruxex_thread_store::ListThreadsParams as StoreListThreadsParams;
use ruxex_thread_store::ListTimelineParams as StoreListTimelineParams;
use ruxex_thread_store::ListTurnsParams as StoreListTurnsParams;
use ruxex_thread_store::LoadThreadHistoryParams as StoreLoadThreadHistoryParams;
use ruxex_thread_store::ReadThreadByRolloutPathParams as StoreReadThreadByRolloutPathParams;
use ruxex_thread_store::ReadThreadParams as StoreReadThreadParams;
use ruxex_thread_store::SearchThreadOccurrencesParams as StoreSearchThreadOccurrencesParams;
use ruxex_thread_store::SearchThreadsParams as StoreSearchThreadsParams;
use ruxex_thread_store::SortDirection as StoreSortDirection;
use ruxex_thread_store::StoredThread;
use ruxex_thread_store::StoredTurn;
use ruxex_thread_store::StoredTurnItemsView;
use ruxex_thread_store::StoredTurnStatus;
use ruxex_thread_store::ThreadMetadataPatch as StoreThreadMetadataPatch;
use ruxex_thread_store::ThreadRelationFilter as StoreThreadRelationFilter;
use ruxex_thread_store::ThreadSortKey as StoreThreadSortKey;
use ruxex_thread_store::ThreadStore;
use ruxex_thread_store::ThreadStoreError;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_pty::DEFAULT_OUTPUT_BYTES_CAP;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
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

mod account_processor;
mod catalog_processor;
mod command_exec_processor;
mod config_processor;
mod diagnostics;
mod environment_processor;
mod fs_processor;
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

pub(crate) use account_processor::AccountRequestProcessor;
pub(crate) use catalog_processor::CatalogRequestProcessor;
pub(crate) use command_exec_processor::CommandExecRequestProcessor;
pub(crate) use config_processor::ConfigRequestProcessor;
pub(crate) use diagnostics::read_server_diagnostics;
pub(crate) use environment_processor::EnvironmentRequestProcessor;
pub(crate) use fs_processor::FsRequestProcessor;
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
                    "invalid cwd for environment `{environment_id}`: path `{}` does not use absolute POSIX path syntax",
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
                            "invalid runtime workspace root for environment `{environment_id}`: path `{root}` does not use absolute POSIX path syntax"
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
