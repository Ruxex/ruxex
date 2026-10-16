use super::*;
use pretty_assertions::assert_eq;

#[test]
fn terminal_draw_keeps_intermediate_cursor_positions_hidden() {
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(/*width*/ 12, /*height*/ 2)).expect("terminal");
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 12, /*height*/ 2,
    );
    let cursor = (1, 4);
    terminal.set_viewport_area(area);
    let mut parser =
        vt100::Parser::new(/*rows*/ 2, /*cols*/ 12, /*scrollback_len*/ 0);

    for (index, (marker, show_cursor)) in [
        ("x", true),
        ("y", true),
        ("y", true),
        ("", false),
        ("z", true),
    ]
    .into_iter()
    .enumerate()
    {
        terminal.backend_mut().output.clear();
        terminal
            .draw(|frame| {
                let buffer = frame.buffer_mut();
                buffer.set_string(0, 0, "anchor", Style::default());
                if !marker.is_empty() {
                    buffer.set_string(10, 1, marker, Style::default());
                }
                if show_cursor {
                    frame.set_cursor_position((cursor.1, cursor.0));
                }
            })
            .expect("draw");

        // Process one byte at a time to expose cursor motion even if a terminal displays a
        // synchronized frame before all writes have arrived.
        let output = terminal.backend().output();
        for byte in output.bytes() {
            parser.process(&[byte]);
            if index > 0 && !parser.screen().hide_cursor() {
                assert_eq!(parser.screen().cursor_position(), cursor, "frame {index}");
            }
        }
        assert_eq!(parser.screen().hide_cursor(), !show_cursor, "frame {index}");
        assert_eq!(
            parser.screen().cell(1, 10).expect("marker cell").contents(),
            marker,
            "frame {index}"
        );
    }
}

#[test]
fn terminal_draw_repaints_the_cursor_anchor_only_when_its_style_needs_restoring() {
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(/*width*/ 12, /*height*/ 2)).expect("terminal");
    let area = Rect::new(
        /*x*/ 0, /*y*/ 0, /*width*/ 12, /*height*/ 2,
    );
    terminal.set_viewport_area(area);
    let render = |terminal: &mut Terminal<CaptureBackend>, marker| {
        terminal.backend_mut().output.clear();
        terminal
            .draw(|frame| {
                let buffer = frame.buffer_mut();
                buffer.set_string(0, 0, "transcript", Style::default());
                buffer.set_string(10, 1, marker, Style::default());
                frame.set_cursor_position((4, 1));
            })
            .expect("draw");
        terminal.backend().output()
    };

    let first = render(&mut terminal, "x");
    assert!(first.contains("\x1b[1;1H"));
    let next = render(&mut terminal, "y");
    assert!(next.contains("\x1b[2;11H"));
    assert!(!next.contains("\x1b[1;1H"));

    terminal.invalidate_cursor_state();
    let restored = render(&mut terminal, "y");
    assert!(restored.contains("\x1b[1;1H"));
    let unchanged = render(&mut terminal, "y");
    assert!(!unchanged.contains("\x1b[1;1H"));
}

#[test]
fn terminal_draw_omits_cursor_style_without_an_owned_glyph() {
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(/*width*/ 2, /*height*/ 1)).expect("terminal");
    for width in [0, 2] {
        terminal.set_viewport_area(Rect::new(
            /*x*/ 0, /*y*/ 0, width, /*height*/ 1,
        ));
        for buffer in &mut terminal.buffers {
            for cell in &mut buffer.content {
                cell.diff_option = CellDiffOption::Skip;
            }
        }
        terminal.backend_mut().output.clear();
        terminal
            .draw(|frame| {
                for cell in &mut frame.buffer_mut().content {
                    cell.diff_option = CellDiffOption::Skip;
                }
                frame.set_cursor_style(SetCursorStyle::SteadyBar);
                frame.set_cursor_position((1, 0));
            })
            .expect("draw");
        let output = terminal.backend().output();
        assert!(
            !output.contains("\x1b[6 q"),
            "cursor style should not be emitted without an owned glyph: {output:?}"
        );
        assert!(output.contains("\x1b[1;2H"));
        assert!(output.ends_with("\x1b[?25h"));
    }
    terminal.set_viewport_area(Rect::default());
    terminal.backend_mut().output.clear();
    terminal.draw(|_| {}).expect("hide cursor");
    let output = terminal.backend().output();
    assert!(
        !output.contains("\x1b[6 q"),
        "cursor style should not be emitted when hiding an unowned cursor: {output:?}"
    );
    assert!(output.ends_with("\x1b[?25l"));
}
