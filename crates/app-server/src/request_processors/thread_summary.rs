use super::*;
use ruxex_protocol::config_types::MultiAgentMode;

#[cfg(test)]
fn extract_conversation_summary(
    path: PathBuf,
    head: &[serde_json::Value],
    session_meta: &SessionMeta,
    git: Option<&CoreGitInfo>,
    updated_at: Option<String>,
) -> Option<ConversationSummary> {
    let preview = head
        .iter()
        .filter_map(|value| serde_json::from_value::<ResponseItem>(value.clone()).ok())
        .find_map(|item| match ruxex_core::parse_turn_item(&item) {
            Some(TurnItem::UserMessage(user)) => Some(user.message()),
            _ => None,
        })?;

    let preview = strip_user_message_prefix(preview.as_str());

    let timestamp = if session_meta.timestamp.is_empty() {
        None
    } else {
        Some(session_meta.timestamp.clone())
    };
    let conversation_id = session_meta.id;
    let git_info = git.map(map_git_info);
    let updated_at = updated_at.or_else(|| timestamp.clone());

    Some(ConversationSummary {
        conversation_id,
        timestamp,
        updated_at,
        path,
        preview: preview.to_string(),
        cwd: session_meta.cwd.clone(),
        cli_version: session_meta.cli_version.clone(),
        source: session_meta.source.clone(),
        git_info,
    })
}

#[cfg(test)]
fn map_git_info(git_info: &CoreGitInfo) -> ConversationGitInfo {
    ConversationGitInfo {
        sha: git_info.commit_hash.as_ref().map(|sha| sha.0.clone()),
        branch: git_info.branch.clone(),
        origin_url: git_info.repository_url.clone().map(String::from),
    }
}

pub(super) fn with_thread_spawn_agent_metadata(
    source: ruxex_protocol::protocol::SessionSource,
    agent_nickname: Option<String>,
    agent_role: Option<String>,
) -> ruxex_protocol::protocol::SessionSource {
    if agent_nickname.is_none() && agent_role.is_none() {
        return source;
    }

    match source {
        ruxex_protocol::protocol::SessionSource::SubAgent(
            ruxex_protocol::protocol::SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_nickname: existing_agent_nickname,
                agent_role: existing_agent_role,
            },
        ) => ruxex_protocol::protocol::SessionSource::SubAgent(
            ruxex_protocol::protocol::SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
                agent_path,
                agent_nickname: agent_nickname.or(existing_agent_nickname),
                agent_role: agent_role.or(existing_agent_role),
            },
        ),
        _ => source,
    }
}

pub(crate) fn thread_response_active_permission_profile(
    active_permission_profile: Option<ruxex_protocol::models::ActivePermissionProfile>,
) -> Option<ruxex_app_server_protocol::ActivePermissionProfile> {
    active_permission_profile.map(Into::into)
}

pub(crate) fn thread_settings_from_config_snapshot(
    config_snapshot: &ThreadConfigSnapshot,
) -> ThreadSettings {
    ThreadSettings {
        disabled_plugin_ids: config_snapshot.disabled_plugin_ids.clone(),
        cwd: config_snapshot.cwd().clone(),
        approval_policy: config_snapshot.approval_policy.into(),
        sandbox_policy: config_snapshot.sandbox_policy().into(),
        active_permission_profile: thread_response_active_permission_profile(
            config_snapshot.active_permission_profile.clone(),
        ),
        model: config_snapshot.model.clone(),
        service_tier: config_snapshot.service_tier.clone(),
        effort: config_snapshot.reasoning_effort.clone(),
        summary: config_snapshot.reasoning_summary,
        collaboration_mode: config_snapshot.collaboration_mode.clone(),
        multi_agent_mode: MultiAgentMode::ExplicitRequestOnly,
    }
}

pub(super) fn thread_started_notification(mut thread: Thread) -> ThreadStartedNotification {
    thread.turns.clear();
    ThreadStartedNotification { thread }
}

#[cfg(test)]
#[path = "thread_summary_tests.rs"]
mod thread_summary_tests;
