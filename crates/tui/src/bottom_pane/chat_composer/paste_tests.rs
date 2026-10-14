//! Raw paste tabs remain draft text until the user explicitly submits or queues it.

use super::tests::new_test_composer;
use super::*;
use pretty_assertions::assert_eq;
use ratatui::Terminal;
use ratatui::backend::TestBackend;


#[test]
fn paste_burst_tabs_refresh_idle_timeout() {
    for first_char in ['x', '界'] {
        let (mut composer, _rx) = new_test_composer();
        let mut now = Instant::now();
        composer.handle_input_basic_with_time(KeyEvent::from(KeyCode::Char(first_char)), now);
        assert!(composer.handle_paste_tab(KeyEvent::from(KeyCode::Tab), now));

        for _ in 0..3 {
            now += PasteBurst::recommended_active_flush_delay() / 2;
            assert!(composer.handle_paste_tab(KeyEvent::from(KeyCode::Tab), now));
        }

        composer.handle_paste_burst_flush(now + PasteBurst::recommended_active_flush_delay());
        assert_eq!(composer.current_text(), format!("{first_char}\t\t\t\t"));
        assert!(!composer.handle_paste_tab(
            KeyEvent::from(KeyCode::Tab),
            now + PasteBurst::recommended_active_flush_delay()
        ));
    }
}

#[test]
fn paste_burst_tab_does_not_accept_a_completion() {
    let (mut composer, _rx) = new_test_composer();
    composer.insert_str("/");
    assert!(matches!(composer.popups.active, ActivePopup::Command(_)));
    composer
        .draft
        .paste_burst
        .begin_with_retro_grabbed("review this".to_string(), Instant::now());

    let (result, _) = composer.handle_key_event(KeyEvent::from(KeyCode::Tab));

    assert_eq!(result, InputResult::None);
    let pasted = composer
        .draft
        .paste_burst
        .flush_before_modified_input()
        .unwrap();
    composer.handle_paste(pasted);
    assert_eq!(composer.current_text(), "/review this\t");
}

#[test]
fn paste_burst_expired_before_tab_still_queues() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_task_running(/*running*/ true);
    composer.handle_input_basic_with_time(
        KeyEvent::from(KeyCode::Char('x')),
        Instant::now() - Duration::from_secs(/*secs*/ 1),
    );

    let (result, _) = composer.handle_key_event(KeyEvent::from(KeyCode::Tab));

    assert_eq!(
        result,
        InputResult::Queued {
            text: "x".to_string(),
            text_elements: Vec::new(),
            action: QueuedInputAction::Plain,
            pending_pastes: Vec::new(),
        }
    );
}

#[test]
fn paste_burst_modified_queue_binding_still_dispatches() {
    let (mut composer, _rx) = new_test_composer();
    composer.set_task_running(/*running*/ true);
    composer.queue_keys = vec![key_hint::ctrl(KeyCode::Char('q'))];
    composer.handle_input_basic_with_time(KeyEvent::from(KeyCode::Char('x')), Instant::now());

    let (result, _) =
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));

    assert_eq!(
        result,
        InputResult::Queued {
            text: "x".to_string(),
            text_elements: Vec::new(),
            action: QueuedInputAction::Plain,
            pending_pastes: Vec::new(),
        }
    );
}
