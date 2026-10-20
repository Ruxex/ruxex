use super::{PreviewDesign, PreviewError, PreviewScene, PreviewViewport};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

pub(super) fn render_scene(
    design: &PreviewDesign,
    scene: PreviewScene,
    viewport: PreviewViewport,
) -> Result<Buffer, PreviewError> {
    if viewport.width == 0 || viewport.height == 0 {
        return Err(PreviewError::InvalidDesign(
            "preview viewport must not be empty".to_string(),
        ));
    }

    let area = Rect::new(0, 0, viewport.width, viewport.height);
    let mut buffer = Buffer::empty(area);
    match scene {
        PreviewScene::Welcome => render_welcome(&mut buffer, area, design),
        PreviewScene::Composer => render_composer(&mut buffer, area, design),
        PreviewScene::Conversation => render_conversation(&mut buffer, area, design),
        PreviewScene::Status => render_status(&mut buffer, area, design),
        PreviewScene::All => render_all(&mut buffer, area, design),
    }
    Ok(buffer)
}

fn render_all(buffer: &mut Buffer, area: Rect, design: &PreviewDesign) {
    let theme = &design.theme;
    write_line(
        buffer,
        area,
        1,
        1,
        "WELCOME",
        Style::default().fg(theme.colors.accent.into()),
    );
    render_welcome_at(buffer, area, design, 1, 2);
    let composer_y = area.height.saturating_div(3);
    write_line(
        buffer,
        area,
        1,
        composer_y,
        "COMPOSER",
        Style::default().fg(theme.colors.accent.into()),
    );
    render_composer_at(buffer, area, design, composer_y.saturating_add(1));
    write_line(
        buffer,
        area,
        1,
        area.height.saturating_div(2),
        "CONVERSATION",
        Style::default().fg(theme.colors.accent.into()),
    );
    render_conversation_at(buffer, area, design, 1, area.height.saturating_div(2) + 1);
    render_status_at(buffer, area, design, area.height.saturating_sub(2));
}

fn render_welcome(buffer: &mut Buffer, area: Rect, design: &PreviewDesign) {
    write_line(
        buffer,
        area,
        1,
        0,
        "WELCOME",
        Style::default().fg(design.theme.colors.accent.into()),
    );
    render_welcome_at(buffer, area, design, 0, 1);
}

fn render_welcome_at(buffer: &mut Buffer, area: Rect, design: &PreviewDesign, x: u16, y: u16) {
    let theme = &design.theme;
    let brand = &design.brand;
    let name = brand.product_name.as_deref().unwrap_or("Ruxex");
    let tagline = brand
        .welcome_tagline
        .as_deref()
        .unwrap_or("A terminal-native agent runtime");
    let art = if brand.welcome_art.is_empty() {
        vec![format!("[{name}]")]
    } else {
        brand.welcome_art.clone()
    };

    for (offset, line) in art.iter().enumerate() {
        write_centered(
            buffer,
            area,
            x,
            y.saturating_add(offset as u16),
            line,
            theme.colors.text,
        );
    }
    write_centered(
        buffer,
        area,
        x,
        y.saturating_add(art.len() as u16 + 1),
        tagline,
        theme.colors.muted,
    );
    if let Some(version) = brand.version_label.as_deref() {
        write_centered(
            buffer,
            area,
            x,
            y.saturating_add(art.len() as u16 + 2),
            version,
            theme.colors.muted,
        );
    }
}

fn render_composer(buffer: &mut Buffer, area: Rect, design: &PreviewDesign) {
    write_line(
        buffer,
        area,
        1,
        0,
        "COMPOSER",
        Style::default().fg(design.theme.colors.accent.into()),
    );
    render_composer_at(buffer, area, design, 1);
}

fn render_composer_at(buffer: &mut Buffer, area: Rect, design: &PreviewDesign, y: u16) {
    let theme = &design.theme;
    let placeholder = design
        .brand
        .prompt_placeholder
        .as_deref()
        .unwrap_or("Describe a task or ask a question...");
    write_line(
        buffer,
        area,
        2,
        y,
        &format!("{} {placeholder}", theme.symbols.pointer),
        Style::default()
            .fg(theme.colors.muted.into())
            .add_modifier(Modifier::ITALIC),
    );
    write_line(
        buffer,
        area,
        1,
        y.saturating_add(2),
        &format!(
            "{} Enter to send  {} Esc to cancel",
            theme.symbols.separator, theme.symbols.ellipsis
        ),
        Style::default().fg(theme.colors.muted.into()),
    );
}

fn render_conversation(buffer: &mut Buffer, area: Rect, design: &PreviewDesign) {
    render_conversation_at(buffer, area, design, 0, 0);
}

fn render_conversation_at(buffer: &mut Buffer, area: Rect, design: &PreviewDesign, x: u16, y: u16) {
    let theme = &design.theme;
    write_line(
        buffer,
        area,
        x,
        y,
        &format!("{} You", theme.symbols.pointer),
        Style::default()
            .fg(theme.colors.accent.into())
            .add_modifier(Modifier::BOLD),
    );
    write_line(
        buffer,
        area,
        x,
        y.saturating_add(1),
        "  Create a small landing page for this project.",
        Style::default().fg(theme.colors.text.into()),
    );
    write_line(
        buffer,
        area,
        x,
        y.saturating_add(3),
        &format!("{} Assistant", theme.symbols.bullet),
        Style::default()
            .fg(theme.colors.success.into())
            .add_modifier(Modifier::BOLD),
    );
    write_line(
        buffer,
        area,
        x,
        y.saturating_add(4),
        "  I can help shape the layout and content.",
        Style::default().fg(theme.colors.text.into()),
    );
}

fn render_status(buffer: &mut Buffer, area: Rect, design: &PreviewDesign) {
    render_status_at(buffer, area, design, 0);
}

fn render_status_at(buffer: &mut Buffer, area: Rect, design: &PreviewDesign, y: u16) {
    let theme = &design.theme;
    let version = design.brand.version_label.as_deref().unwrap_or("v0.0.0");
    write_line(
        buffer,
        area,
        1,
        y,
        &format!(
            "{} Ready  {}  {} {}",
            theme.symbols.success, version, theme.symbols.separator, "main"
        ),
        Style::default().fg(theme.colors.success.into()),
    );
}

fn write_centered(
    buffer: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    text: &str,
    color: crate::TuiColor,
) {
    let width = text.chars().count() as u16;
    let centered_x = x.saturating_add(area.width.saturating_sub(width) / 2);
    write_line(
        buffer,
        area,
        centered_x,
        y,
        text,
        Style::default().fg(color.into()),
    );
}

fn write_line(buffer: &mut Buffer, area: Rect, x: u16, y: u16, text: &str, style: Style) {
    if x >= area.right() || y >= area.bottom() {
        return;
    }
    buffer.set_stringn(x, y, text, area.right().saturating_sub(x) as usize, style);
}

#[cfg(test)]
mod tests {
    use super::super::{PreviewDesign, PreviewScene, render_preview};

    fn design() -> PreviewDesign {
        PreviewDesign::from_toml(
            r##"
version = 1

[brand]
product_name = "Preview <Brand>"
welcome_tagline = "Try & see"
version_label = "v1.2.3"
welcome_art = ["<RUXEX>"]
prompt_placeholder = "Write something"

[theme.colors]
accent = "#58a6ff"
success = "#3fb950"

[theme.symbols]
pointer = ">"
"##,
        )
        .expect("valid design")
    }

    #[test]
    fn each_scene_contains_a_stable_marker() {
        for (scene, marker) in [
            (PreviewScene::Welcome, "WELCOME"),
            (PreviewScene::Composer, "Write something"),
            (PreviewScene::Conversation, "Assistant"),
            (PreviewScene::Status, "Ready"),
            (PreviewScene::All, "CONVERSATION"),
        ] {
            let svg = render_preview(
                &design(),
                scene,
                crate::preview::PreviewViewport {
                    width: 80,
                    height: 30,
                },
            )
            .expect("render scene");
            assert!(svg.contains(marker), "missing {marker} in {scene:?}");
        }
    }
}
