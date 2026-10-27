use super::PreviewError;
use crate::{TuiColor, TuiColors, TuiNamedColor, TuiSymbols, TuiTheme};
use serde::Deserialize;
use std::str::FromStr;

const MAX_DESIGN_BYTES: usize = 1024 * 1024;
const MAX_ART_LINES: usize = 32;
const MAX_ART_LINE_CHARS: usize = 400;
const MAX_TEXT_CHARS: usize = 400;
const MAX_SYMBOL_CHARS: usize = 16;
const MAX_VIEWPORT_WIDTH: u16 = 300;
const MAX_VIEWPORT_HEIGHT: u16 = 100;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PreviewBrand {
    pub product_name: Option<String>,
    pub product_slug: Option<String>,
    pub welcome_tagline: Option<String>,
    pub prompt_placeholder: Option<String>,
    pub side_prompt_placeholder: Option<String>,
    pub version_label: Option<String>,
    pub welcome_art: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewViewport {
    pub width: u16,
    pub height: u16,
}

impl PreviewViewport {
    pub fn validate(&self) -> Result<(), PreviewError> {
        if self.width == 0 || self.width > MAX_VIEWPORT_WIDTH {
            return Err(PreviewError::InvalidDesign(format!(
                "viewport width must be between 1 and {MAX_VIEWPORT_WIDTH}"
            )));
        }
        if self.height == 0 || self.height > MAX_VIEWPORT_HEIGHT {
            return Err(PreviewError::InvalidDesign(format!(
                "viewport height must be between 1 and {MAX_VIEWPORT_HEIGHT}"
            )));
        }
        Ok(())
    }
}

impl Default for PreviewViewport {
    fn default() -> Self {
        Self {
            width: 100,
            height: 30,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewScene {
    Welcome,
    Composer,
    Conversation,
    Status,
    All,
}

impl FromStr for PreviewScene {
    type Err = PreviewError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "welcome" => Ok(Self::Welcome),
            "composer" => Ok(Self::Composer),
            "conversation" => Ok(Self::Conversation),
            "status" => Ok(Self::Status),
            "all" => Ok(Self::All),
            _ => Err(PreviewError::InvalidDesign(format!(
                "unknown scene `{value}`"
            ))),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewDesign {
    pub version: u8,
    pub brand: PreviewBrand,
    pub theme: TuiTheme,
    pub viewport: PreviewViewport,
}

impl PreviewDesign {
    pub fn from_toml(input: &str) -> Result<Self, PreviewError> {
        if input.len() > MAX_DESIGN_BYTES {
            return Err(PreviewError::InvalidDesign(format!(
                "design is too large (maximum {MAX_DESIGN_BYTES} bytes)"
            )));
        }

        let raw: RawDesign = toml::from_str(input)
            .map_err(|error| PreviewError::InvalidDesign(error.to_string()))?;
        if raw.version != 1 {
            return Err(PreviewError::InvalidDesign(format!(
                "unsupported design version {}; expected 1",
                raw.version
            )));
        }

        let brand = raw.brand.into_brand()?;
        let theme = raw.theme.into_theme()?;
        let viewport = raw.viewport.into_viewport()?;

        Ok(Self {
            version: raw.version,
            brand,
            theme,
            viewport,
        })
    }

    pub fn scene_default(&self) -> PreviewScene {
        PreviewScene::All
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDesign {
    version: u8,
    #[serde(default)]
    brand: RawBrand,
    #[serde(default)]
    theme: RawTheme,
    #[serde(default)]
    viewport: RawViewport,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBrand {
    product_name: Option<String>,
    product_slug: Option<String>,
    welcome_tagline: Option<String>,
    prompt_placeholder: Option<String>,
    side_prompt_placeholder: Option<String>,
    version_label: Option<String>,
    #[serde(default)]
    welcome_art: Vec<String>,
}

impl RawBrand {
    fn into_brand(self) -> Result<PreviewBrand, PreviewError> {
        for (name, value) in [
            ("product_name", self.product_name.as_deref()),
            ("product_slug", self.product_slug.as_deref()),
            ("welcome_tagline", self.welcome_tagline.as_deref()),
            ("prompt_placeholder", self.prompt_placeholder.as_deref()),
            (
                "side_prompt_placeholder",
                self.side_prompt_placeholder.as_deref(),
            ),
            ("version_label", self.version_label.as_deref()),
        ] {
            if let Some(value) = value {
                validate_text(name, value, MAX_TEXT_CHARS)?;
            }
        }
        if self.welcome_art.len() > MAX_ART_LINES {
            return Err(PreviewError::InvalidDesign(format!(
                "welcome_art has too many lines (maximum {MAX_ART_LINES})"
            )));
        }
        for line in &self.welcome_art {
            validate_text("welcome_art line", line, MAX_ART_LINE_CHARS)?;
        }

        Ok(PreviewBrand {
            product_name: self.product_name,
            product_slug: self.product_slug,
            welcome_tagline: self.welcome_tagline,
            prompt_placeholder: self.prompt_placeholder,
            side_prompt_placeholder: self.side_prompt_placeholder,
            version_label: self.version_label,
            welcome_art: self.welcome_art,
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTheme {
    #[serde(default)]
    colors: RawColors,
    #[serde(default)]
    symbols: RawSymbols,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawColors {
    text: Option<String>,
    muted: Option<String>,
    accent: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    error: Option<String>,
    border: Option<String>,
    selection: Option<String>,
}

impl RawTheme {
    fn into_theme(self) -> Result<TuiTheme, PreviewError> {
        let mut colors = TuiColors::default();
        for (name, value, target) in [
            ("text", self.colors.text, &mut colors.text),
            ("muted", self.colors.muted, &mut colors.muted),
            ("accent", self.colors.accent, &mut colors.accent),
            ("success", self.colors.success, &mut colors.success),
            ("warning", self.colors.warning, &mut colors.warning),
            ("error", self.colors.error, &mut colors.error),
            ("border", self.colors.border, &mut colors.border),
            ("selection", self.colors.selection, &mut colors.selection),
        ] {
            if let Some(value) = value {
                *target = parse_color(name, &value)?;
            }
        }

        let mut symbols = TuiSymbols::default();
        for (name, value, target) in [
            ("pointer", self.symbols.pointer, &mut symbols.pointer),
            ("bullet", self.symbols.bullet, &mut symbols.bullet),
            ("success", self.symbols.success, &mut symbols.success),
            ("failure", self.symbols.failure, &mut symbols.failure),
            ("pending", self.symbols.pending, &mut symbols.pending),
            ("separator", self.symbols.separator, &mut symbols.separator),
            ("ellipsis", self.symbols.ellipsis, &mut symbols.ellipsis),
        ] {
            if let Some(value) = value {
                validate_text(name, &value, MAX_SYMBOL_CHARS)?;
                if value.is_empty() {
                    return Err(PreviewError::InvalidDesign(format!(
                        "theme.symbols.{name} must not be empty"
                    )));
                }
                *target = value;
            }
        }

        Ok(TuiTheme { colors, symbols })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSymbols {
    pointer: Option<String>,
    bullet: Option<String>,
    success: Option<String>,
    failure: Option<String>,
    pending: Option<String>,
    separator: Option<String>,
    ellipsis: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawViewport {
    width: Option<u16>,
    height: Option<u16>,
}

impl RawViewport {
    fn into_viewport(self) -> Result<PreviewViewport, PreviewError> {
        let viewport = PreviewViewport {
            width: self.width.unwrap_or(100),
            height: self.height.unwrap_or(30),
        };
        viewport.validate()?;
        Ok(viewport)
    }
}

fn validate_text(name: &str, value: &str, max_chars: usize) -> Result<(), PreviewError> {
    if value.chars().count() > max_chars {
        return Err(PreviewError::InvalidDesign(format!(
            "{name} is too long (maximum {max_chars} characters)"
        )));
    }
    Ok(())
}

fn parse_color(name: &str, value: &str) -> Result<TuiColor, PreviewError> {
    let normalized = value.trim().to_ascii_lowercase();
    let color = match normalized.as_str() {
        "default" => TuiColor::Default,
        "black" => TuiColor::Named(TuiNamedColor::Black),
        "red" => TuiColor::Named(TuiNamedColor::Red),
        "green" => TuiColor::Named(TuiNamedColor::Green),
        "yellow" => TuiColor::Named(TuiNamedColor::Yellow),
        "blue" => TuiColor::Named(TuiNamedColor::Blue),
        "magenta" => TuiColor::Named(TuiNamedColor::Magenta),
        "cyan" => TuiColor::Named(TuiNamedColor::Cyan),
        "white" | "gray" | "grey" => TuiColor::Named(TuiNamedColor::White),
        "bright-black" | "bright_black" | "dark-gray" | "dark_gray" => {
            TuiColor::Named(TuiNamedColor::BrightBlack)
        }
        "bright-red" | "bright_red" => TuiColor::Named(TuiNamedColor::BrightRed),
        "bright-green" | "bright_green" => TuiColor::Named(TuiNamedColor::BrightGreen),
        "bright-yellow" | "bright_yellow" => TuiColor::Named(TuiNamedColor::BrightYellow),
        "bright-blue" | "bright_blue" => TuiColor::Named(TuiNamedColor::BrightBlue),
        "bright-magenta" | "bright_magenta" => TuiColor::Named(TuiNamedColor::BrightMagenta),
        "bright-cyan" | "bright_cyan" => TuiColor::Named(TuiNamedColor::BrightCyan),
        "bright-white" | "bright_white" => TuiColor::Named(TuiNamedColor::BrightWhite),
        value if value.starts_with("ansi:") => {
            let index = value[5..].parse::<u8>().map_err(|_| {
                PreviewError::InvalidDesign(format!(
                    "theme.colors.{name} has invalid ANSI index `{value}`"
                ))
            })?;
            TuiColor::Indexed(index)
        }
        value if value.len() == 7 && value.starts_with('#') => {
            let parse = |range: std::ops::Range<usize>| {
                u8::from_str_radix(&value[range], 16).map_err(|_| {
                    PreviewError::InvalidDesign(format!(
                        "theme.colors.{name} has invalid hex color `{value}`"
                    ))
                })
            };
            TuiColor::Rgb {
                r: parse(1..3)?,
                g: parse(3..5)?,
                b: parse(5..7)?,
            }
        }
        _ => {
            return Err(PreviewError::InvalidDesign(format!(
                "theme.colors.{name} has unknown color `{value}`"
            )));
        }
    };
    Ok(color)
}

#[cfg(test)]
mod tests {
    use super::super::{PreviewDesign, PreviewScene};
    use crate::{TuiColor, TuiNamedColor};

    #[test]
    fn parses_versioned_design_with_structured_theme() {
        let design = PreviewDesign::from_toml(
            r##"
version = 1

[brand]
product_name = "My Agent"
welcome_art = ["+----+", "| Hi |", "+----+"]

[theme.colors]
text = "#e6edf3"
accent = "ansi:39"

[theme.symbols]
pointer = ">"
success = "+"

[viewport]
width = 100
height = 30
"##,
        )
        .expect("valid preview design");

        assert_eq!(design.scene_default(), PreviewScene::All);
        assert_eq!(design.brand.product_name.as_deref(), Some("My Agent"));
        assert_eq!(design.brand.welcome_art.len(), 3);
        assert_eq!(
            design.theme.colors.text,
            TuiColor::Rgb {
                r: 230,
                g: 237,
                b: 243
            }
        );
        assert_eq!(design.theme.colors.accent, TuiColor::Indexed(39));
        assert_eq!(design.theme.symbols.pointer, ">");
        assert_eq!(design.viewport.width, 100);
        assert_eq!(design.viewport.height, 30);
    }

    #[test]
    fn accepts_named_default_and_indexed_colors() {
        let design = PreviewDesign::from_toml(
            r#"
version = 1

[theme.colors]
text = "default"
muted = "bright-black"
accent = "cyan"
selection = "ansi:201"
"#,
        )
        .expect("valid color design");

        assert_eq!(design.theme.colors.text, TuiColor::Default);
        assert_eq!(
            design.theme.colors.muted,
            TuiColor::Named(TuiNamedColor::BrightBlack)
        );
        assert_eq!(
            design.theme.colors.accent,
            TuiColor::Named(TuiNamedColor::Cyan)
        );
        assert_eq!(design.theme.colors.selection, TuiColor::Indexed(201));
    }

    #[test]
    fn missing_optional_theme_fields_keep_visual_defaults() {
        let design = PreviewDesign::from_toml("version = 1\n").expect("minimal design");

        assert_eq!(
            design.theme.colors.accent,
            crate::TuiColors::default().accent
        );
        assert_eq!(design.theme.symbols, crate::TuiSymbols::default());
        assert!(design.brand.product_name.is_none());
    }

    #[test]
    fn rejects_unknown_fields() {
        let error = PreviewDesign::from_toml("version = 1\n\n[theme.colors]\nacccent = \"red\"\n")
            .expect_err("typos must be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_invalid_dimensions() {
        let error = PreviewDesign::from_toml("version = 1\n\n[viewport]\nwidth = 0\nheight = 30\n")
            .expect_err("zero-width previews are invalid");

        assert!(error.to_string().contains("width"));
    }
}
