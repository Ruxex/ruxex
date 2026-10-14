//! Pasted text edits the active history query while preserving the original draft.

use super::super::super::chat_composer_history::HistoryEntry;
use super::super::ChatComposer;
use super::super::InputResult;
use super::super::LARGE_PASTE_CHAR_THRESHOLD;
use crate::app_event_sender::AppEventSender;
use crate::render::renderable::Renderable;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc::unbounded_channel;

fn composer_with_history() -> ChatComposer {
    let (tx, _rx) = unbounded_channel();
    let mut composer = ChatComposer::new(
        /*has_input_focus*/ true,
        AppEventSender::new(tx),
        /*enhanced_keys_supported*/ false,
        "Ask Ruxex to do anything".to_string(),
        /*disable_paste_burst*/ false,
    );
    for entry in ["git status", "git log"] {
        composer
            .history
            .record_local_submission(HistoryEntry::new(entry.to_string()));
    }
    composer.set_text_content("draft".to_string(), Vec::new(), Vec::new());
    composer
}

fn render_composer(composer: &ChatComposer, width: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, /*height*/ 5)).unwrap();
    terminal
        .draw(|frame| composer.render(frame.area(), frame.buffer_mut()))
        .unwrap();
    terminal
}





#[test]
fn history_search_paste_preserves_original_draft_on_miss_and_cancel() {
    let mut composer = composer_with_history();
    composer.handle_paste("x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 1));
    composer.draft.textarea.set_cursor(/*pos*/ 2);
    let original_draft = composer.snapshot_draft();

    for cancel_key in [
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ] {
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        composer.handle_paste("git".to_string());
        assert_eq!(composer.draft.textarea.text(), "git log");

        composer.handle_paste(" missing".to_string());
        assert_eq!(composer.snapshot_draft(), original_draft);
        assert_eq!(
            composer.history_search.as_ref().unwrap().query,
            "git missing"
        );

        composer.handle_key_event(cancel_key);
        assert!(!composer.history_search_active());
        assert_eq!(composer.snapshot_draft(), original_draft);
    }
}

#[test]
fn history_search_paste_uses_full_sanitized_text() {
    let temp = tempfile::tempdir().unwrap();
    let image_path = temp.path().join("history.png");
    image::RgbaImage::new(/*width*/ 1, /*height*/ 1)
        .save(&image_path)
        .unwrap();
    let image_path = image_path.to_string_lossy().into_owned();
    let large_paste = "x".repeat(LARGE_PASTE_CHAR_THRESHOLD + 1);

    for (pasted, query) in [
        (image_path.clone(), image_path),
        (large_paste.clone(), large_paste),
        (
            "é\r\n中\r\x1b[31mtext\x1b[0m".to_string(),
            "é\n中\ntext".to_string(),
        ),
    ] {
        let mut composer = composer_with_history();
        composer
            .history
            .record_local_submission(HistoryEntry::new(query.clone()));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        composer.handle_paste(pasted);

        assert_eq!(composer.history_search.as_ref().unwrap().query, query);
        assert_eq!(composer.draft.textarea.text(), query);
        assert!(composer.attachments.local_image_paths().is_empty());
        assert!(composer.draft.pending_pastes.is_empty());
        assert!(composer.current_text_elements().is_empty());
    }
}
