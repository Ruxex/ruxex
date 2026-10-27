//! Startup warnings, model migration prompts, and bootstrap prompt helpers.
//!
//! These helpers run before or during `App::run` bootstrap. They translate configuration and model
//! catalog state into one-time TUI prompts or warning cells without owning the main event loop.

use super::*;
use ruxex_config::ConfigLayerSource;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Hash)]
struct SkillLoadWarningKey {
    path: PathBuf,
    message: String,
}

#[derive(Debug, Default)]
pub(super) struct SkillLoadWarningState {
    active: HashSet<SkillLoadWarningKey>,
    // Both session attachment and the background startup fetch can finish the initial load.
    pub(super) startup_complete: bool,
}

impl SkillLoadWarningState {
    pub(super) fn clear(&mut self) {
        self.active.clear();
    }

    pub(super) fn newly_active_errors(&mut self, errors: &[SkillErrorInfo]) -> Vec<SkillErrorInfo> {
        let previous = std::mem::take(&mut self.active);
        let mut current = HashSet::new();
        let mut newly_active = Vec::new();

        for error in errors {
            let key = SkillLoadWarningKey {
                path: error.path.clone(),
                message: error.message.clone(),
            };
            let was_active = previous.contains(&key);
            if current.insert(key) && !was_active {
                newly_active.push(error.clone());
            }
        }

        self.active = current;
        newly_active
    }
}

pub(super) fn skill_load_warning_messages(errors: &[SkillErrorInfo]) -> Vec<String> {
    if errors.is_empty() {
        return Vec::new();
    }

    let error_count = errors.len();
    let mut messages = vec![format!(
        "Skipped loading {error_count} skill(s) due to invalid SKILL.md files."
    )];
    messages.extend(
        errors
            .iter()
            .map(|error| format!("{}: {}", error.path.display(), error.message)),
    );
    messages
}

pub(super) fn project_config_warning(config: &Config) -> Option<String> {
    let mut disabled_folders = Vec::new();

    for layer in config.config_layer_stack.all_layers_low_to_high() {
        let ConfigLayerSource::Project { dot_ruxex_folder } = &layer.name else {
            continue;
        };
        let Some(disabled_reason) = &layer.disabled_reason else {
            continue;
        };
        disabled_folders.push((
            dot_ruxex_folder.as_path().display().to_string(),
            disabled_reason.clone(),
        ));
    }

    if disabled_folders.is_empty() {
        return None;
    }

    let mut message = concat!(
        "Project-local config, hooks, and exec policies are disabled in the following folders ",
        "until the project is trusted, but skills still load.\n",
    )
    .to_string();
    for (index, (folder, reason)) in disabled_folders.iter().enumerate() {
        let display_index = index + 1;
        message.push_str(&format!("    {display_index}. {folder}\n"));
        message.push_str(&format!("       {reason}\n"));
    }

    Some(message)
}

pub(super) fn emit_system_bwrap_warning(app_event_tx: &AppEventSender, config: &Config) {
    let Some(message) = ruxex_sandboxing::system_bwrap_warning(
        &config.permissions.effective_permission_profile(),
        &config.cwd,
    ) else {
        return;
    };

    app_event_tx.send(AppEvent::InsertHistoryCell(Box::new(
        history_cell::StartupWarningsCell::new(vec![message]),
    )));
}

pub(super) fn model_upgrade_for_migration(
    model: &str,
    available_models: &[ModelPreset],
) -> Option<ModelUpgrade> {
    if let Some(preset) = available_models.iter().find(|preset| preset.model == model) {
        return preset.upgrade.clone();
    }

    // Saved selections can outlive their catalog entries, but migration targets must come from
    // the mounted provider catalog rather than a built-in model mapping.
    None
}

pub(super) fn should_show_model_migration_prompt(
    current_model: &str,
    target_model: &str,
    seen_migrations: &BTreeMap<String, String>,
    available_models: &[ModelPreset],
) -> bool {
    if target_model == current_model {
        return false;
    }

    if let Some(seen_target) = seen_migrations.get(current_model)
        && seen_target == target_model
    {
        return false;
    }

    if !available_models
        .iter()
        .any(|preset| preset.model == target_model && preset.show_in_picker)
    {
        return false;
    }

    if model_upgrade_for_migration(current_model, available_models)
        .is_some_and(|upgrade| upgrade.id == target_model)
    {
        return true;
    }

    if available_models
        .iter()
        .any(|preset| preset.upgrade.as_ref().map(|u| u.id.as_str()) == Some(target_model))
    {
        return true;
    }

    false
}

pub(super) fn target_preset_for_upgrade<'a>(
    available_models: &'a [ModelPreset],
    target_model: &str,
) -> Option<&'a ModelPreset> {
    available_models
        .iter()
        .find(|preset| preset.model == target_model && preset.show_in_picker)
}

pub(super) fn apply_accepted_model_migration(
    config: &mut Config,
    app_event_tx: &AppEventSender,
    from_model: String,
    target_model: String,
    target_default_effort: ReasoningEffortConfig,
) {
    app_event_tx.send(AppEvent::PersistModelMigrationPromptAcknowledged {
        from_model,
        to_model: target_model.clone(),
    });

    config.model = Some(target_model.clone());
    config.model_reasoning_effort = Some(target_default_effort.clone());
    app_event_tx.send(AppEvent::UpdateModel(target_model.clone()));
    app_event_tx.send(AppEvent::UpdateReasoningEffort(Some(
        target_default_effort.clone(),
    )));
    app_event_tx.send(AppEvent::PersistModelSelection {
        model: target_model,
        effort: Some(target_default_effort),
    });
}

pub(super) async fn handle_model_migration_prompt_if_needed(
    tui: &mut tui::Tui,
    config: &mut Config,
    local_settings: &crate::local_settings::LocalSettings,
    model: &str,
    app_event_tx: &AppEventSender,
    available_models: &[ModelPreset],
) -> std::io::Result<Option<AppExitInfo>> {
    let upgrade = model_upgrade_for_migration(model, available_models);

    if let Some(ModelUpgrade {
        id: target_model,
        model_link,
        upgrade_copy,
        migration_markdown,
        ..
    }) = upgrade.as_ref()
    {
        let target_model = target_model.to_string();
        if !should_show_model_migration_prompt(
            model,
            &target_model,
            &local_settings.notices.model_migrations,
            available_models,
        ) {
            return Ok(None);
        }

        let current_preset = available_models.iter().find(|preset| preset.model == model);
        let Some(target_preset) = target_preset_for_upgrade(available_models, &target_model) else {
            return Ok(None);
        };
        let target_display_name = target_preset.display_name.clone();
        let heading_label = if target_display_name == model {
            target_model.clone()
        } else {
            target_display_name.clone()
        };
        let target_description =
            (!target_preset.description.is_empty()).then(|| target_preset.description.clone());
        let can_opt_out = current_preset.is_some();
        let prompt_copy = migration_copy_for_models(
            model,
            &target_model,
            model_link.clone(),
            upgrade_copy.clone(),
            migration_markdown.clone(),
            heading_label,
            target_description,
            can_opt_out,
        );
        match run_model_migration_prompt(tui, prompt_copy).await? {
            ModelMigrationOutcome::Accepted => {
                apply_accepted_model_migration(
                    config,
                    app_event_tx,
                    model.to_string(),
                    target_model.clone(),
                    target_preset.default_reasoning_effort.clone(),
                );
            }
            ModelMigrationOutcome::Rejected => {
                app_event_tx.send(AppEvent::PersistModelMigrationPromptAcknowledged {
                    from_model: model.to_string(),
                    to_model: target_model.clone(),
                });
            }
            ModelMigrationOutcome::Exit => {
                return Ok(Some(AppExitInfo {
                    token_usage: TokenUsage::default(),
                    thread_id: None,
                    resume_hint: None,
                    disconnect_info: None,
                    exit_reason: ExitReason::UserRequested,
                }));
            }
        }
    }

    Ok(None)
}
pub(super) fn normalize_harness_overrides_for_cwd(
    mut overrides: ConfigOverrides,
    base_cwd: &AbsolutePathBuf,
) -> Result<ConfigOverrides> {
    if overrides.additional_writable_roots.is_empty() {
        return Ok(overrides);
    }

    let mut normalized = Vec::with_capacity(overrides.additional_writable_roots.len());
    for root in overrides.additional_writable_roots.drain(..) {
        let absolute = base_cwd.join(root);
        normalized.push(absolute.into_path_buf());
    }
    overrides.additional_writable_roots = normalized;
    Ok(overrides)
}
