use super::{PreviewError, PreviewViewport};
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use std::fmt::Write;

const CELL_WIDTH: u16 = 10;
const CELL_HEIGHT: u16 = 20;
const PREVIEW_BACKGROUND: &str = "#0d1117";

pub(super) fn buffer_to_svg(
    buffer: &Buffer,
    viewport: PreviewViewport,
) -> Result<String, PreviewError> {
    let width = u32::from(viewport.width) * u32::from(CELL_WIDTH);
    let height = u32::from(viewport.height) * u32::from(CELL_HEIGHT);
    let mut svg = String::new();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    )
    .map_err(|_| PreviewError::InvalidDesign("failed to build SVG".to_string()))?;
    svg.push_str("<style>text{font-family:monospace;font-size:14px;white-space:pre}</style>\n");
    // Keep the preview readable when opened on a light page or image viewer.
    svg.push_str(&format!(
        r#"<rect width="100%" height="100%" fill="{PREVIEW_BACKGROUND}"/>"#
    ));
    svg.push('\n');

    for y in 0..viewport.height {
        for x in 0..viewport.width {
            let cell = buffer.cell((x, y)).ok_or_else(|| {
                PreviewError::InvalidDesign("rendered buffer did not match viewport".to_string())
            })?;
            let (_, bg) = effective_colors(cell);
            if let Some(color) = css_color(bg) {
                writeln!(
                    svg,
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                    u32::from(x) * u32::from(CELL_WIDTH),
                    u32::from(y) * u32::from(CELL_HEIGHT),
                    CELL_WIDTH,
                    CELL_HEIGHT,
                    color,
                )
                .map_err(|_| PreviewError::InvalidDesign("failed to build SVG".to_string()))?;
            }
        }
    }

    for y in 0..viewport.height {
        let mut x = 0u16;
        while x < viewport.width {
            let cell = buffer.cell((x, y)).ok_or_else(|| {
                PreviewError::InvalidDesign("rendered buffer did not match viewport".to_string())
            })?;
            let (fg, bg) = effective_colors(cell);
            if cell.symbol() != " " && cell.symbol() != "" {
                let start_x = x;
                let mut text = String::new();
                let modifier = cell.modifier;
                let mut end_x = x;
                let mut last_visible_x = x;
                while end_x < viewport.width {
                    let current = buffer.cell((end_x, y)).ok_or_else(|| {
                        PreviewError::InvalidDesign(
                            "rendered buffer did not match viewport".to_string(),
                        )
                    })?;
                    let (current_fg, current_bg) = effective_colors(current);
                    if current_fg != fg || current_bg != bg || current.modifier != modifier {
                        break;
                    }
                    if current.symbol() != " " && !current.symbol().is_empty() {
                        last_visible_x = end_x;
                    }
                    end_x = end_x.saturating_add(1);
                }
                for position in start_x..=last_visible_x {
                    let symbol = buffer
                        .cell((position, y))
                        .ok_or_else(|| {
                            PreviewError::InvalidDesign(
                                "rendered buffer did not match viewport".to_string(),
                            )
                        })?
                        .symbol();
                    if symbol == " " || symbol.is_empty() {
                        text.push(' ');
                    } else {
                        text.push_str(symbol);
                    }
                }
                x = last_visible_x.saturating_add(1);
                let escaped = escape_xml(&text);
                let style = style_attributes(fg, modifier);
                writeln!(
                    svg,
                    r#"<text x="{}" y="{}"{}>{}</text>"#,
                    u32::from(start_x) * u32::from(CELL_WIDTH),
                    (u32::from(y) + 1) * u32::from(CELL_HEIGHT) - 4,
                    style,
                    escaped
                )
                .map_err(|_| PreviewError::InvalidDesign("failed to build SVG".to_string()))?;
                continue;
            }
            x = x.saturating_add(1);
        }
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}

fn style_attributes(fg: Color, modifier: Modifier) -> String {
    let mut style = String::new();
    if let Some(color) = css_color(fg) {
        write!(style, r##" fill="{color}""##).expect("writing to String cannot fail");
    }
    if modifier.contains(Modifier::BOLD) {
        style.push_str(r##" font-weight="bold""##);
    }
    if modifier.contains(Modifier::ITALIC) {
        style.push_str(r##" font-style="italic""##);
    }
    if modifier.contains(Modifier::DIM) {
        style.push_str(r##" opacity="0.65""##);
    }
    let mut decorations = Vec::new();
    if modifier.contains(Modifier::UNDERLINED) {
        decorations.push("underline");
    }
    if modifier.contains(Modifier::CROSSED_OUT) {
        decorations.push("line-through");
    }
    if !decorations.is_empty() {
        write!(style, r##" text-decoration="{}""##, decorations.join(" "))
            .expect("writing to String cannot fail");
    }
    style
}

fn effective_colors(cell: &ratatui::buffer::Cell) -> (Color, Color) {
    if cell.modifier.contains(Modifier::REVERSED) {
        (cell.bg, cell.fg)
    } else {
        (cell.fg, cell.bg)
    }
}

fn css_color(color: Color) -> Option<String> {
    match color {
        Color::Reset => None,
        Color::Black => Some("#000000".to_string()),
        Color::Red => Some("#800000".to_string()),
        Color::Green => Some("#008000".to_string()),
        Color::Yellow => Some("#808000".to_string()),
        Color::Blue => Some("#000080".to_string()),
        Color::Magenta => Some("#800080".to_string()),
        Color::Cyan => Some("#008080".to_string()),
        Color::Gray => Some("#c0c0c0".to_string()),
        Color::DarkGray => Some("#808080".to_string()),
        Color::LightRed => Some("#ff0000".to_string()),
        Color::LightGreen => Some("#00ff00".to_string()),
        Color::LightYellow => Some("#ffff00".to_string()),
        Color::LightBlue => Some("#0000ff".to_string()),
        Color::LightMagenta => Some("#ff00ff".to_string()),
        Color::LightCyan => Some("#00ffff".to_string()),
        Color::White => Some("#ffffff".to_string()),
        Color::Indexed(index) => Some(format!("var(--ansi-{index})")),
        Color::Rgb(r, g, b) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::super::{PreviewDesign, PreviewScene, PreviewViewport, render_preview};

    #[test]
    fn svg_escapes_text_and_preserves_theme_styles() {
        let design = PreviewDesign::from_toml(
            r##"
version = 1

[brand]
product_name = "A <Brand> & Co"
welcome_art = ["<tag>&"]

[theme.colors]
text = "#e6edf3"
accent = "#58a6ff"

[theme.symbols]
pointer = ">"
"##,
        )
        .expect("valid design");
        let svg = render_preview(
            &design,
            PreviewScene::Welcome,
            PreviewViewport {
                width: 80,
                height: 30,
            },
        )
        .expect("render SVG");

        assert!(svg.contains("&lt;") || svg.contains("&amp;"));
        assert!(!svg.contains("\\x1b"));

        let conversation = render_preview(
            &design,
            PreviewScene::Conversation,
            PreviewViewport {
                width: 80,
                height: 30,
            },
        )
        .expect("render conversation SVG");
        assert!(conversation.contains("#58a6ff"));
        assert!(conversation.contains("font-weight=\"bold\""));
    }

    #[test]
    fn svg_has_a_dark_preview_canvas_background() {
        let design = PreviewDesign::from_toml("version = 1\n").unwrap();
        let svg = render_preview(
            &design,
            PreviewScene::Status,
            PreviewViewport {
                width: 20,
                height: 8,
            },
        )
        .unwrap();

        let background = r##"<rect width="100%" height="100%" fill="#0d1117"/>"##;
        assert!(svg.contains(background));
        assert!(svg.find(background).unwrap() < svg.find("<text").unwrap());
    }
}
