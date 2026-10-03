use super::*;
use crate::app_server_session::ResumeModelSettings;
use crate::test_backend::VT100Backend;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::widgets::FrameExt as _;

fn prompt(action: SessionStartAction) -> UnarchivePrompt {
    UnarchivePrompt::new(
        ThreadId::from_string("019e72f4-e09a-70f2-b2c2-a153a57b8cc0").unwrap(),
        action,
    )
}

fn render(screen: &UnarchivePrompt) -> String {
    let mut terminal = Terminal::new(VT100Backend::new(/*width*/ 80, /*height*/ 10)).unwrap();
    terminal
        .draw(|frame| frame.render_widget_ref(screen, frame.area()))
        .unwrap();
    terminal
        .backend()
        .to_string()
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[test]
fn confirmation_requires_an_accept_key() {
    let mut screen = prompt(SessionStartAction::Fork(
        crate::app_server_session::ForkPermissionMode::InheritSaved,
    ));
    for key in [
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE),
    ] {
        assert_eq!(screen.handle_key(key), Some(UnarchiveChoice::Cancel));
    }
    for key in ['c', 'd'] {
        assert_eq!(
            screen.handle_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::CONTROL)),
            Some(UnarchiveChoice::Quit),
        );
    }
    assert_eq!(
        screen.handle_key(KeyEvent::new_with_kind(
            KeyCode::Enter,
            KeyModifiers::NONE,
            KeyEventKind::Release
        )),
        None
    );
    assert_eq!(
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(UnarchiveChoice::Unarchive)
    );
}

fn render_inline(width: u16) -> String {
    let screen = prompt(SessionStartAction::Fork(
        crate::app_server_session::ForkPermissionMode::InheritSaved,
    ));
    let height = screen.content().desired_height(width);
    let mut terminal = Terminal::new(VT100Backend::new(width, height + 2)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new("$ genex fork saved-session"),
                Rect::new(/*x*/ 0, /*y*/ 0, width, /*height*/ 1),
            );
            frame.render_widget_ref(&screen, Rect::new(/*x*/ 0, /*y*/ 2, width, height));
        })
        .unwrap();
    terminal
        .backend()
        .to_string()
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

#[tokio::test]
async fn inline_prompt_only_claims_content_height() -> Result<()> {
    let mut tui = crate::tui::test_support::make_test_tui()?;
    tui.set_alt_screen_enabled(/*enabled*/ false);
    let original = Rect::new(
        /*x*/ 0, /*y*/ 3, /*width*/ 80, /*height*/ 0,
    );
    tui.terminal.set_viewport_area(original);
    let screen = prompt(SessionStartAction::Fork(
        crate::app_server_session::ForkPermissionMode::InheritSaved,
    ));
    {
        let mut guard = PromptScreenGuard::enter(&mut tui)?;
        guard.draw(&screen)?;
        assert!(!guard.tui.is_alt_screen_active());
        assert_eq!(
            guard.tui.terminal.viewport_area,
            Rect {
                height: screen.content().desired_height(/*width*/ 80),
                ..original
            }
        );
    }
    assert_eq!(tui.terminal.viewport_area, original);
    Ok(())
}

#[tokio::test]
async fn alternate_prompt_restores_original_viewport() -> Result<()> {
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let original = Rect::new(
        /*x*/ 0, /*y*/ 3, /*width*/ 80, /*height*/ 0,
    );
    tui.terminal.set_viewport_area(original);
    {
        let mut guard = PromptScreenGuard::enter(&mut tui)?;
        guard.draw(&prompt(SessionStartAction::Fork(
            crate::app_server_session::ForkPermissionMode::InheritSaved,
        )))?;
        assert!(guard.tui.is_alt_screen_active());
        assert_eq!(
            guard.tui.terminal.viewport_area,
            Rect::new(
                /*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 24
            )
        );
    }
    assert!(!tui.is_alt_screen_active());
    assert_eq!(tui.terminal.viewport_area, original);
    Ok(())
}
