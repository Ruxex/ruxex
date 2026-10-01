use super::ScrollbackStrategy;
use crate::custom_terminal::Terminal;
use crate::insert_history::HistoryLineWrapPolicy;
use crate::insert_history::InsertHistoryMode;
use crate::insert_history::insert_history_lines_with_mode_and_wrap_policy;
use crate::test_backend::VT100Backend;
use codex_terminal_detection::Multiplexer;
use codex_terminal_detection::TerminalInfo;
use codex_terminal_detection::TerminalName;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::Print;
use pretty_assertions::assert_eq;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui::text::Line;

#[test]
fn windows_terminal_uses_full_screen_unless_zellij_is_active() {
    let mut terminal = TerminalInfo {
        name: TerminalName::WindowsTerminal,
        term_program: None,
        version: None,
        term: None,
        multiplexer: None,
    };
    let mut strategy = ScrollbackStrategy::detect(&terminal);

    assert_eq!(
        [
            strategy.history_insertion_mode(HistoryLineWrapPolicy::PreWrap),
            strategy.history_insertion_mode(HistoryLineWrapPolicy::Terminal),
        ],
        [InsertHistoryMode::FullScreen, InsertHistoryMode::FullScreen]
    );

    terminal.multiplexer = Some(Multiplexer::Zellij { version: None });
    strategy = ScrollbackStrategy::detect(&terminal);
    assert_eq!(strategy, ScrollbackStrategy::Zellij);
}

#[test]
fn zellij_only_uses_full_screen_insertion_for_terminal_wrapped_history() {
    assert_eq!(
        [
            ScrollbackStrategy::Zellij.history_insertion_mode(HistoryLineWrapPolicy::PreWrap),
            ScrollbackStrategy::Zellij.history_insertion_mode(HistoryLineWrapPolicy::Terminal),
        ],
        [InsertHistoryMode::Standard, InsertHistoryMode::FullScreen]
    );
}

#[test]
fn standard_viewport_growth_uses_scrollback_preserving_newlines() {
    for scroll_by in [1, 3, 4] {
        let screen_size = Size::new(/*width*/ 24, /*height*/ 6);
        let cursor = Position::new(/*x*/ 7, /*y*/ 5);
        let backend = CrosstermBackend::new(Vec::<u8>::new());
        let mut terminal =
            Terminal::with_screen_size_and_cursor_position_for_test(backend, screen_size, cursor);
        ScrollbackStrategy::Standard
            .grow_viewport(
                &mut terminal,
                /*viewport_top*/ 4,
                screen_size,
                scroll_by,
            )
            .expect("grow standard viewport");
        assert_eq!(
            terminal.backend().writer(),
            format!(
                "\x1b[1;4r\x1b[4;1H{}\x1b[r\x1b[6;8H",
                "\r\n".repeat(usize::from(scroll_by))
            )
            .as_bytes()
        );
    }
}

#[test]
fn standard_viewport_growth_without_rows_is_cursor_neutral() {
    for (viewport_top, scroll_by) in [(4, 0), (0, 1)] {
        let backend = VT100Backend::new(/*width*/ 24, /*height*/ 6);
        let mut terminal = Terminal::with_options(backend).expect("terminal");
        let cursor = Position::new(/*x*/ 7, /*y*/ 5);
        terminal
            .set_cursor_position(cursor)
            .expect("position cursor");
        let before = terminal.backend().vt100().screen().state_formatted();
        ScrollbackStrategy::Standard
            .grow_viewport(
                &mut terminal,
                viewport_top,
                Size::new(/*width*/ 24, /*height*/ 6),
                scroll_by,
            )
            .expect("no-op viewport growth");
        assert_eq!(
            terminal.backend().vt100().screen().state_formatted(),
            before
        );
    }
}
