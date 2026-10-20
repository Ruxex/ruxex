//! Ruxex-owned copy and lightweight visual identity for the TUI.

use std::sync::Arc;

use crate::version::RUXEX_CLI_VERSION;

pub(crate) const PRODUCT_NAME: &str = "Ruxex";
pub(crate) const PRODUCT_SLUG: &str = "ruxex";

pub(crate) const WELCOME_ART: &[&str] = &[
    "+------------------+",
    "|      RUXEX       |",
    "+------------------+",
];

pub(crate) const WELCOME_TAGLINE: &str = "A terminal-native agent runtime";
pub(crate) const PROMPT_PLACEHOLDER: &str = "Describe a task or ask a question...";
pub(crate) const SIDE_PROMPT_PLACEHOLDER: &str = "Ask a side question...";
pub(crate) const STARTUP_GUIDE_INTRO: &str =
    "  To get started, describe a task or try one of these commands:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiNamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiColor {
    Default,
    Named(TuiNamedColor),
    Indexed(u8),
    Rgb { r: u8, g: u8, b: u8 },
}

impl From<TuiColor> for ratatui::style::Color {
    fn from(color: TuiColor) -> Self {
        use ratatui::style::Color;

        match color {
            TuiColor::Default => Color::Reset,
            TuiColor::Named(named) => match named {
                TuiNamedColor::Black => Color::Black,
                TuiNamedColor::Red => Color::Red,
                TuiNamedColor::Green => Color::Green,
                TuiNamedColor::Yellow => Color::Yellow,
                TuiNamedColor::Blue => Color::Blue,
                TuiNamedColor::Magenta => Color::Magenta,
                TuiNamedColor::Cyan => Color::Cyan,
                TuiNamedColor::White => Color::Gray,
                TuiNamedColor::BrightBlack => Color::DarkGray,
                TuiNamedColor::BrightRed => Color::LightRed,
                TuiNamedColor::BrightGreen => Color::LightGreen,
                TuiNamedColor::BrightYellow => Color::LightYellow,
                TuiNamedColor::BrightBlue => Color::LightBlue,
                TuiNamedColor::BrightMagenta => Color::LightMagenta,
                TuiNamedColor::BrightCyan => Color::LightCyan,
                TuiNamedColor::BrightWhite => Color::White,
            },
            TuiColor::Indexed(index) => Color::Indexed(index),
            TuiColor::Rgb { r, g, b } => Color::Rgb(r, g, b),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TuiColors {
    pub text: TuiColor,
    pub muted: TuiColor,
    pub accent: TuiColor,
    pub success: TuiColor,
    pub warning: TuiColor,
    pub error: TuiColor,
    pub border: TuiColor,
    pub selection: TuiColor,
}

impl Default for TuiColors {
    fn default() -> Self {
        Self {
            text: TuiColor::Default,
            muted: TuiColor::Named(TuiNamedColor::BrightBlack),
            accent: TuiColor::Named(TuiNamedColor::Cyan),
            success: TuiColor::Named(TuiNamedColor::Green),
            warning: TuiColor::Named(TuiNamedColor::Yellow),
            error: TuiColor::Named(TuiNamedColor::Red),
            border: TuiColor::Named(TuiNamedColor::BrightBlack),
            selection: TuiColor::Named(TuiNamedColor::Blue),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiSymbols {
    pub pointer: String,
    pub bullet: String,
    pub success: String,
    pub failure: String,
    pub pending: String,
    pub separator: String,
    pub ellipsis: String,
}

impl Default for TuiSymbols {
    fn default() -> Self {
        Self {
            pointer: "›".to_string(),
            bullet: "•".to_string(),
            success: "✓".to_string(),
            failure: "✗".to_string(),
            pending: "·".to_string(),
            separator: "─".to_string(),
            ellipsis: "…".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TuiTheme {
    pub colors: TuiColors,
    pub symbols: TuiSymbols,
}

/// Optional branding surfaces owned by the TUI.
///
/// Implementors may provide only the surfaces they want to customize. A `None` value omits that
/// surface from the relevant UI instead of falling back to Ruxex copy.
pub trait TuiBrandProfile: Send + Sync {
    fn product_name(&self) -> Option<&str> {
        None
    }

    fn product_slug(&self) -> Option<&str> {
        None
    }

    fn welcome_art(&self) -> Option<&[&str]> {
        None
    }

    fn welcome_tagline(&self) -> Option<&str> {
        None
    }

    fn prompt_placeholder(&self) -> Option<&str> {
        None
    }

    fn side_prompt_placeholder(&self) -> Option<&str> {
        None
    }

    fn startup_guide_intro(&self) -> Option<&str> {
        None
    }

    fn version_label(&self) -> Option<String> {
        None
    }

    fn visual_theme(&self) -> Option<&TuiTheme> {
        None
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultTuiBrandProfile;

impl TuiBrandProfile for DefaultTuiBrandProfile {
    fn product_name(&self) -> Option<&str> {
        Some(PRODUCT_NAME)
    }

    fn product_slug(&self) -> Option<&str> {
        Some(PRODUCT_SLUG)
    }

    fn welcome_art(&self) -> Option<&[&str]> {
        Some(WELCOME_ART)
    }

    fn welcome_tagline(&self) -> Option<&str> {
        Some(WELCOME_TAGLINE)
    }

    fn prompt_placeholder(&self) -> Option<&str> {
        Some(PROMPT_PLACEHOLDER)
    }

    fn side_prompt_placeholder(&self) -> Option<&str> {
        Some(SIDE_PROMPT_PLACEHOLDER)
    }

    fn startup_guide_intro(&self) -> Option<&str> {
        Some(STARTUP_GUIDE_INTRO)
    }

    fn version_label(&self) -> Option<String> {
        Some(format!("v{RUXEX_CLI_VERSION}"))
    }
}

pub fn default_tui_brand_profile() -> Arc<dyn TuiBrandProfile> {
    Arc::new(DefaultTuiBrandProfile)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SparseProfile;

    impl TuiBrandProfile for SparseProfile {}

    #[test]
    fn brand_assets_are_compact_and_versioned() {
        let profile = DefaultTuiBrandProfile;
        assert_eq!(WELCOME_ART.len(), 3);
        assert!(WELCOME_ART.iter().all(|line| line.is_ascii()));
        assert!(
            profile
                .version_label()
                .is_some_and(|version| version.starts_with('v'))
        );
        assert!(!PROMPT_PLACEHOLDER.is_empty());
        assert!(!SIDE_PROMPT_PLACEHOLDER.is_empty());
    }

    #[test]
    fn brand_profile_can_omit_optional_surfaces() {
        let profile = SparseProfile;
        assert!(profile.product_name().is_none());
        assert!(profile.welcome_art().is_none());
        assert!(profile.prompt_placeholder().is_none());
        assert!(profile.version_label().is_none());
        assert!(profile.visual_theme().is_none());
    }

    #[test]
    fn brand_profile_can_supply_a_structured_theme() {
        struct ThemedProfile {
            theme: TuiTheme,
        }

        impl TuiBrandProfile for ThemedProfile {
            fn visual_theme(&self) -> Option<&TuiTheme> {
                Some(&self.theme)
            }
        }

        let profile = ThemedProfile {
            theme: TuiTheme {
                symbols: TuiSymbols {
                    pointer: "→".to_string(),
                    ..TuiSymbols::default()
                },
                ..TuiTheme::default()
            },
        };
        assert_eq!(profile.visual_theme().unwrap().symbols.pointer, "→");
    }

    #[test]
    fn theme_colors_convert_to_ratatui_colors() {
        assert_eq!(
            ratatui::style::Color::from(TuiColor::Rgb { r: 1, g: 2, b: 3 }),
            ratatui::style::Color::Rgb(1, 2, 3)
        );
        assert_eq!(
            ratatui::style::Color::from(TuiColor::Indexed(42)),
            ratatui::style::Color::Indexed(42)
        );
    }
}
