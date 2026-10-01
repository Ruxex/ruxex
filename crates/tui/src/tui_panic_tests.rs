//! Exercise caught-panic recovery through a real terminal and the production panic hook.

use std::fs::File;
use std::io::Read as _;
use std::io::Write as _;
use std::os::fd::FromRawFd;
use std::process::Command;
use std::process::Stdio;

use crossterm::cursor::SetCursorStyle;
use ratatui::layout::Rect;
use ratatui::style::Style;

const BEFORE_PANIC: &str = "\x1b]1337;codex-before-panic\x07";
const BEFORE_RECOVERY: &str = "\x1b]1337;codex-before-recovery\x07";
const BEFORE_VISIBLE_FRAME: &str = "\x1b]1337;codex-before-visible-frame\x07";
const AFTER_VISIBLE_FRAME: &str = "\x1b]1337;codex-after-visible-frame\x07";

#[tokio::test(flavor = "current_thread")]
#[ignore]
async fn caught_panic_pty_child() {
    if std::env::var_os("CODEX_TUI_CAUGHT_PANIC_PTY_CHILD").is_none() {
        return;
    }
    super::set_modes().expect("set terminal modes");
    super::set_panic_hook();
    let mut tui = super::test_support::make_test_tui().expect("create TUI");
    tui.terminal.set_viewport_area(Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 12, /*height*/ 2,
    ));

    draw(&mut tui, "before panic", Some((4, 1))).expect("draw initial visible cursor");
    draw(&mut tui, "before panic", /*cursor*/ None).expect("hide initial cursor");
    mark(BEFORE_PANIC).expect("mark before panic");
    assert!(std::panic::catch_unwind(|| panic!("simulate caught startup panic")).is_err());
    mark(BEFORE_RECOVERY).expect("mark before recovery");
    tui.recover_after_caught_panic()
        .expect("recover TUI after panic");
    draw(&mut tui, "after panic", /*cursor*/ None).expect("draw recovered hidden cursor");
    mark(BEFORE_VISIBLE_FRAME).expect("mark before visible frame");
    draw(&mut tui, "after panic", Some((4, 1))).expect("draw recovered visible cursor");
    mark(AFTER_VISIBLE_FRAME).expect("mark after visible frame");
    super::restore_after_exit().expect("restore terminal after test");
}

fn draw(tui: &mut super::Tui, text: &str, cursor: Option<(u16, u16)>) -> std::io::Result<()> {
    tui.terminal.draw(|frame| {
        frame.buffer_mut().set_string(0, 0, text, Style::default());
        if let Some(cursor) = cursor {
            frame.set_cursor_style(SetCursorStyle::SteadyBar);
            frame.set_cursor_position(cursor);
        }
    })
}

fn mark(marker: &str) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(marker.as_bytes())?;
    stdout.flush()
}
