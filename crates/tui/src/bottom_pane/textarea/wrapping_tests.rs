use super::super::TextArea;
use super::super::TextAreaState;
use super::visible_prefix;
use super::wrapped_lines;
use crate::width::display_width;
use crate::wrapping::RtOptions;
use crate::wrapping::adaptive_wrap_lines;
use codex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::StatefulWidgetRef;
use ratatui::widgets::WidgetRef;

fn wrapped_rows(text: &str, width: u16) -> Vec<&str> {
    wrapped_lines(text, width)
        .into_iter()
        .map(|range| &text[range.start..range.end - 1])
        .collect()
}

#[test]
fn vertical_navigation_preserves_destination_tab_columns() {
    let mut t = TextArea::new();
    t.insert_str("a\tb\nxyz");
    let area = Rect::new(0, 0, /*width*/ 8, /*height*/ 2);
    t.set_cursor(/*pos*/ 5);
    let _ = t.desired_height(area.width);
    t.move_cursor_up();
    assert_eq!((t.cursor(), t.cursor_pos(area)), (1, Some((1, 0))));
    t.move_cursor_down();
    assert_eq!((t.cursor(), t.cursor_pos(area)), (5, Some((1, 1))));
}


#[test]
fn visible_prefix_bounds_long_hanging_runs_without_splitting_graphemes() {
    let width = 80;
    let count = MAX_USER_INPUT_TEXT_CHARS - 2;
    let area = Rect::new(0, 0, width, /*height*/ 2);
    for separator in [" ", "\t", "\u{2003}", "\u{3000}"] {
        let text = format!("a{}x", separator.repeat(count));
        assert_eq!(
            visible_prefix(&text, width),
            format!("a{}", separator.repeat(usize::from(width)))
        );
        let mut t = TextArea::new();
        t.insert_str(&text);
        t.set_cursor(text.len() - 1 - separator.len());
        assert_eq!(t.cursor_pos(area), Some((0, 1)));
        assert_eq!(t.text(), text);
    }
    assert_eq!(
        visible_prefix("a  \u{301} hidden", /*width*/ 2),
        "a  \u{301}"
    );
}

#[test]
fn preserves_textwrap_word_boundaries() {
    for (text, width, expected_rows) in [
        ("a foo-barbaz", 10, ["a foo-", "barbaz"]),
        ("a foo-barbazqux", 10, ["a foo-", "barbazqux"]),
        ("a café-barbaz", 10, ["a café-", "barbaz"]),
        ("a foo/barbaz", 10, ["a foo/", "barbaz"]),
        ("a foo—barbaz", 10, ["a foo—", "barbaz"]),
        ("a abc\u{a0}de", 7, ["a ", "abc\u{a0}de"]),
        ("a abc\u{2007}de", 7, ["a ", "abc\u{2007}de"]),
        ("a abc\u{202f}de", 7, ["a ", "abc\u{202f}de"]),
        ("a \u{a0}abc", 5, ["a ", "\u{a0}abc"]),
    ] {
        let rows = wrapped_lines(text, width)
            .iter()
            .map(|range| &text[range.start..range.end - 1])
            .collect::<Vec<_>>();

        assert_eq!(rows, expected_rows, "text={text:?}, width={width}");
    }
}

#[test]
fn breakable_unicode_spaces_hang_before_following_words() {
    for (text, expected_rows) in [
        ("abad abcde", ["abad ", "abcd", "e"]),
        ("abad\u{2003}abcde", ["abad\u{2003}", "abcd", "e"]),
        (
            "abad\u{2003}\u{2003}abcde",
            ["abad\u{2003}\u{2003}", "abcd", "e"],
        ),
        ("abad\u{3000}abcde", ["abad\u{3000}", "abcd", "e"]),
        (
            "abad\u{3000} \u{2003}abcde",
            ["abad\u{3000} \u{2003}", "abcd", "e"],
        ),
    ] {
        assert_eq!(
            wrapped_rows(text, /*width*/ 4),
            expected_rows,
            "text={text:?}"
        );
    }
}

#[test]
fn hanging_spaces_preserve_source_editing_and_visual_navigation() {
    let text = "abad  next";
    let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
    let mut t = TextArea::new();
    t.insert_str(text);
    assert_eq!(t.desired_height(area.width), 3);
    assert_eq!(t.text(), text);

    for cursor in 4..=6 {
        t.set_cursor(cursor);
        assert_eq!(t.cursor_pos(area), Some((0, 1)));
    }
    t.move_cursor_left();
    assert_eq!(t.cursor(), 5);
    t.move_cursor_up();
    assert_eq!(t.cursor(), 0);
    t.move_cursor_down();
    assert_eq!(t.cursor(), 6);
    t.delete_backward(/*n*/ 1);
    assert_eq!(t.text(), "abad next");
    assert_eq!(t.cursor_pos(area), Some((0, 1)));
    t.delete_backward(/*n*/ 1);
    assert_eq!(t.text(), "abadnext");
    assert_eq!(t.cursor_pos(area), Some((0, 1)));
}




#[test]
fn explicit_indentation_and_trailing_spaces_do_not_hang() {
    for (text, expected_rows) in [
        ("abad\n a", vec!["abad", "", " a"]),
        ("abad \n a", vec!["abad", " ", " a"]),
        ("    a", vec!["    ", "a"]),
        ("abad  ", vec!["abad", "  "]),
        ("abad\u{2003}", vec!["abad", "\u{2003}"]),
        ("abad\u{3000}\n a", vec!["abad", "\u{3000}", " a"]),
    ] {
        assert_eq!(
            wrapped_rows(text, /*width*/ 4),
            expected_rows,
            "text={text:?}"
        );
    }
}

#[test]
fn nonbreaking_spaces_never_hang() {
    for space in ['\u{a0}', '\u{2007}', '\u{202f}'] {
        let text = format!("abcd{space} x");
        let last = format!("{space} x");
        assert_eq!(wrapped_rows(&text, /*width*/ 4), ["abcd", last.as_str()]);
    }
}

#[test]
fn prose_wraps_like_the_queued_input_preview() {
    for text in [
        "new access tokens",
        "aaaa   next",
        "a foo-barbaz",
        "some ordinary prose",
    ] {
        for width in 3..=16 {
            let composer = wrapped_rows(text, width)
                .into_iter()
                .map(|row| row.trim_end_matches(' '))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            let preview =
                adaptive_wrap_lines([Line::from(text)], RtOptions::new(usize::from(width)))
                    .into_iter()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>();
            assert_eq!(composer, preview, "text={text:?}, width={width}");
        }
    }
}

#[test]
fn wraps_maximum_length_unbroken_word_in_one_pass() {
    let text = "x".repeat(MAX_USER_INPUT_TEXT_CHARS);
    let width = 80;
    let ranges = wrapped_lines(&text, width);

    assert_eq!(
        ranges.len(),
        MAX_USER_INPUT_TEXT_CHARS.div_ceil(usize::from(width))
    );
    assert_eq!(ranges.first(), Some(&(0..usize::from(width) + 1)));
    assert_eq!(
        ranges.last().map(|range| range.end),
        Some(MAX_USER_INPUT_TEXT_CHARS + 1)
    );
}

#[test]
fn ascii_wrapped_rows_fit_and_preserve_cursor_positions() {
    for len in 0_u32..=7 {
        for mut encoded in 0..4_usize.pow(len) {
            let mut text = String::with_capacity(len as usize);
            for _ in 0..len {
                text.push([' ', 'a', 'b', '-'][encoded % 4]);
                encoded /= 4;
            }

            for width in 1_u16..=5 {
                let mut t = TextArea::new();
                t.insert_str(&text);
                let ranges = t.wrapped_lines(width).to_vec();
                let mut end = 0;
                for range in &ranges {
                    assert_eq!(range.start, end, "text={text:?}, width={width}");
                    end = range.end - 1;
                    let row = &text[range.start..end];
                    assert!(
                        display_width(row.trim_end_matches(' ')) <= usize::from(width),
                        "text={text:?}, width={width}, row={row:?}"
                    );
                }
                assert_eq!(end, text.len(), "text={text:?}, width={width}");

                let area = Rect::new(0, 0, width, text.len() as u16 + 1);
                let mut previous: Option<(u16, u16)> = None;
                for cursor in 0..=text.len() {
                    t.set_cursor(cursor);
                    let position = t.cursor_pos(area).unwrap();
                    if let Some(previous) = previous {
                        assert!(
                            position.1 > previous.1
                                || (position.1 == previous.1 && position.0 > previous.0)
                                || (position == previous && text.as_bytes()[cursor - 1] == b' '),
                            "text={text:?}, width={width}, ranges={ranges:?}, cursor={cursor}, previous={previous:?}, position={position:?}"
                        );
                    }
                    previous = Some(position);
                }
            }
        }
    }
}
