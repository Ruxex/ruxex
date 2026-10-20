use super::truncate_line_word_boundary_with_ellipsis;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

#[test]
fn halfwidth_sound_marks_are_truncated_at_a_grapheme_boundary() {
    let line = Line::from("abｶﾞ tail");

    assert_eq!(
        truncate_line_word_boundary_with_ellipsis(line, /*max_width*/ 4),
        Line::from(vec![Span::raw("ab"), Span::raw("…")])
    );
}
