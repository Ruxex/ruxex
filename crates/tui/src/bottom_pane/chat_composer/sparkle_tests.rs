//! Covers the original starfield appearance, protected content, and composer eligibility.

use super::*;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::BottomPaneParams;
use crate::bottom_pane::chat_composer_history::HistoryEntry;
use crate::render::renderable::Renderable;
use crate::slash_command::SlashCommand;
use crate::terminal_palette::with_test_default_colors;
use crate::terminal_probe::DefaultColors;
use crate::tui::FrameRequester;
use pretty_assertions::assert_eq;
use ratatui::layout::Position;
use ratatui::style::Color;

use super::field::DOTS;

fn pane() -> BottomPane {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    BottomPane::new(BottomPaneParams {
        app_event_tx: AppEventSender::new(tx),
        frame_requester: FrameRequester::test_dummy(),
        has_input_focus: true,
        enhanced_keys_supported: false,
        placeholder_text: "Ask Genex to do anything".into(),
        disable_paste_burst: true,
        animations_enabled: true,
        skills: None,
    })
}

fn palette<T>(render: impl FnOnce() -> T) -> T {
    with_test_default_colors(
        DefaultColors {
            fg: (230, 216, 255),
            bg: (36, 27, 53),
        },
        render,
    )
}

fn enabled() -> Tui {
    Tui {
        animations: true,
        whimsy: true,
        ..Tui::default()
    }
}

fn draw(composer: &ChatComposer, width: u16, now: Instant) -> (Buffer, Rect) {
    let area = Rect::new(
        /*x*/ 0,
        /*y*/ 0,
        width,
        composer.desired_height(width),
    );
    let [padding, _, textarea, _] =
        composer.layout_areas_with_textarea_right_reserve(area, /*textarea_right_reserve*/ 0);
    let phase = composer.sparkle.phase.replace(Phase::Unarmed);
    let mut buffer = Buffer::empty(area);
    composer.render(area, &mut buffer);
    composer.sparkle.phase.set(phase);
    composer.render_sparkle_at(
        padding,
        textarea,
        composer.cursor_pos(area),
        now,
        &mut buffer,
    );
    (buffer, textarea)
}

fn text(buffer: &Buffer) -> String {
    buffer
        .content
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

fn dots(buffer: &Buffer) -> Vec<(u16, u16, String)> {
    buffer
        .content
        .iter()
        .enumerate()
        .filter(|&(_, cell)| DOTS.contains(&cell.symbol()))
        .map(|(index, cell)| {
            let (x, y) = buffer.pos_of(index);
            (x, y, cell.symbol().to_string())
        })
        .collect()
}

fn key(pane: &mut BottomPane, key: KeyCode) -> InputResult {
    pane.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE))
}

fn type_text(pane: &mut BottomPane, text: &str) {
    for ch in text.chars() {
        key(pane, KeyCode::Char(ch));
    }
}



#[test]
fn the_first_buffered_character_and_empty_escape_clear_visible_stars() {
    palette(|| {
        let now = Instant::now();
        for input in [
            KeyCode::Char('x'),
            KeyCode::Char('?'),
            KeyCode::Esc,
            KeyCode::Left,
        ] {
            let mut pane = pane();
            pane.composer.set_disable_paste_burst(/*disabled*/ false);
            pane.mark_fresh_task_for_sparkle("astra", &enabled());
            assert!(!dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
            key(&mut pane, input);
            assert!(dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
            assert!(pane.composer.current_text().is_empty());
            assert_eq!(
                pane.composer.sparkle.draft.get(),
                if input == KeyCode::Char('x') {
                    SparkleDraft::Dismissed
                } else {
                    SparkleDraft::Untouched
                }
            );
        }
    });
}





#[test]
fn an_attachment_ends_the_sparkle_even_after_the_composer_is_cleared() {
    palette(|| {
        let now = Instant::now();
        let mut pane = pane();
        pane.mark_fresh_task_for_sparkle("astra", &enabled());
        assert!(!dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
        pane.attach_image(crate::test_support::test_path_buf("/tmp/image.png"));
        assert!(!pane.composer.is_empty());
        assert!(dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
        pane.on_ctrl_c();
        assert!(pane.composer.is_empty());
        pane.select_sparkle_model("astra", &enabled());
        assert!(dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
    });
}




#[test]
fn cancelling_history_search_preserves_a_typed_command_despite_attachment_previews() {
    palette(|| {
        let now = Instant::now();
        let mut pane = pane();
        pane.mark_fresh_task_for_sparkle("example-model", &enabled());
        type_text(&mut pane, "/status");
        let mut entry = HistoryEntry::new("history attachment".into());
        entry.local_image_paths = vec![crate::test_support::test_path_buf("/tmp/image.png")];
        entry.remote_image_urls = vec!["https://example.com/image.png".into()];
        pane.composer.history.record_local_submission(entry);
        pane.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        type_text(&mut pane, "history");
        assert!(!pane.composer.local_image_paths().is_empty());
        assert!(!pane.composer.remote_image_urls().is_empty());
        assert_eq!(pane.composer.sparkle.draft.get(), SparkleDraft::Command);
        key(&mut pane, KeyCode::Esc);
        assert_eq!(pane.composer.current_text(), "/status");
        assert!(pane.composer.local_image_paths().is_empty());
        assert!(pane.composer.remote_image_urls().is_empty());
        assert!(matches!(
            key(&mut pane, KeyCode::Enter),
            InputResult::Command(SlashCommand::Status)
        ));
        pane.select_sparkle_model("astra", &enabled());
        assert!(!dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
    });
}

#[test]
fn replacing_a_draft_outside_the_history_preview_dismisses_sparkles_during_search() {
    let mut pane = pane();
    pane.mark_fresh_task_for_sparkle("example-model", &enabled());
    pane.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
    assert!(pane.composer.history_search_active());
    pane.composer
        .set_text_content("external draft".into(), Vec::new(), Vec::new());
    assert_eq!(pane.composer.sparkle.draft.get(), SparkleDraft::Dismissed);
    key(&mut pane, KeyCode::Esc);
    assert!(pane.composer.is_empty());
    assert_eq!(pane.composer.sparkle.draft.get(), SparkleDraft::Dismissed);
}

#[test]
fn arbitrary_commands_preserve_a_never_used_fresh_opportunity() {
    palette(|| {
        let now = Instant::now();
        for completion in [false, true] {
            let mut pane = pane();
            pane.mark_fresh_task_for_sparkle("example-model", &enabled());
            assert!(dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
            for command in ["/status", "/diff", "/pwd"] {
                type_text(&mut pane, command);
                assert!(matches!(
                    key(&mut pane, KeyCode::Enter),
                    InputResult::Command(_)
                ));
                assert_eq!(pane.composer.sparkle.draft.get(), SparkleDraft::Untouched);
            }
            type_text(&mut pane, if completion { "/mo" } else { "/model" });
            if completion {
                key(&mut pane, KeyCode::Tab);
            }
            assert!(matches!(
                key(&mut pane, KeyCode::Enter),
                InputResult::Command(SlashCommand::Model)
            ));
            assert_eq!(pane.composer.sparkle.draft.get(), SparkleDraft::Untouched);
            pane.select_sparkle_model("provider-model-a", &enabled());
            assert!(!dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
        }
        for input in ["hello", "/model extra", "/other", "漢", "/tmp/file"] {
            let mut pane = pane();
            pane.mark_fresh_task_for_sparkle("example-model", &enabled());
            type_text(&mut pane, input);
            pane.composer
                .set_text_content(String::new(), Vec::new(), Vec::new());
            pane.select_sparkle_model("provider-model-a", &enabled());
            assert_eq!(
                pane.composer.sparkle.draft.get(),
                SparkleDraft::Dismissed,
                "{input}"
            );
            assert!(
                dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty(),
                "{input}"
            );
        }
    });
}

#[test]
fn canceled_commands_remain_eligible_but_inserted_content_and_ordinary_paste_do_not() {
    palette(|| {
        let now = Instant::now();
        let mut provisional = pane();
        type_text(&mut provisional, "x");
        key(&mut provisional, KeyCode::Backspace);
        let snapshot = provisional.composer.draft_snapshot();
        assert!(snapshot.text.is_empty());
        let mut initialized = pane();
        initialized.mark_fresh_task_for_sparkle("astra", &enabled());
        initialized.inherit_startup_sparkle(snapshot.sparkle_draft);
        assert!(dots(&draw(&initialized.composer, /*width*/ 80, now).0).is_empty());

        let mut canceled = pane();
        canceled.mark_fresh_task_for_sparkle("example-model", &enabled());
        type_text(&mut canceled, "/sta");
        for _ in 0..4 {
            key(&mut canceled, KeyCode::Backspace);
        }
        assert_eq!(
            canceled.composer.sparkle.draft.get(),
            SparkleDraft::Untouched
        );
        canceled.select_sparkle_model("astra", &enabled());
        assert!(!dots(&draw(&canceled.composer, /*width*/ 80, now).0).is_empty());

        for input in ["paste draft", "inserted"] {
            let mut pane = pane();
            pane.mark_fresh_task_for_sparkle("example-model", &enabled());
            if input == "inserted" {
                type_text(&mut pane, "/mention");
                assert!(matches!(
                    key(&mut pane, KeyCode::Enter),
                    InputResult::Command(SlashCommand::Mention)
                ));
                pane.composer.insert_str("@");
            } else {
                pane.handle_paste(input.to_string());
            }
            pane.composer
                .set_text_content(String::new(), Vec::new(), Vec::new());
            pane.select_sparkle_model("astra", &enabled());
            assert_eq!(pane.composer.sparkle.draft.get(), SparkleDraft::Dismissed);
            assert!(dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
        }
    });
}


#[test]
fn buffered_commands_and_read_only_inline_arguments_preserve_eligibility() {
    palette(|| {
        let now = Instant::now();
        let mut buffered = pane();
        buffered
            .composer
            .set_disable_paste_burst(/*disabled*/ false);
        buffered.mark_fresh_task_for_sparkle("example-model", &enabled());
        type_text(&mut buffered, "/status");
        buffered
            .composer
            .handle_paste_burst_flush(now + Duration::from_secs(/*secs*/ 1));
        assert!(matches!(
            key(&mut buffered, KeyCode::Enter),
            InputResult::Command(SlashCommand::Status)
        ));
        assert_eq!(
            buffered.composer.sparkle.draft.get(),
            SparkleDraft::Untouched
        );

        let mut inline = pane();
        inline.mark_fresh_task_for_sparkle("example-model", &enabled());
        type_text(&mut inline, "/rename task name");
        assert_eq!(inline.composer.sparkle.draft.get(), SparkleDraft::Command);
        assert!(matches!(
            key(&mut inline, KeyCode::Enter),
            InputResult::CommandWithArgs(SlashCommand::Rename, _, _)
        ));
        inline
            .composer
            .set_text_content(String::new(), Vec::new(), Vec::new());
        inline.select_sparkle_model("astra", &enabled());
        assert!(!dots(&draw(&inline.composer, /*width*/ 80, now).0).is_empty());
    });
}

#[test]
fn commands_hide_a_started_sparkle_without_restarting_its_deadline() {
    palette(|| {
        let now = Instant::now();
        let mut pane = pane();
        pane.mark_fresh_task_for_sparkle("astra", &enabled());
        assert!(!dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
        type_text(&mut pane, "/status");
        assert!(
            dots(
                &draw(
                    &pane.composer,
                    /*width*/ 80,
                    now + Duration::from_secs(/*secs*/ 5)
                )
                .0
            )
            .is_empty()
        );
        assert!(matches!(
            key(&mut pane, KeyCode::Enter),
            InputResult::Command(SlashCommand::Status)
        ));
        assert!(
            !dots(
                &draw(
                    &pane.composer,
                    /*width*/ 80,
                    now + Duration::from_secs(/*secs*/ 6)
                )
                .0
            )
            .is_empty()
        );
        type_text(&mut pane, "/diff");
        assert!(dots(&draw(&pane.composer, /*width*/ 80, now + IDLE_TIMEOUT).0).is_empty());
        assert!(matches!(
            key(&mut pane, KeyCode::Enter),
            InputResult::Command(SlashCommand::Diff)
        ));
        assert!(
            dots(
                &draw(
                    &pane.composer,
                    /*width*/ 80,
                    now + IDLE_TIMEOUT + IDLE_FADE
                )
                .0
            )
            .is_empty()
        );
    });
}

#[test]
fn focus_startup_work_and_configuration_do_not_extend_a_started_deadline() {
    palette(|| {
        let now = Instant::now();
        let mut pane = pane();
        pane.mark_fresh_task_for_sparkle("astra", &enabled());
        pane.composer.set_task_running(/*running*/ true);
        assert!(dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
        pane.composer.set_task_running(/*running*/ false);
        assert!(!dots(&draw(&pane.composer, /*width*/ 80, now + IDLE_TIMEOUT).0).is_empty());
        pane.set_sparkle_terminal_focus(/*focused*/ false);
        assert!(
            dots(
                &draw(
                    &pane.composer,
                    /*width*/ 80,
                    now + IDLE_TIMEOUT + Duration::from_secs(/*secs*/ 1)
                )
                .0
            )
            .is_empty()
        );
        pane.set_sparkle_terminal_focus(/*focused*/ true);
        assert!(dots(&draw(&pane.composer, /*width*/ 80, now + IDLE_TIMEOUT * 2).0).is_empty());

        for settings in [
            Tui {
                whimsy: false,
                ..enabled()
            },
            Tui {
                animations: false,
                ..enabled()
            },
        ] {
            let mut pane = super::tests::pane();
            pane.mark_fresh_task_for_sparkle("astra", &settings);
            assert!(dots(&draw(&pane.composer, /*width*/ 80, now).0).is_empty());
        }
        let mut narrow = super::tests::pane();
        narrow.mark_fresh_task_for_sparkle("astra", &enabled());
        let (buffer, textarea) = draw(&narrow.composer, /*width*/ 25, now);
        assert!(
            dots(&buffer)
                .iter()
                .all(|(x, y, _)| !textarea.contains(Position::new(*x, *y)))
        );
    });
}
