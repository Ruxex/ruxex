//! Session-scoped shortcuts for the ordinary built-in permission modes.

use super::*;

impl ChatWidget {
    pub(super) fn handle_permission_shortcut(&mut self, key_event: KeyEvent) -> bool {
        let forward = if self.chat_keymap.next_permission_mode.is_pressed(key_event) {
            true
        } else if self
            .chat_keymap
            .previous_permission_mode
            .is_pressed(key_event)
        {
            false
        } else {
            return false;
        };
        if !self.bottom_pane.no_modal_or_popup_active() {
            return false;
        }
        if self.permission_shortcut_pending {
            return true;
        }
        if self.blocks_direct_input {
            self.add_error_message(PARENT_OWNED_INPUT_MESSAGE.to_string());
            return true;
        }
        let Some(thread_id) = self.thread_id else {
            return true;
        };

        let current_approval =
            AskForApproval::from(self.config.permissions.approval_policy.value());
        let active_profile = self.config.permissions.active_permission_profile();
        let mut choices = Vec::new();
        for preset in builtin_approval_presets() {
            if !matches!(preset.id, "read-only" | "auto") {
                continue;
            }
            let approval = AskForApproval::from(preset.approval);
            if self
                .permission_mode_disabled_reason(&preset, approval)
                .is_some()
            {
                continue;
            }
            // These modes still need explicit Windows setup.
            #[cfg(target_os = "windows")]
            if preset.id == "auto"
                && (matches!(
                    self.windows_sandbox_host,
                    crate::app::WindowsSandboxHost::Mixed | crate::app::WindowsSandboxHost::Unknown
                ) || (self.windows_sandbox_host != crate::app::WindowsSandboxHost::Remote
                    && self.windows_sandbox_config.level() == WindowsSandboxLevel::Disabled))
            {
                continue;
            }
            let is_current = current_approval == approval
                && active_profile.as_ref().map_or_else(
                    || {
                        Self::preset_matches_current(
                            current_approval,
                            self.config.permissions.permission_profile(),
                            self.config.cwd.as_path(),
                            &preset,
                        )
                    },
                    |active| active.id == preset.active_permission_profile.id,
                );
            let label = preset.label;
            choices.push((
                is_current,
                PermissionProfileSelection {
                    profile_id: preset.active_permission_profile.id.clone(),
                    approval_policy: Some(approval),
                    display_label: label.to_string(),
                },
            ));
        }
        if !forward {
            choices.reverse();
        }
        let start = choices
            .iter()
            .position(|(current, _)| *current)
            .map_or(0, |index| index + 1);
        if let Some((_, selection)) = choices
            .iter()
            .cycle()
            .skip(start)
            .take(choices.len())
            .find(|(current, _)| !current)
        {
            self.permission_shortcut_pending = true;
            self.app_event_tx.send(AppEvent::ApplyPermissionShortcut {
                thread_id,
                selection: selection.clone(),
            });
        } else {
            self.add_info_message(
                "No other permission modes are available.".to_string(),
                /*hint*/ None,
            );
        }
        true
    }

    pub(crate) fn complete_permission_shortcut(&mut self, thread_id: ThreadId) {
        if self.thread_id == Some(thread_id) {
            self.permission_shortcut_pending = false;
        }
    }
}
