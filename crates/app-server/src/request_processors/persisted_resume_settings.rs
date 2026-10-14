use ruxex_protocol::models::ActivePermissionProfile;
use ruxex_protocol::protocol::AskForApproval;
use ruxex_protocol::protocol::EventMsg;
use ruxex_rollout::RolloutItem;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PersistedResumeSettings {
    pub(super) approval_policy: AskForApproval,
    pub(super) active_permission_profile: Option<ActivePermissionProfile>,
}

pub(super) fn latest_persisted_resume_settings(
    history: &[RolloutItem],
) -> Option<PersistedResumeSettings> {
    history
        .iter()
        .enumerate()
        .rev()
        .find_map(|(_index, item)| match item {
            RolloutItem::TurnContext(turn_context) => Some(PersistedResumeSettings {
                approval_policy: turn_context.approval_policy,
                active_permission_profile: turn_context.active_permission_profile.clone(),
            }),
            RolloutItem::EventMsg(EventMsg::ThreadSettingsApplied(event)) => {
                Some(PersistedResumeSettings {
                    approval_policy: event.thread_settings.approval_policy,
                    active_permission_profile: event
                        .thread_settings
                        .active_permission_profile
                        .clone(),
                })
            }
            _ => None,
        })
}
