use super::*;
use pretty_assertions::assert_eq;

fn prompt(action: SessionStartAction) -> UnarchivePrompt {
    UnarchivePrompt::new(
        ThreadId::from_string("019e72f4-e09a-70f2-b2c2-a153a57b8cc0").unwrap(),
        action,
    )
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
