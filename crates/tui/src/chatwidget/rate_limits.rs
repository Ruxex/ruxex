//! Rate-limit warning, prompt, and notice surfaces, plus quota-aware account refresh cadence.

use super::*;
use genex_app_server_protocol::GenexErrorInfo as AppServerGenexErrorInfo;
use genex_protocol::protocol::DEFAULT_RATE_LIMIT_ID;

pub(super) const RATE_LIMIT_SWITCH_PROMPT_THRESHOLD: f64 = 90.0;
pub(super) const RATE_LIMIT_SWITCH_PROMPT_VIEW_ID: &str = "rate-limit-switch-prompt";

const RATE_LIMIT_WARNING_THRESHOLDS: [f64; 4] = [50.0, 75.0, 90.0, 95.0];
const PRIMARY_LIMIT_FALLBACK_LABEL: &str = "usage";
const SECONDARY_LIMIT_FALLBACK_LABEL: &str = "secondary usage";

#[derive(Default)]
pub(super) struct RateLimitWarningState {
    pub(super) secondary_index: usize,
    pub(super) primary_index: usize,
}

impl RateLimitWarningState {
    pub(super) fn take_warnings(
        &mut self,
        secondary_used_percent: Option<f64>,
        secondary_window_minutes: Option<i64>,
        primary_used_percent: Option<f64>,
        primary_window_minutes: Option<i64>,
    ) -> Vec<String> {
        let reached_secondary_cap =
            matches!(secondary_used_percent, Some(percent) if percent == 100.0);
        let reached_primary_cap = matches!(primary_used_percent, Some(percent) if percent == 100.0);
        if reached_secondary_cap || reached_primary_cap {
            return Vec::new();
        }

        let mut warnings = Vec::new();

        if let Some(secondary_used_percent) = secondary_used_percent {
            let mut highest_secondary: Option<f64> = None;
            while self.secondary_index < RATE_LIMIT_WARNING_THRESHOLDS.len()
                && secondary_used_percent >= RATE_LIMIT_WARNING_THRESHOLDS[self.secondary_index]
            {
                let threshold = RATE_LIMIT_WARNING_THRESHOLDS[self.secondary_index];
                if threshold != 50.0
                    || secondary_window_minutes
                        .is_some_and(|minutes| is_approximate_window(minutes, 5 * 60))
                {
                    highest_secondary = Some(threshold);
                }
                self.secondary_index += 1;
            }
            if let Some(threshold) = highest_secondary {
                let limit_label =
                    limit_label_for_window(secondary_window_minutes, /*is_secondary*/ true);
                let remaining_percent = 100.0 - threshold;
                warnings.push(format!(
                    "Heads up, you have less than {remaining_percent:.0}% of your {limit_label} limit left. Run /status for a breakdown."
                ));
            }
        }

        if let Some(primary_used_percent) = primary_used_percent {
            let mut highest_primary: Option<f64> = None;
            while self.primary_index < RATE_LIMIT_WARNING_THRESHOLDS.len()
                && primary_used_percent >= RATE_LIMIT_WARNING_THRESHOLDS[self.primary_index]
            {
                let threshold = RATE_LIMIT_WARNING_THRESHOLDS[self.primary_index];
                if threshold != 50.0
                    || primary_window_minutes
                        .is_some_and(|minutes| is_approximate_window(minutes, 5 * 60))
                {
                    highest_primary = Some(threshold);
                }
                self.primary_index += 1;
            }
            if let Some(threshold) = highest_primary {
                let limit_label =
                    limit_label_for_window(primary_window_minutes, /*is_secondary*/ false);
                let remaining_percent = 100.0 - threshold;
                warnings.push(format!(
                    "Heads up, you have less than {remaining_percent:.0}% of your {limit_label} limit left. Run /status for a breakdown."
                ));
            }
        }

        warnings
    }
}

pub(crate) fn limit_label_for_window(window_minutes: Option<i64>, is_secondary: bool) -> String {
    window_minutes
        .and_then(get_limits_duration)
        .unwrap_or_else(|| fallback_limit_label(is_secondary).to_string())
}

pub(crate) fn get_limits_duration(windows_minutes: i64) -> Option<String> {
    const MINUTES_PER_HOUR: i64 = 60;
    const MINUTES_PER_5_HOURS: i64 = 5 * MINUTES_PER_HOUR;
    const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;
    const MINUTES_PER_WEEK: i64 = 7 * MINUTES_PER_DAY;
    const MINUTES_PER_MONTH: i64 = 30 * MINUTES_PER_DAY;
    const MINUTES_PER_YEAR: i64 = 365 * MINUTES_PER_DAY;

    let windows_minutes = windows_minutes.max(0);

    if is_approximate_window(windows_minutes, MINUTES_PER_5_HOURS) {
        Some("5h".to_string())
    } else if is_approximate_window(windows_minutes, MINUTES_PER_DAY) {
        Some("daily".to_string())
    } else if is_approximate_window(windows_minutes, MINUTES_PER_WEEK) {
        Some("weekly".to_string())
    } else if is_approximate_window(windows_minutes, MINUTES_PER_MONTH) {
        Some("monthly".to_string())
    } else if is_approximate_window(windows_minutes, MINUTES_PER_YEAR) {
        Some("annual".to_string())
    } else {
        None
    }
}

pub(crate) fn fallback_limit_label(is_secondary: bool) -> &'static str {
    if is_secondary {
        SECONDARY_LIMIT_FALLBACK_LABEL
    } else {
        PRIMARY_LIMIT_FALLBACK_LABEL
    }
}

fn is_approximate_window(minutes: i64, expected_minutes: i64) -> bool {
    let minutes = minutes as f64;
    let expected_minutes = expected_minutes as f64;
    minutes >= expected_minutes * 0.95 && minutes <= expected_minutes * 1.05
}

#[derive(Default)]
pub(super) enum RateLimitSwitchPromptState {
    #[default]
    Idle,
    Pending,
    Shown,
}

#[derive(Debug)]
pub(super) enum RateLimitErrorKind {
    ServerOverloaded,
    UsageLimit,
    Generic,
}

pub(super) fn app_server_rate_limit_error_kind(
    info: &AppServerGenexErrorInfo,
) -> Option<RateLimitErrorKind> {
    match info {
        AppServerGenexErrorInfo::ServerOverloaded => Some(RateLimitErrorKind::ServerOverloaded),
        AppServerGenexErrorInfo::UsageLimitExceeded => Some(RateLimitErrorKind::UsageLimit),
        AppServerGenexErrorInfo::ResponseTooManyFailedAttempts {
            http_status_code: Some(429),
        } => Some(RateLimitErrorKind::Generic),
        _ => None,
    }
}

pub(super) fn is_app_server_cyber_policy_error(info: &AppServerGenexErrorInfo) -> bool {
    matches!(info, AppServerGenexErrorInfo::CyberPolicy)
}

#[derive(Clone, Copy)]
enum RateLimitSnapshotSource {
    AccountUsage,
    RollingUpdate,
}

fn has_usable_workspace_credits(credits: &CreditsSnapshot) -> bool {
    credits.unlimited || credits.has_credits
}

impl ChatWidget {
    /// Poll more often near exhaustion for managed providers, independently of experiments.
    pub(crate) fn rate_limit_refresh_interval(&self) -> Option<std::time::Duration> {
        if !self.should_prefetch_rate_limits() {
            return None;
        }
        // Ignore unrelated model buckets; watch ordinary usage and the selected model's bucket.
        let used_percent = self
            .rate_limit_snapshots_by_limit_id
            .iter()
            .filter(|(limit_id, snapshot)| {
                limit_id.as_str() == DEFAULT_RATE_LIMIT_ID
                    || snapshot.limit_name == self.current_model()
            })
            .flat_map(|(_, snapshot)| snapshot.primary.iter().chain(snapshot.secondary.iter()))
            .map(|window| window.used_percent)
            .filter(|percent| percent.is_finite())
            .max_by(f64::total_cmp)
            .unwrap_or_default();
        let seconds = if used_percent >= 99.0 {
            5
        } else if used_percent >= 90.0 {
            15
        } else if used_percent >= 75.0 {
            30
        } else {
            60
        };
        Some(std::time::Duration::from_secs(seconds))
    }

    pub(crate) fn hold_rate_limit_recovery(&mut self) -> bool {
        std::mem::replace(&mut self.input_queue.rate_limit_recovery_pending, true)
    }

    pub(crate) fn finish_rate_limit_recovery(&mut self) {
        if std::mem::take(&mut self.input_queue.rate_limit_recovery_pending) {
            self.submit_initial_user_message_if_pending();
            self.maybe_send_next_queued_input();
        }
    }

    pub(crate) fn on_rate_limit_snapshot(&mut self, snapshot: Option<RateLimitSnapshot>) {
        self.on_rate_limit_snapshot_from(snapshot, RateLimitSnapshotSource::AccountUsage);
    }

    pub(crate) fn on_rolling_rate_limit_snapshot(&mut self, snapshot: RateLimitSnapshot) {
        // Rolling app-server notifications are sparse. Preserve metadata learned from the full read.
        self.on_rate_limit_snapshot_from(Some(snapshot), RateLimitSnapshotSource::RollingUpdate);
    }

    fn on_rate_limit_snapshot_from(
        &mut self,
        snapshot: Option<RateLimitSnapshot>,
        source: RateLimitSnapshotSource,
    ) {
        if let Some(mut snapshot) = snapshot {
            let limit_id = snapshot
                .limit_id
                .clone()
                .unwrap_or_else(|| DEFAULT_RATE_LIMIT_ID.to_string());
            if matches!(source, RateLimitSnapshotSource::RollingUpdate)
                && snapshot.credits.is_none()
            {
                snapshot.credits = self
                    .rate_limit_snapshots_by_limit_id
                    .get(&limit_id)
                    .and_then(|display| display.credits.as_ref())
                    .map(|credits| CreditsSnapshot {
                        has_credits: credits.has_credits,
                        unlimited: credits.unlimited,
                        balance: credits.balance.clone(),
                    });
            }
            self.plan_type = snapshot.plan_type.or(self.plan_type);

            let is_default_limit = limit_id.eq_ignore_ascii_case(DEFAULT_RATE_LIMIT_ID);
            if is_default_limit
                && (matches!(source, RateLimitSnapshotSource::AccountUsage)
                    || snapshot.spend_control_reached.is_some())
            {
                self.genex_spend_control_reached = snapshot.spend_control_reached;
            }
            if (is_default_limit && matches!(source, RateLimitSnapshotSource::AccountUsage))
                || snapshot.rate_limit_reached_type.is_some()
            {
                self.genex_rate_limit_reached_type = snapshot.rate_limit_reached_type;
            }
            let workspace_limit_reached = self.genex_spend_control_reached == Some(true)
                || matches!(
                    self.genex_rate_limit_reached_type,
                    Some(
                        RateLimitReachedType::WorkspaceOwnerCreditsDepleted
                            | RateLimitReachedType::WorkspaceMemberCreditsDepleted
                            | RateLimitReachedType::WorkspaceOwnerUsageLimitReached
                            | RateLimitReachedType::WorkspaceMemberUsageLimitReached
                    )
                );
            let has_workspace_credits = !workspace_limit_reached
                && snapshot
                    .credits
                    .as_ref()
                    .is_some_and(has_usable_workspace_credits);
            if is_default_limit && has_workspace_credits {
                match self.rate_limit_switch_prompt {
                    RateLimitSwitchPromptState::Pending => {
                        self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Idle;
                    }
                    RateLimitSwitchPromptState::Shown => {
                        self.bottom_pane
                            .dismiss_view_by_id(RATE_LIMIT_SWITCH_PROMPT_VIEW_ID);
                    }
                    RateLimitSwitchPromptState::Idle => {}
                }
            }
            let should_warn_about_rate_limit_usage = is_default_limit && !has_workspace_credits;
            let warnings = if should_warn_about_rate_limit_usage {
                self.rate_limit_warnings.take_warnings(
                    snapshot
                        .secondary
                        .as_ref()
                        .map(|window| f64::from(window.used_percent)),
                    snapshot
                        .secondary
                        .as_ref()
                        .and_then(|window| window.window_duration_mins),
                    snapshot
                        .primary
                        .as_ref()
                        .map(|window| f64::from(window.used_percent)),
                    snapshot
                        .primary
                        .as_ref()
                        .and_then(|window| window.window_duration_mins),
                )
            } else {
                vec![]
            };

            let high_usage = is_default_limit
                && (snapshot
                    .secondary
                    .as_ref()
                    .map(|w| f64::from(w.used_percent) >= RATE_LIMIT_SWITCH_PROMPT_THRESHOLD)
                    .unwrap_or(false)
                    || snapshot
                        .primary
                        .as_ref()
                        .map(|w| f64::from(w.used_percent) >= RATE_LIMIT_SWITCH_PROMPT_THRESHOLD)
                        .unwrap_or(false));

            if high_usage
                && !has_workspace_credits
                && !self.rate_limit_switch_prompt_hidden()
                && !matches!(
                    self.rate_limit_switch_prompt,
                    RateLimitSwitchPromptState::Shown
                )
            {
                self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Pending;
            }

            // /wham/usage identifies ordinary and additional model limits separately. Streamed
            // updates still drive warnings/recovery above, but must not overwrite status data.
            if matches!(source, RateLimitSnapshotSource::AccountUsage) {
                let limit_label = snapshot
                    .limit_name
                    .clone()
                    .unwrap_or_else(|| limit_id.clone());
                let display =
                    rate_limit_snapshot_display_for_limit(&snapshot, limit_label, Local::now());
                self.rate_limit_snapshots_by_limit_id
                    .insert(limit_id, display);
            }

            if !warnings.is_empty() {
                for warning in warnings {
                    self.add_to_history(history_cell::new_warning_event(warning));
                }
                self.request_redraw();
            }
        } else {
            self.rate_limit_snapshots_by_limit_id.clear();
            self.genex_rate_limit_reached_type = None;
            self.genex_spend_control_reached = None;
        }
        self.refresh_status_line();
    }

    pub(super) fn stop_rate_limit_poller(&mut self) {}

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn prefetch_rate_limits(&mut self) {
        self.stop_rate_limit_poller();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn should_prefetch_rate_limits(&self) -> bool {
        self.has_managed_account
    }

    fn lower_cost_preset(&self) -> Option<ModelPreset> {
        let normal_model = self
            .rate_limit_snapshots_by_limit_id
            .values()
            .find_map(|snapshot| snapshot.normal_model_slug.as_deref())?;
        if normal_model == self.current_model() {
            return None;
        }
        let models = self.model_catalog.try_list_models().ok()?;
        models
            .iter()
            .find(|preset| preset.show_in_picker && preset.model == normal_model)
            .cloned()
    }

    fn rate_limit_switch_prompt_hidden(&self) -> bool {
        self.local_settings
            .notices
            .hide_rate_limit_model_nudge
            .unwrap_or(false)
    }

    pub(super) fn maybe_show_pending_rate_limit_prompt(&mut self) {
        if self.rate_limit_switch_prompt_hidden() {
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Idle;
            return;
        }
        if !matches!(
            self.rate_limit_switch_prompt,
            RateLimitSwitchPromptState::Pending
        ) {
            return;
        }
        if let Some(preset) = self.lower_cost_preset() {
            self.open_rate_limit_switch_prompt(preset);
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Shown;
        } else {
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Idle;
        }
    }

    fn open_rate_limit_switch_prompt(&mut self, preset: ModelPreset) {
        let switch_model = preset.model;
        let switch_model_for_events = switch_model.clone();
        let default_effort: ReasoningEffortConfig = preset.default_reasoning_effort;

        let switch_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::GenexOp(AppCommand::override_turn_context(
                /*cwd*/ None,
                /*approval_policy*/ None,
                /*permission_profile*/ None,
                /*active_permission_profile*/ None,
                Some(switch_model_for_events.clone()),
                Some(Some(default_effort.clone())),
                /*summary*/ None,
                /*service_tier*/ None,
                /*collaboration_mode*/ None,
            )));
            tx.send(AppEvent::UpdateModel(switch_model_for_events.clone()));
            tx.send(AppEvent::UpdateReasoningEffort(Some(
                default_effort.clone(),
            )));
        })];

        let keep_actions: Vec<SelectionAction> = Vec::new();
        let never_actions: Vec<SelectionAction> = vec![Box::new(|tx| {
            tx.send(AppEvent::UpdateRateLimitSwitchPromptHidden(true));
            tx.send(AppEvent::PersistRateLimitSwitchPromptHidden);
        })];
        let description = if preset.description.is_empty() {
            Some("Uses fewer credits for upcoming turns.".to_string())
        } else {
            Some(preset.description)
        };

        let items = vec![
            SelectionItem {
                name: format!("Switch to {switch_model}"),
                description,
                selected_description: None,
                is_current: false,
                actions: switch_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Keep current model".to_string(),
                description: None,
                selected_description: None,
                is_current: false,
                actions: keep_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Keep current model (never show again)".to_string(),
                description: Some(
                    "Hide future rate limit reminders about switching models.".to_string(),
                ),
                selected_description: None,
                is_current: false,
                actions: never_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            view_id: Some(RATE_LIMIT_SWITCH_PROMPT_VIEW_ID),
            title: Some("Approaching rate limits".to_string()),
            subtitle: Some(format!("Switch to {switch_model} for lower credit usage?")),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    pub(crate) fn set_rate_limit_switch_prompt_hidden(&mut self, hidden: bool) {
        self.local_settings.notices.hide_rate_limit_model_nudge = Some(hidden);
        if hidden {
            self.rate_limit_switch_prompt = RateLimitSwitchPromptState::Idle;
        }
    }
}
