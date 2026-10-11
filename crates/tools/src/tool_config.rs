use genex_features::Feature;
use genex_features::Features;
use genex_protocol::config_types::ModeKind;
use genex_protocol::config_types::TUI_VISIBLE_COLLABORATION_MODES;
pub fn request_user_input_available_modes(features: &Features) -> Vec<ModeKind> {
    TUI_VISIBLE_COLLABORATION_MODES
        .into_iter()
        .filter(|mode| {
            mode.allows_request_user_input()
                || (features.enabled(Feature::DefaultModeRequestUserInput)
                    && *mode == ModeKind::Default)
        })
        .collect()
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UnifiedExecShellMode {
    Direct,
}

impl UnifiedExecShellMode {
    pub fn for_session(
        _features: &Features,
    ) -> Self {
        Self::Direct
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEnvironmentMode {
    None,
    Single,
    Multiple,
}

impl ToolEnvironmentMode {
    pub fn from_count(count: usize) -> Self {
        match count {
            0 => Self::None,
            1 => Self::Single,
            _ => Self::Multiple,
        }
    }

    pub fn has_environment(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[cfg(test)]
#[path = "tool_config_tests.rs"]
mod tests;
