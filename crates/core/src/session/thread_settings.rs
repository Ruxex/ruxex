//! Handles persistent thread-settings updates and serializes their persistence
//! with checkpoints written directly to storage.

use super::session::Session;
use super::session::SessionSettingsUpdate;
use super::step_settings::StepSettingsUpdate;
use crate::config::ConstraintResult;
use ruxex_history::RolloutItem;
use ruxex_protocol::protocol::ErrorEvent;
use ruxex_protocol::protocol::Event;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::RuxexErrorInfo;
use ruxex_protocol::protocol::ThreadSettingsAppliedEvent;
use ruxex_protocol::protocol::ThreadSettingsOverrides;
use ruxex_protocol::protocol::ThreadSettingsSnapshot;
use ruxex_thread_store::ThreadStoreResult;
use std::sync::Arc;
use tokio::sync::SemaphorePermit;

impl Session {
    /// Captures and flushes current settings under the shared persistence permit.
    pub(crate) async fn checkpoint_thread_settings(&self) -> ThreadStoreResult<()> {
        let _settings_guard = acquire_persistence_lock(self).await;
        if let Some(live_thread) = self.live_thread() {
            live_thread
                .append_items(&[RolloutItem::EventMsg(applied_event(self).await)])
                .await?;
            live_thread.flush().await?;
        }
        Ok(())
    }
}

/// Applies standalone thread settings and reports invalid overrides through the
/// normal event stream.
pub(super) async fn update(
    session: &Arc<Session>,
    submission_id: String,
    overrides: ThreadSettingsOverrides,
) {
    let updates = prepare_update(overrides);
    if let Err(error) = apply_update(session, submission_id.clone(), updates).await {
        session
            .send_event_raw(Event {
                id: submission_id,
                msg: EventMsg::Error(ErrorEvent {
                    misalignment: None,
                    message: format!("invalid thread settings override: {error}"),
                    ruxex_error_info: Some(RuxexErrorInfo::BadRequest),
                }),
            })
            .await;
    } else {
        // Standalone settings changes supersede a pending automatic continuation.
        session.state.lock().await.last_started_turn_id = None;
    }
}

/// Converts protocol overrides into the internal settings update shape.
pub(super) fn prepare_update(overrides: ThreadSettingsOverrides) -> SessionSettingsUpdate {
    let ThreadSettingsOverrides {
        environments,
        runtime_workspace_roots,
        profile_workspace_roots,
        approval_policy,
        sandbox_policy,
        permission_profile,
        active_permission_profile,
        model,
        effort,
        summary,
        service_tier,
        collaboration_mode,
        disabled_plugin_ids,
    } = overrides;
    SessionSettingsUpdate {
        step_settings: StepSettingsUpdate {
            model,
            effort,
            collaboration_mode,
            reasoning_summary: summary,
            service_tier,
            approval_policy,
        },
        environments,
        runtime_workspace_roots,
        profile_workspace_roots,
        sandbox_policy,
        permission_profile,
        active_permission_profile,
        disabled_plugin_ids,
        ..Default::default()
    }
}

/// Acquires the shared permit before capturing or changing persistent settings.
pub(super) async fn acquire_persistence_lock(session: &Session) -> SemaphorePermit<'_> {
    session
        .thread_settings_persistence
        .acquire()
        .await
        .unwrap_or_else(|_| unreachable!("thread settings persistence semaphore is never closed"))
}

/// Applies persistent settings and emits the resulting thread-owned snapshot.
pub(super) async fn apply_update(
    session: &Session,
    submission_id: String,
    updates: SessionSettingsUpdate,
) -> ConstraintResult<()> {
    let _settings_guard = acquire_persistence_lock(session).await;
    let commit = session.update_settings(updates).await?;
    emit_applied(session, submission_id, commit.snapshot).await;
    Ok(())
}

/// Emits the snapshot published by one successful settings update.
pub(super) async fn emit_applied(
    session: &Session,
    submission_id: String,
    snapshot: ThreadSettingsSnapshot,
) {
    let msg = EventMsg::ThreadSettingsApplied(ThreadSettingsAppliedEvent {
        thread_id: Some(session.thread_id()),
        thread_settings: snapshot,
    });
    session
        .send_event_raw_without_materializing_rollout(Event {
            id: submission_id,
            msg,
        })
        .await;
}

/// Builds a current thread-owned snapshot for storage checkpoints.
pub(super) async fn applied_event(session: &Session) -> EventMsg {
    EventMsg::ThreadSettingsApplied(ThreadSettingsAppliedEvent {
        thread_id: Some(session.thread_id()),
        thread_settings: session.thread_settings_snapshot().await,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruxex_protocol::protocol::ThreadSettingsOverrides;

    #[test]
    fn prepares_protocol_overrides_without_losing_sparse_fields() {
        let updates = prepare_update(ThreadSettingsOverrides {
            model: Some("model-b".to_string()),
            service_tier: Some(Some("priority".to_string())),
            disabled_plugin_ids: Some(vec!["plugin-a".to_string()]),
            ..Default::default()
        });

        assert_eq!(updates.step_settings.model.as_deref(), Some("model-b"));
        assert_eq!(
            updates.step_settings.service_tier,
            Some(Some("priority".to_string()))
        );
        assert_eq!(
            updates.disabled_plugin_ids,
            Some(vec!["plugin-a".to_string()])
        );
        assert!(updates.environments.is_none());
        assert!(updates.permission_profile.is_none());
    }
}
