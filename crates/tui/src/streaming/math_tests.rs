use super::StreamCore;
use super::render_source;
use crate::history_cell::HistoryRenderMode;
use crate::markdown::render_streaming_markdown_agent_with_links_and_cwd;
use pretty_assertions::assert_eq;

#[test]
fn unicode_math_stream_holds_display_until_closed_and_preserves_source() {
    let cwd = std::env::temp_dir();
    for mode in [HistoryRenderMode::Rich, HistoryRenderMode::Raw] {
        for (delimiters, prefix) in [(("$$", "$$"), "Before.\n\n"), (("\\[", "\\]"), "Before.\n")] {
            let mut stream = StreamCore::new(
                Some(60),
                &cwd,
                mode,
                /*inline_visualization_context*/ None,
            );
            let mut emitted = Vec::new();
            let mut source = String::new();
            for chunk in [
                prefix,
                delimiters.0,
                "\n",
                "\\frac{a+b}{c}\n",
                "\n",
                delimiters.1,
                "\n\n",
                "After $\\alpha^2$.\n",
            ] {
                source.push_str(chunk);
                stream.push_delta(chunk);
                emitted.extend(stream.tick_batch(usize::MAX));
                if mode == HistoryRenderMode::Rich && source.ends_with("\\frac{a+b}{c}\n") {
                    assert_eq!(
                        emitted,
                        render_source(
                            prefix,
                            Some(60),
                            &cwd,
                            mode,
                            /*inline_visualization_context*/ None
                        )
                    );
                }
            }
            let (remaining, raw) = stream.finalize_remaining();
            emitted.extend(remaining);
            assert_eq!(raw, source);
            assert_eq!(
                emitted,
                render_source(
                    &source,
                    Some(60),
                    &cwd,
                    mode,
                    /*inline_visualization_context*/ None
                )
            );
        }
    }
}

#[test]
fn unicode_math_rejected_display_keeps_its_closer_and_following_text() {
    let cwd = std::env::temp_dir();
    for (open, close) in [("$$", "$$"), ("\\[", "\\]")] {
        for (prefix, body) in [
            ("", "x".repeat(/*n*/ 5000)),
            ("", "`x`".to_owned()),
            ("", format!("`{close}\nx`")),
            ("", "[x](https://example.com)".to_owned()),
            ("Equation: ", "x^2".to_owned()),
        ] {
            let mut render = super::super::render::StreamingRender::new();
            let mut source = String::new();
            for chunk in [
                format!("{prefix}{open}\n\n"),
                format!("{body}\n\n"),
                format!("{close}\n\n"),
                "After $\\alpha$.\n\n".to_owned(),
                "$$\\beta$$\n".to_owned(),
            ] {
                source.push_str(&chunk);
                render.append(
                    &source,
                    &chunk,
                    Some(40),
                    &cwd,
                    HistoryRenderMode::Rich,
                    /*inline_visualization_context*/ None,
                );
                let expected = render_streaming_markdown_agent_with_links_and_cwd(
                    &source,
                    Some(40),
                    Some(&cwd),
                );
                assert_eq!(
                    (&render.lines, render.pending_math_start),
                    (&expected.lines, expected.pending_math_start)
                );
                if source.len() > 4096 || !prefix.is_empty() {
                    assert_eq!(render.pending_math_start, None);
                }
            }
            assert!(
                render
                    .lines
                    .iter()
                    .any(|line| line.line.to_string().contains("After α."))
            );
            assert!(
                render
                    .lines
                    .iter()
                    .any(|line| line.line.to_string().contains('β'))
            );
            assert_eq!(render.pending_math_start, None);
        }
    }
}
