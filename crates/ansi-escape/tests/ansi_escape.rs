use ratatui::style::{Color, Modifier};

#[test]
fn converts_ansi_text_to_ruxex_ratatui_types() {
    let text = ruxex_ansi_escape::ansi_escape("\x1b[31;1mred\x1b[0m");
    let span = &text.lines[0].spans[0];

    assert_eq!(span.content, "red");
    assert_eq!(span.style.fg, Some(Color::Red));
    assert!(span.style.add_modifier.contains(Modifier::BOLD));
}
