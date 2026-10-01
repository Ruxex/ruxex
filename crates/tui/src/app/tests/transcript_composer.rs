//! Regression coverage for transcript viewer input, prompt selection, and restoration.
//!
//! The default-off feature must leave the existing viewer and its draft intact.
//! Analytics also preserves the composer and stays separate from transcript backtracking.

use super::*;
use crate::test_support::test_path_display;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    buffer
        .content()
        .chunks(usize::from(buffer.area.width))
        .map(|row| {
            row.iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn press_key(
    app: &mut App,
    tui: &mut crate::tui::Tui,
    app_server: &mut AppServerSession,
    code: KeyCode,
) -> Result<()> {
    app.handle_tui_event(
        tui,
        app_server,
        TuiEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn analytics_preserves_the_draft_and_retains_the_view_on_reopen() -> Result<()> {
    let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
    let mut app_server = start_config_write_test_app_server(&app).await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    app.chat_widget
        .apply_external_edit("preserved draft".into());
    app.handle_event(
        &mut tui,
        &mut app_server,
        AppEvent::OpenAnalytics { view: None },
    )
    .await?;
    for code in [
        KeyCode::Char('4'),
        KeyCode::Enter,
        KeyCode::Down,
        KeyCode::Char('r'),
    ] {
        press_key(&mut app, &mut tui, &mut app_server, code).await?;
        assert!(matches!(app.overlay, Some(Overlay::Analytics(_))));
        assert!(!app.backtrack.overlay_preview_active);
    }
    press_key(&mut app, &mut tui, &mut app_server, KeyCode::Char('q')).await?;
    assert!(app.overlay.is_none());
    assert!(app.retained_analytics.is_some());
    app.handle_event(
        &mut tui,
        &mut app_server,
        AppEvent::OpenAnalytics { view: None },
    )
    .await?;
    assert!(matches!(app.overlay, Some(Overlay::Analytics(_))));
    assert!(app.retained_analytics.is_none());
    press_key(&mut app, &mut tui, &mut app_server, KeyCode::Char('q')).await?;
    assert_eq!(
        app.chat_widget.composer_text_with_pending(),
        "preserved draft"
    );
    Ok(())
}


#[tokio::test]
async fn analytics_keeps_queued_and_late_startup_history_out_of_the_overlay() -> Result<()> {
    for alternate_screen in [true, false] {
        let (mut app, _app_event_rx, _op_rx) = make_test_app_with_channels().await;
        let mut app_server = start_config_write_test_app_server(&app).await?;
        let mut tui = crate::tui::test_support::make_test_tui()?;
        tui.set_alt_screen_enabled(alternate_screen);
        app.insert_history_cell(
            &mut tui,
            Box::new(PlainHistoryCell::new(vec!["Startup header".into()])),
        );
        assert!(!tui.pending_history_lines_for_test().is_empty());
        app.handle_event(
            &mut tui,
            &mut app_server,
            AppEvent::OpenAnalytics { view: None },
        )
        .await?;
        assert!(tui.pending_history_lines_for_test().is_empty());
        app.insert_history_cell(
            &mut tui,
            Box::new(PlainHistoryCell::new(vec![
                "Late startup announcement".into(),
            ])),
        );
        assert!(tui.pending_history_lines_for_test().is_empty());
        let deferred = app.deferred_history_lines.clone();
        assert!(!deferred.is_empty());
        press_key(&mut app, &mut tui, &mut app_server, KeyCode::Char('q')).await?;
        assert_eq!(tui.pending_history_lines_for_test(), deferred);
        assert_eq!(app.transcript_cells.len(), 2);
    }
    Ok(())
}



#[tokio::test]
async fn transcript_viewer_close_repaints_preserved_inline_draft() -> Result<()> {
    assert_transcript_close_repaints_inline_draft(make_test_app().await).await
}
