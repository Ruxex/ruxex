//! Runtime configuration persistence helpers for the TUI app.
//!
//! This module owns the app-level glue between config.toml edits, in-memory `Config` refreshes,
//! and the ChatWidget copy of session settings, keeping persistence-heavy code out of the main app
//! loop.

use super::*;
use ruxex_config::ConfigLayerSource;

async fn build_config_on_runtime_worker(
    builder: ConfigBuilder,
    error_context: String,
) -> Result<Config> {
    // Tokio stores the task output inline even when it boxes the future. Keep the large
    // Config off the caller's stack while Tokio allocates the task during session switches.
    match tokio::spawn(async move { builder.build().await.map(Box::new) }).await {
        Ok(build_result) => build_result.map(|config| *config).wrap_err(error_context),
        Err(err) if err.is_panic() => std::panic::resume_unwind(err.into_panic()),
        Err(err) => Err(err).wrap_err_with(|| format!("{error_context} task failed")),
    }
}

pub(super) fn resume_model_settings_for_overrides(
    config: &Config,
    harness_overrides: &ConfigOverrides,
) -> crate::app_server_session::ResumeModelSettings {
    let has_layer_override = config.config_layer_stack.layers_high_to_low().any(|layer| {
        matches!(
            &layer.name,
            ConfigLayerSource::SessionFlags
                | ConfigLayerSource::User {
                    profile: Some(_),
                    ..
                }
        ) && ["model", "model_reasoning_effort"]
            .iter()
            .any(|key| layer.config.get(*key).is_some())
    });
    if harness_overrides.model.is_some() || has_layer_override {
        crate::app_server_session::ResumeModelSettings::OverrideFromCurrentConfig
    } else {
        crate::app_server_session::ResumeModelSettings::RestoreFromThread
    }
}

pub(super) fn has_explicit_resume_permission_override(
    config: &Config,
    overrides: &ConfigOverrides,
) -> bool {
    overrides.approval_policy.is_some()
        || overrides.sandbox_mode.is_some()
        || overrides.permission_profile.is_some()
        || overrides.default_permissions.is_some()
        || !overrides.additional_writable_roots.is_empty()
        || overrides.workspace_roots.is_some()
        || config.config_layer_stack.layers_high_to_low().any(|layer| {
            matches!(
                &layer.name,
                ConfigLayerSource::SessionFlags
                    | ConfigLayerSource::User {
                        profile: Some(_),
                        ..
                    }
            ) && [
                "approval_policy",
                "sandbox_mode",
                "default_permissions",
                "permissions",
                "network",
                "sandbox_workspace_write",
            ]
            .iter()
            .any(|key| layer.config.get(*key).is_some())
        })
}

impl App {
    pub(super) async fn enable_feature_for_new_threads(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &AppServerSession,
        feature: Feature,
    ) {
        let label = match feature {
            Feature::Collab => "Subagents",
            _ => return,
        };
        let edits = vec![crate::config_update::build_feature_enabled_edit(
            feature.key(),
            /*enabled*/ true,
        )];
        let notice: Box<dyn HistoryCell> = match crate::config_update::write_config_batch(
            app_server.request_handle(),
            edits,
        )
        .await
        {
            Ok(response) if response.status == WriteStatus::Ok => {
                Box::new(history_cell::new_warning_event(format!(
                    "{label} setting saved on the server for new threads. This thread is unchanged. Project or task settings may override it."
                )))
            }
            Ok(response) => Box::new(history_cell::new_error_event(format!(
                "{label} setting was saved but is overridden: {}",
                overridden_write_message(&response)
            ))),
            Err(err) => Box::new(history_cell::new_error_event(format!(
                "Failed to save {label} setting: {}",
                crate::config_update::format_config_error(&err)
            ))),
        };
        self.insert_history_cell(tui, notice);
    }

    pub(super) async fn rebuild_config_for_cwd(&self, cwd: PathBuf) -> Result<Config> {
        let mut overrides = self.harness_overrides.clone();
        overrides.cwd = Some(cwd.clone());
        let cwd_display = cwd.display().to_string();
        let builder = ConfigBuilder::default()
            .ruxex_home(self.config.ruxex_home.to_path_buf())
            .cli_overrides(self.cli_kv_overrides.clone())
            .harness_overrides(overrides)
            .loader_overrides(self.loader_overrides.clone());
        build_config_on_runtime_worker(
            builder,
            format!("Failed to rebuild config for cwd {cwd_display}"),
        )
        .await
    }

    pub(super) async fn rebuild_config_for_permission_profile(
        &self,
        profile_id: &str,
    ) -> Result<Config> {
        let mut overrides = self.harness_overrides.clone();
        overrides.cwd = Some(self.chat_widget.config_ref().cwd.to_path_buf());
        overrides.sandbox_mode = None;
        overrides.permission_profile = None;
        overrides.default_permissions = Some(profile_id.to_string());
        let builder = ConfigBuilder::default()
            .ruxex_home(self.config.ruxex_home.to_path_buf())
            .cli_overrides(self.cli_kv_overrides.clone())
            .harness_overrides(overrides)
            .loader_overrides(self.loader_overrides.clone());
        build_config_on_runtime_worker(
            builder,
            format!("Failed to rebuild config for permission profile {profile_id}"),
        )
        .await
    }

    pub(super) async fn apply_permission_profile_selection(
        &mut self,
        selection: PermissionProfileSelection,
    ) -> bool {
        if self.reject_pending_permission_change() {
            return false;
        }
        let PermissionProfileSelection {
            profile_id,
            approval_policy,
            display_label,
        } = selection;
        let selected_config = match self
            .rebuild_config_for_permission_profile(profile_id.as_str())
            .await
        {
            Ok(config) => config,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    profile_id,
                    "failed to resolve selected permission profile"
                );
                self.chat_widget.add_error_message(format!(
                    "Failed to set permission profile `{profile_id}`: {err}"
                ));
                return false;
            }
        };
        let permission_profile = selected_config.permissions.permission_profile();
        let active_permission_profile = selected_config.permissions.active_permission_profile();
        let network = selected_config.permissions.network.clone();

        let mut config = self.config.clone();
        if let Some(policy) = approval_policy
            && !self.try_set_approval_policy_on_config(
                &mut config,
                policy,
                "Failed to set approval policy",
                "failed to set selected permission profile approval policy on app config",
            )
        {
            return false;
        }
        if let Err(err) = config
            .permissions
            .set_permission_profile_from_session_snapshot(
                PermissionProfileSnapshot::from_session_snapshot(
                    permission_profile.clone(),
                    active_permission_profile.clone(),
                ),
            )
        {
            tracing::warn!(
                error = %err,
                profile_id,
                "failed to set selected permission profile on app config"
            );
            self.chat_widget.add_error_message(format!(
                "Failed to set permission profile `{profile_id}`: {err}"
            ));
            return false;
        }
        config.permissions.network = network.clone();
        self.config = config;

        if let Some(policy) = approval_policy {
            self.runtime_approval_policy_override =
                Some(RuntimeApprovalPolicyOverride::Explicit(policy));
            self.chat_widget.set_approval_policy(policy);
        }
        if let Err(err) = self.chat_widget.set_permission_profile_with_active_profile(
            permission_profile.clone(),
            active_permission_profile.clone(),
        ) {
            tracing::warn!(
                error = %err,
                profile_id,
                "failed to set selected permission profile on chat config"
            );
            self.chat_widget.add_error_message(format!(
                "Failed to set permission profile `{profile_id}`: {err}"
            ));
            return false;
        }
        self.chat_widget.set_permission_network(network);
        if let Some(thread_id) = self.chat_widget.thread_id() {
            self.agents_overview
                .selected_permission_profiles
                .insert(thread_id, profile_id.clone());
        }
        self.runtime_permission_profile_override =
            Some(RuntimePermissionProfileOverride::from_config(&self.config));
        self.sync_active_thread_permission_settings_to_cached_session()
            .await;
        self.app_event_tx
            .send(AppEvent::RuxexOp(AppCommand::override_turn_context(
                /*cwd*/ None,
                approval_policy,
                Some(permission_profile.clone()),
                active_permission_profile,
                /*model*/ None,
                /*effort*/ None,
                /*summary*/ None,
                /*service_tier*/ None,
                /*collaboration_mode*/ None,
            )));
        self.app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
            history_cell::new_info_event(
                format!("Permissions updated to {display_label}"),
                /*hint*/ None,
            ),
        )));
        true
    }

    pub(super) async fn select_permission_profile(
        &mut self,
        app_server: &mut AppServerSession,
        selection: PermissionProfileSelection,
    ) {
        if self.reject_pending_permission_change() {
            return;
        }
        if (self.chat_widget.thread_id().is_none() && selection.profile_id.starts_with(':'))
            || (app_server.thread_params_mode()
                == crate::app_server_session::ThreadParamsMode::Embedded
                && self
                    .config
                    .custom_permission_profiles
                    .iter()
                    .any(|profile| profile.id == selection.profile_id))
        {
            if self.apply_permission_profile_selection(selection).await {
                self.chat_widget.submit_initial_user_message_if_pending();
            }
            return;
        }
        let Some(thread_id) = self.chat_widget.thread_id() else {
            self.chat_widget
                .retain_input_after_failed_permission_selection();
            self.chat_widget.add_error_message(
                "Wait for the task to connect before selecting permissions.".into(),
            );
            return;
        };
        if !selection.profile_id.starts_with(':')
            && self.chat_widget.is_user_turn_pending_or_running()
        {
            self.chat_widget
                .retain_input_after_failed_permission_selection();
            self.chat_widget.add_error_message(
                "Wait for the current turn to finish before changing permissions.".into(),
            );
            return;
        }
        let config = self.chat_widget.config_ref();
        if config
            .permissions
            .active_permission_profile()
            .is_some_and(|profile| profile.id == selection.profile_id)
            && selection
                .approval_policy
                .is_none_or(|policy| config.permissions.approval_policy.value() == policy.to_core())
        {
            self.agents_overview
                .selected_permission_profiles
                .insert(thread_id, selection.profile_id);
            self.chat_widget.submit_initial_user_message_if_pending();
            return;
        }
        let params = ThreadSettingsUpdateParams {
            thread_id: thread_id.to_string(),
            permissions: Some(selection.profile_id.clone()),
            approval_policy: selection.approval_policy,
            ..Default::default()
        };
        match app_server.thread_settings_update(params).await {
            Ok(true) => {
                self.agents_overview
                    .selected_permission_profiles
                    .insert(thread_id, selection.profile_id.clone());
                self.pending_server_profiles
                    .insert(thread_id, selection.clone());
                self.chat_widget.add_info_message(
                    format!(
                        "Permission selection requested: {}",
                        selection.display_label
                    ),
                    /*hint*/ None,
                );
                self.chat_widget.submit_initial_user_message_if_pending();
            }
            Ok(false) => {
                self.chat_widget
                    .retain_input_after_failed_permission_selection();
                self.chat_widget
                    .add_error_message("Permission selection requires a newer app server.".into());
            }
            Err(error) => {
                self.chat_widget
                    .retain_input_after_failed_permission_selection();
                self.chat_widget
                    .add_error_message(format!("Failed to select permissions: {error:#}"));
            }
        }
    }

    pub(super) fn reject_pending_permission_change(&mut self) -> bool {
        if self
            .chat_widget
            .thread_id()
            .is_some_and(|thread_id| self.pending_server_profiles.contains_key(&thread_id))
        {
            self.chat_widget.add_error_message(
                "Wait for permissions to update before changing permissions.".into(),
            );
            return true;
        }
        false
    }

    pub(super) fn reject_pending_permission_root_switch(&mut self) -> bool {
        if !self
            .pending_server_profiles
            .keys()
            .any(|thread_id| !self.thread_unavailable(*thread_id))
        {
            return false;
        }
        self.chat_widget
            .add_error_message("Wait for permissions to update before switching tasks.".into());
        true
    }

    pub(super) fn confirmed_server_profile(
        &self,
        thread_id: ThreadId,
    ) -> Option<PermissionProfileSelection> {
        if self.chat_widget.thread_id() != Some(thread_id) {
            return None;
        }
        let config = self.chat_widget.config_ref();
        let active = config.permissions.active_permission_profile()?;
        if active.id.starts_with(':')
            || (self.app_server_target.thread_params_mode()
                == crate::app_server_session::ThreadParamsMode::Embedded
                && self
                    .config
                    .custom_permission_profiles
                    .iter()
                    .any(|profile| profile.id == active.id))
        {
            return None;
        }
        Some(PermissionProfileSelection {
            profile_id: active.id.clone(),
            approval_policy: Some(config.permissions.approval_policy.value().into()),
            display_label: active.id,
        })
    }

    pub(super) async fn refresh_in_memory_config_from_disk(&mut self) -> Result<()> {
        let mut config = self
            .rebuild_config_for_cwd(self.chat_widget.config_ref().cwd.to_path_buf())
            .await?;
        self.apply_runtime_policy_overrides(&mut config, RuntimePolicyOverrideScope::All);
        self.local_settings = crate::local_settings::LocalSettings::from(&config);
        self.refresh_server_version_overview_notice(RUXEX_CLI_VERSION);
        // Other preferences have runtime caches and are adopted when the widget is replaced.
        self.chat_widget
            .local_settings
            .tui
            .terminal_resize_reflow_max_rows =
            self.local_settings.tui.terminal_resize_reflow_max_rows;
        self.config = config;
        self.chat_widget.sync_plugin_mentions_config(&self.config);
        Ok(())
    }

    pub(super) async fn refresh_in_memory_config_from_disk_best_effort(&mut self, action: &str) {
        if let Err(err) = self.refresh_in_memory_config_from_disk().await {
            tracing::warn!(
                error = %err,
                action,
                "failed to refresh config before thread transition; continuing with current in-memory config"
            );
        }
    }

    pub(super) async fn read_effective_config_after_overridden_write(
        &mut self,
        app_server: &mut AppServerSession,
        setting: &str,
    ) -> Option<ConfigReadResponse> {
        let cwd = self.chat_widget.config_ref().cwd.display().to_string();
        match crate::config_update::read_effective_config(app_server.request_handle(), cwd).await {
            Ok(response) => Some(response),
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    setting,
                    "failed to refresh effective config after an overridden write"
                );
                self.chat_widget.add_error_message(format!(
                    "{setting} were saved, but Ruxex could not refresh the effective config: {err}"
                ));
                None
            }
        }
    }

    pub(super) async fn rebuild_config_for_resume_or_fallback(
        &mut self,
        current_cwd: &Path,
        resume_cwd: PathBuf,
    ) -> Result<(Config, crate::local_settings::LocalSettings)> {
        match self.rebuild_config_for_cwd(resume_cwd.clone()).await {
            Ok(config) => {
                let local_settings = crate::local_settings::LocalSettings::from(&config);
                Ok((config, local_settings))
            }
            Err(err) => {
                if crate::session_resume::cwds_differ(current_cwd, &resume_cwd) {
                    Err(err)
                } else {
                    let resume_cwd_display = resume_cwd.display().to_string();
                    tracing::warn!(
                        error = %err,
                        cwd = %resume_cwd_display,
                        "failed to rebuild config for same-cwd resume; using current in-memory config"
                    );
                    Ok((self.config.clone(), self.local_settings.clone()))
                }
            }
        }
    }

    pub(super) fn apply_runtime_policy_overrides(
        &mut self,
        config: &mut Config,
        scope: RuntimePolicyOverrideScope,
    ) {
        if let Some(policy) = self.runtime_approval_policy_override
            && (scope == RuntimePolicyOverrideScope::All
                || matches!(policy, RuntimeApprovalPolicyOverride::Explicit(_)))
            && let Err(err) = config
                .permissions
                .approval_policy
                .set(policy.policy().to_core())
        {
            tracing::warn!(%err, "failed to carry forward approval policy override");
            self.chat_widget.add_error_message(format!(
                "Failed to carry forward approval policy override: {err}"
            ));
        }
        if let Some(profile_override) = self.runtime_permission_profile_override.as_ref()
            && (scope == RuntimePolicyOverrideScope::All
                || profile_override.turn_override
                    == RuntimePermissionProfileTurnOverride::LegacySandbox)
        {
            match config
                .permissions
                .set_permission_profile_from_session_snapshot(
                    PermissionProfileSnapshot::from_session_snapshot(
                        profile_override.permission_profile.clone(),
                        profile_override.active_permission_profile.clone(),
                    ),
                ) {
                Ok(()) => {
                    config.permissions.network = profile_override.network.clone();
                }
                Err(err) => {
                    tracing::warn!(%err, "failed to carry forward permission profile override");
                    self.chat_widget.add_error_message(format!(
                        "Failed to carry forward permission profile override: {err}"
                    ));
                }
            }
        }
    }

    pub(super) fn try_set_approval_policy_on_config(
        &mut self,
        config: &mut Config,
        policy: AskForApproval,
        user_message_prefix: &str,
        log_message: &str,
    ) -> bool {
        if let Err(err) = config.permissions.approval_policy.set(policy.to_core()) {
            tracing::warn!(error = %err, "{log_message}");
            self.chat_widget
                .add_error_message(format!("{user_message_prefix}: {err}"));
            return false;
        }

        true
    }

    pub(super) fn try_set_builtin_active_permission_profile_on_config(
        &mut self,
        config: &mut Config,
        active_permission_profile: ActivePermissionProfile,
        user_message_prefix: &str,
        log_message: &str,
    ) -> Option<PermissionProfile> {
        let permission_profile =
            builtin_permission_profile_for_active_permission_profile(&active_permission_profile)?;
        if let Err(err) = config
            .permissions
            .set_permission_profile_from_session_snapshot(PermissionProfileSnapshot::active(
                permission_profile.clone(),
                active_permission_profile,
            ))
        {
            tracing::warn!(error = %err, "{log_message}");
            self.chat_widget
                .add_error_message(format!("{user_message_prefix}: {err}"));
            return None;
        }
        Some(permission_profile)
    }

    pub(super) async fn update_feature_flags(
        &mut self,
        app_server: &mut AppServerSession,
        updates: Vec<(Feature, bool)>,
    ) {
        if updates.is_empty() {
            return;
        }
        let mut next_config = self.config.clone();
        let mut feature_updates_to_apply = Vec::with_capacity(updates.len());
        let mut config_edits = Vec::new();
        for (feature, enabled) in updates {
            let feature_key = feature.key();
            let mut feature_config = next_config.clone();
            let Ok(()) = feature_config.features.set_enabled(feature, enabled) else {
                self.chat_widget
                    .add_error_message(format!("Failed to update feature {feature_key}"));
                continue;
            };
            let effective_enabled = feature_config.features.enabled(feature);
            next_config = feature_config;
            feature_updates_to_apply.push((feature, effective_enabled));
            config_edits.push(crate::config_update::build_feature_enabled_edit(
                feature_key,
                effective_enabled,
            ));
        }
        let write_response = match crate::config_update::write_config_batch(
            app_server.request_handle(),
            config_edits,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => {
                self.chat_widget
                    .add_error_message(format!("Failed to update features: {err}"));
                return;
            }
        };
        if write_response.status == WriteStatus::OkOverridden {
            if let Some(effective_config) = self
                .read_effective_config_after_overridden_write(app_server, "Feature changes")
                .await
            {
                self.sync_feature_state_from_effective_config(
                    &effective_config,
                    &feature_updates_to_apply,
                );
            }
            return;
        }
        self.config = next_config;
        for (feature, enabled) in feature_updates_to_apply {
            self.chat_widget.set_feature_enabled(feature, enabled);
        }
    }

    pub(super) fn reasoning_label(reasoning_effort: Option<&ReasoningEffortConfig>) -> String {
        match reasoning_effort {
            None | Some(ReasoningEffortConfig::None) => "default".to_string(),
            Some(reasoning_effort) => reasoning_effort.as_str().to_string(),
        }
    }

    pub(super) fn reasoning_label_for(
        _model: &str,
        reasoning_effort: Option<&ReasoningEffortConfig>,
    ) -> Option<String> {
        Some(Self::reasoning_label(reasoning_effort))
    }

    pub(crate) fn token_usage(&self) -> crate::token_usage::TokenUsage {
        self.chat_widget.token_usage()
    }

    pub(super) fn on_update_reasoning_effort(&mut self, effort: Option<ReasoningEffortConfig>) {
        let clear_ephemeral_plan_effort = effort != Some(ReasoningEffortConfig::Ultra)
            && self.chat_widget.config_ref().plan_mode_reasoning_effort
                == Some(ReasoningEffortConfig::Ultra)
            && self.config.plan_mode_reasoning_effort != Some(ReasoningEffortConfig::Ultra);
        // TODO(aibrahim): Remove this and don't use config as a state object.
        // Instead, explicitly pass the stored collaboration mode's effort into new sessions.
        self.config.model_reasoning_effort = effort.clone();
        self.chat_widget.set_reasoning_effort(effort.clone());
        if clear_ephemeral_plan_effort {
            self.chat_widget.set_plan_mode_reasoning_effort(effort);
        }
    }

    pub(super) fn on_update_plan_mode_reasoning_effort(
        &mut self,
        effort: Option<ReasoningEffortConfig>,
    ) {
        let clear_ephemeral_default_effort = effort != Some(ReasoningEffortConfig::Ultra)
            && self
                .chat_widget
                .current_collaboration_mode()
                .reasoning_effort()
                == Some(ReasoningEffortConfig::Ultra)
            && self.config.model_reasoning_effort != Some(ReasoningEffortConfig::Ultra);
        self.config.plan_mode_reasoning_effort = effort.clone();
        self.chat_widget
            .set_plan_mode_reasoning_effort(effort.clone());
        if clear_ephemeral_default_effort {
            self.chat_widget.set_reasoning_effort(effort);
        }
    }

    pub(super) fn on_apply_advanced_reasoning(
        &mut self,
        model: &str,
        effort: ReasoningEffortConfig,
    ) -> Option<ReasoningEffortConfig> {
        let default_effort = self.default_reasoning_effort_for_conversation_model(model);
        if let Some(default_effort) = default_effort.as_ref() {
            self.config.model = Some(model.to_string());
            self.config.model_reasoning_effort = Some(default_effort.clone());
        }
        self.chat_widget.set_model(model);
        self.chat_widget.set_reasoning_effort(Some(effort.clone()));
        self.chat_widget
            .set_plan_mode_reasoning_effort(Some(effort));
        default_effort
    }

    fn default_reasoning_effort_for_conversation_model(
        &self,
        model: &str,
    ) -> Option<ReasoningEffortConfig> {
        let configured_effort = self
            .config
            .model_reasoning_effort
            .as_ref()
            .filter(|effort| **effort != ReasoningEffortConfig::Ultra);
        let preset = self
            .model_catalog
            .try_list_models()
            .ok()?
            .into_iter()
            .find(|preset| preset.model == model)?;
        let supported = &preset.supported_reasoning_efforts;

        configured_effort
            .filter(|effort| supported.iter().any(|option| option.effort == **effort))
            .cloned()
            .or_else(|| {
                (preset.default_reasoning_effort != ReasoningEffortConfig::Ultra)
                    .then_some(preset.default_reasoning_effort)
            })
            .or_else(|| {
                supported
                    .iter()
                    .find(|option| option.effort != ReasoningEffortConfig::Ultra)
                    .map(|option| option.effort.clone())
            })
    }

    pub(super) fn resume_model_settings(&self) -> crate::app_server_session::ResumeModelSettings {
        resume_model_settings_for_overrides(&self.config, &self.harness_overrides)
    }

    pub(super) fn reject_remote_resume_permission_override(&mut self, config: &Config) -> bool {
        if self.app_server_target.thread_params_mode()
            != crate::app_server_session::ThreadParamsMode::Remote
        {
            return false;
        }
        let explicitly_selected =
            has_explicit_resume_permission_override(config, &self.harness_overrides)
                || matches!(
                    self.runtime_approval_policy_override,
                    Some(RuntimeApprovalPolicyOverride::Explicit(_))
                )
                || self
                    .runtime_permission_profile_override
                    .as_ref()
                    .is_some_and(|profile| {
                        profile.turn_override == RuntimePermissionProfileTurnOverride::LegacySandbox
                    });
        if explicitly_selected {
            self.add_agents_overview_error(
                "Permission overrides are not supported when resuming a remote task.".into(),
            );
            return true;
        }
        false
    }

    pub(super) fn sync_tui_theme_selection(&mut self, name: String) {
        self.local_settings.tui.theme = Some(name.clone());
        self.chat_widget.set_tui_theme(Some(name));
    }

    pub(super) fn restore_runtime_theme_from_config(&self) {
        if let Some(name) = self.local_settings.tui.theme.as_deref()
            && let Some(theme) = crate::render::highlight::resolve_theme_by_name(
                name,
                Some(&self.local_settings.ruxex_home),
            )
        {
            crate::render::highlight::set_syntax_theme(theme);
            return;
        }

        let auto_theme_name = crate::render::highlight::adaptive_default_theme_name();
        if let Some(theme) = crate::render::highlight::resolve_theme_by_name(
            auto_theme_name,
            Some(&self.local_settings.ruxex_home),
        ) {
            crate::render::highlight::set_syntax_theme(theme);
        }
    }

    fn sync_feature_state_from_effective_config(
        &mut self,
        effective_config: &ConfigReadResponse,
        feature_updates: &[(Feature, bool)],
    ) {
        for (feature, _) in feature_updates {
            let enabled = feature_enabled_from_effective_config(effective_config, *feature);
            if let Err(err) = self.config.features.set_enabled(*feature, enabled) {
                tracing::warn!(error = %err, feature = feature.key(), "failed to sync effective feature state");
                continue;
            }
            self.chat_widget.set_feature_enabled(*feature, enabled);
        }
    }
}

pub(super) fn overridden_write_message(write_response: &ConfigWriteResponse) -> &str {
    write_response
        .overridden_metadata
        .as_ref()
        .map(|metadata| metadata.message.as_str())
        .unwrap_or("the effective config is overridden by a higher-priority layer")
}

fn feature_enabled_from_effective_config(
    effective_config: &ConfigReadResponse,
    feature: Feature,
) -> bool {
    let root_features = effective_config
        .config
        .additional
        .get("features")
        .and_then(features_toml_from_json);
    root_features
        .as_ref()
        .and_then(|features| features.entries().get(feature.key()).copied())
        .unwrap_or_else(|| feature.default_enabled())
}

fn approval_policy_from_effective_config(
    effective_config: &ConfigReadResponse,
) -> Option<AskForApproval> {
    effective_config.config.approval_policy
}

fn sandbox_mode_from_effective_config(
    effective_config: &ConfigReadResponse,
) -> Option<AppServerSandboxMode> {
    effective_config.config.sandbox_mode
}

fn features_toml_from_json(value: &serde_json::Value) -> Option<FeaturesToml> {
    serde_json::from_value(value.clone()).ok()
}
