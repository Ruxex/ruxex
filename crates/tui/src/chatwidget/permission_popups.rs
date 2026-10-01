//! Permission and approval popup flows for `ChatWidget`.
//!
//! This module owns the generic permission pickers and confirmation surfaces;
//! Windows-specific sandbox prompting lives beside it in
//! `windows_sandbox_prompts`.

use super::*;
use genex_protocol::model_types::MODEL_SPECIALTY_CYBER;

impl ChatWidget {
    /// Open the permissions popup.
    pub(crate) fn open_approvals_popup(&mut self) {
        self.open_permissions_popup();
    }

    /// Open a popup to choose the permissions mode.
    pub(crate) fn open_permissions_popup(&mut self) {
        if self.config.explicit_permission_profile_mode
            || self.permission_profiles_menu_opened
            || self
                .config
                .permissions
                .active_permission_profile()
                .is_some_and(|profile| !profile.id.starts_with(':'))
        {
            self.request_permission_profiles();
            return;
        }

        self.open_legacy_permissions_popup();
    }

    pub(super) fn open_legacy_permissions_popup(&mut self) {
        let include_read_only = cfg!(target_os = "windows");
        let current_approval =
            AskForApproval::from(self.config.permissions.approval_policy.value());
        let current_permission_profile = self.config.permissions.permission_profile().clone();
        let mut items: Vec<SelectionItem> = Vec::new();
        let presets: Vec<ApprovalPreset> = builtin_approval_presets();

        #[cfg(target_os = "windows")]
        let windows_sandbox_level = self.windows_sandbox_config.level();
        #[cfg(target_os = "windows")]
        let windows_degraded_sandbox_enabled =
            matches!(windows_sandbox_level, WindowsSandboxLevel::RestrictedToken)
                && self.windows_sandbox_local_server
                && self.windows_sandbox_host == crate::app::WindowsSandboxHost::Local;
        #[cfg(not(target_os = "windows"))]
        let windows_degraded_sandbox_enabled = false;

        let show_elevate_sandbox_hint = windows_degraded_sandbox_enabled
            && self
                .windows_sandbox_config
                .allows(WindowsSandboxSetupMode::Elevated)
            && presets.iter().any(|preset| preset.id == "auto");

        for preset in presets.into_iter() {
            if !include_read_only && preset.id == "read-only" {
                continue;
            }
            let base_name = if preset.id == "auto" && windows_degraded_sandbox_enabled {
                format!("{ASK_FOR_APPROVAL_LABEL} (non-admin sandbox)")
            } else if preset.id == "auto" {
                ASK_FOR_APPROVAL_LABEL.to_string()
            } else {
                preset.label.to_string()
            };
            let base_description =
                Some(preset.description.replace(" (Identical to Agent mode)", ""));
            let approval_disabled_reason = match self
                .config
                .permissions
                .approval_policy
                .can_set(&preset.approval)
            {
                Ok(()) => None,
                Err(err) => Some(err.to_string()),
            };
            let default_disabled_reason = approval_disabled_reason.clone();
            let default_actions = self.permission_mode_actions(
                &preset,
                base_name.clone(),
                /*profile_selection*/ None,
                /*return_to_permissions*/ !include_read_only,
            );
            items.push(SelectionItem {
                name: base_name,
                description: base_description,
                is_current: Self::preset_matches_current(
                    current_approval,
                    &current_permission_profile,
                    self.config.cwd.as_path(),
                    &preset,
                ),
                actions: default_actions,
                dismiss_on_select: true,
                disabled_reason: default_disabled_reason,
                ..Default::default()
            });
        }

        let footer_note = show_elevate_sandbox_hint.then(|| {
            vec![
                "The non-admin sandbox protects your files and prevents network access under most circumstances. However, it carries greater risk if prompt injected. To upgrade to the default sandbox, run ".dim(),
                "/setup-default-sandbox".cyan(),
                ".".dim(),
            ]
            .into()
        });

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some("Update Model Permissions".to_string()),
            footer_note,
            footer_hint: Some(standard_popup_hint_line()),
            items,
            header: Box::new(()),
            ..Default::default()
        });
    }

    pub(super) fn approval_preset_actions(
        approval: AskForApproval,
        permission_profile: PermissionProfile,
        active_permission_profile: ActivePermissionProfile,
        label: String,
    ) -> Vec<SelectionAction> {
        vec![Box::new(move |tx| {
            tx.send(AppEvent::CodexOp(AppCommand::override_turn_context(
                /*cwd*/ None,
                Some(approval),
                Some(permission_profile.clone()),
                Some(active_permission_profile.clone()),
                /*model*/ None,
                /*effort*/ None,
                /*summary*/ None,
                /*service_tier*/ None,
                /*collaboration_mode*/ None,
            )));
            tx.send(AppEvent::UpdateAskForApprovalPolicy(approval));
            tx.send(AppEvent::UpdateActivePermissionProfile(
                active_permission_profile.clone(),
            ));
            tx.send(AppEvent::InsertHistoryCell(Box::new(
                history_cell::new_info_event(
                    format!("Permissions updated to {label}"),
                    /*hint*/ None,
                ),
            )));
        })]
    }

    pub(super) fn permission_profile_selection_actions(
        selection: PermissionProfileSelection,
    ) -> Vec<SelectionAction> {
        vec![Box::new(move |tx| {
            tx.send(AppEvent::SelectPermissionProfile(selection.clone()));
        })]
    }

    pub(super) fn permission_mode_actions(
        &self,
        preset: &ApprovalPreset,
        label: String,
        profile_selection: Option<PermissionProfileSelection>,
        return_to_permissions: bool,
    ) -> Vec<SelectionAction> {
        let profile_selection = profile_selection.or_else(|| {
            self.thread_id.map(|_| PermissionProfileSelection {
                profile_id: preset.active_permission_profile.id.clone(),
                approval_policy: Some(AskForApproval::from(preset.approval)),
                display_label: label.clone(),
            })
        });
        let apply_actions = || {
            profile_selection.clone().map_or_else(
                || {
                    Self::approval_preset_actions(
                        AskForApproval::from(preset.approval),
                        preset.permission_profile.clone(),
                        preset.active_permission_profile.clone(),
                        label.clone(),
                    )
                },
                Self::permission_profile_selection_actions,
            )
        };
        let requires_confirmation = preset.id == "full-access";
        #[cfg(target_os = "windows")]
        if preset.id == "auto"
            && matches!(
                self.windows_sandbox_host,
                crate::app::WindowsSandboxHost::Mixed | crate::app::WindowsSandboxHost::Unknown
            )
        {
            let preset = preset.clone();
            return vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenWindowsSandboxEnablePrompt {
                    preset: preset.clone(),
                    profile_selection: profile_selection.clone(),
                });
            })];
        }
        if requires_confirmation {
            let preset = preset.clone();
            return vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenFullAccessConfirmation {
                    preset: preset.clone(),
                    return_to_permissions,
                    profile_selection: profile_selection.clone(),
                });
            })];
        }
        apply_actions()
    }

    pub(super) fn preset_matches_current(
        current_approval: AskForApproval,
        current_permission_profile: &PermissionProfile,
        cwd: &std::path::Path,
        preset: &ApprovalPreset,
    ) -> bool {
        let preset_approval = AskForApproval::from(preset.approval);
        if current_approval != preset_approval {
            return false;
        }

        match preset.id {
            "full-access" => matches!(current_permission_profile, PermissionProfile::Disabled),
            "read-only" => {
                let file_system_policy = current_permission_profile.file_system_sandbox_policy();
                matches!(
                    current_permission_profile,
                    PermissionProfile::Managed { .. }
                ) && !file_system_policy.has_full_disk_write_access()
                    && file_system_policy
                        .get_writable_roots_with_cwd(cwd)
                        .is_empty()
                    && current_permission_profile.network_sandbox_policy()
                        == preset.permission_profile.network_sandbox_policy()
            }
            "auto" => {
                let file_system_policy = current_permission_profile.file_system_sandbox_policy();
                matches!(
                    current_permission_profile,
                    PermissionProfile::Managed { .. }
                ) && file_system_policy.can_write_local_path_with_cwd(cwd, cwd)
                    && !file_system_policy.has_full_disk_write_access()
            }
            _ => current_permission_profile == &preset.permission_profile,
        }
    }

    pub(crate) fn open_full_access_confirmation(
        &mut self,
        preset: ApprovalPreset,
        return_to_permissions: bool,
        profile_selection: Option<PermissionProfileSelection>,
    ) {
        let selected_name = preset.label.to_string();
        let approval = AskForApproval::from(preset.approval);
        let is_cyber_model = self.model_catalog.try_list_models().is_ok_and(|models| {
            models.iter().any(|model| {
                model.model == self.current_model()
                    && model.model_specialty.as_deref() == Some(MODEL_SPECIALTY_CYBER)
            })
        });
        let title_line = Line::from("Enable full access?").bold();
        let info_lines = if is_cyber_model {
            let recommendation = "We strongly recommend selecting \"Ask for approval\" instead.";
            vec![
                Line::default(),
                Line::from(
                    "When Codex runs with full access, it can edit any file on your computer and run commands with network, without your approval.",
                ),
                Line::default(),
                Line::from(vec![
                    "Cyber models carry a higher risk of dangerous actions.".red(),
                    " Ensure proper safeguards are in place before granting full access. ".into(),
                    recommendation.into(),
                ]),
            ]
        } else {
            vec![Line::from(vec![
                "When Codex runs with full access, it can edit any file on your computer and run commands with network, without your approval. "
                    .into(),
                "Exercise caution when enabling full access. This significantly increases the risk of data loss, leaks, or unexpected behavior."
                    .red(),
            ])]
        };
        let header = Paragraph::new(
            std::iter::once(title_line)
                .chain(info_lines)
                .collect::<Vec<_>>(),
        )
        .wrap(Wrap { trim: false });

        let accept_actions = profile_selection.map_or_else(
            || {
                Self::approval_preset_actions(
                    approval,
                    preset.permission_profile,
                    preset.active_permission_profile,
                    selected_name,
                )
            },
            Self::permission_profile_selection_actions,
        );

        let deny_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            if return_to_permissions {
                tx.send(AppEvent::OpenPermissionsPopup);
            } else {
                tx.send(AppEvent::OpenApprovalsPopup);
            }
        })];

        let items = vec![
            SelectionItem {
                name: "Yes, continue anyway".to_string(),
                description: Some("Apply full access for this session".to_string()),
                actions: accept_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Cancel".to_string(),
                description: Some("Go back without enabling full access".to_string()),
                actions: deny_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
        ];

        self.bottom_pane.show_selection_view(SelectionViewParams {
            footer_hint: Some(standard_popup_hint_line()),
            items,
            header: Box::new(header),
            ..Default::default()
        });
    }
}
