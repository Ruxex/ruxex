//! Startup diagnostic ordering, aggregation, and transcript coverage.

use super::*;
use crate::history_cell::StartupWarningsCell;
use genex_app_server_protocol::McpServerStartupFailureReason;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc::UnboundedReceiver;

fn insert_warnings(app: &mut App, tui: &mut tui::Tui, messages: &[&str]) {
    app.insert_history_cell(
        tui,
        Box::new(StartupWarningsCell::new(
            messages.iter().map(|message| (*message).into()).collect(),
        )),
    );
}

fn insert_splash(app: &mut App, tui: &mut tui::Tui) {
    let session = test_thread_session(ThreadId::new(), app.config.cwd.to_path_buf());
    app.insert_history_cell(
        tui,
        Box::new(history_cell::new_session_info(
            &app.config,
            &app.local_settings,
            "gpt-test",
            &session,
            /*is_first_event*/ false,
            /*tooltip_override*/ None,
            /*auth_plan*/ None,
        )),
    );
}

fn drain_history(app: &mut App, tui: &mut tui::Tui, events: &mut UnboundedReceiver<AppEvent>) {
    while let Ok(event) = events.try_recv() {
        if let AppEvent::InsertHistoryCell(cell) = event {
            app.insert_history_cell(tui, cell);
        }
    }
}

#[tokio::test]
async fn startup_warnings_preserve_stream_repair_and_backtrack_selection() -> Result<()> {
    let mut app = crate::app::test_support::make_test_app().await;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    app.chat_widget.handle_thread_session(test_thread_session(
        ThreadId::new(),
        app.config.cwd.to_path_buf(),
    ));
    app.transcript_cells = vec![Arc::new(UserHistoryCell {
        message: "Selected prompt".into(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: Vec::new(),
        spoken: false,
    })];
    app.transcript_reflow.mark_resize_requested_during_stream();
    insert_warnings(&mut app, &mut tui, &["First warning"]);
    assert!(app.transcript_reflow.take_stream_finish_reflow_needed());
    // Recreate the insertion case with a backtracking overlay already open.
    app.transcript_cells.remove(0);
    app.backtrack.overlay_preview_active = true;
    app.open_transcript_overlay(&mut tui);
    app.apply_backtrack_selection_internal(/*nth_user_message*/ 0);
    insert_warnings(&mut app, &mut tui, &["Second warning"]);
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 80, /*height*/ 12,
    );
    let mut buffer = Buffer::empty(area);
    let Some(Overlay::Transcript(overlay)) = app.overlay.as_mut() else {
        panic!("expected transcript")
    };
    overlay.render(area, &mut buffer);
    let highlighted = area
        .positions()
        .filter(|position| {
            buffer[*position]
                .style()
                .add_modifier
                .contains(ratatui::style::Modifier::REVERSED)
        })
        .map(|position| buffer[position].symbol())
        .collect::<String>();
    assert!(highlighted.contains("Selected prompt"), "{highlighted}");
    Ok(())
}
