use super::MathMarkdown;
use super::render::render;
use crate::markdown_render::render_markdown_text_with_width;
use pretty_assertions::assert_eq;
use pulldown_cmark::Options;

fn plain(source: &str, width: usize) -> String {
    render_markdown_text_with_width(source, Some(width))
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn unicode_math_pending_display_tracks_original_offset() {
    assert_eq!(
        MathMarkdown::new("Prose\n\n$$\nx^2\n\n", Options::empty(), Some(80)).pending_start,
        Some(7)
    );
    assert_eq!(
        MathMarkdown::new("```\n$$\n", Options::empty(), Some(80)).pending_start,
        None
    );
    assert_eq!(
        MathMarkdown::new("echo $$\n", Options::empty(), Some(80)).pending_start,
        None
    );
}

#[test]
fn unicode_math_nested_fractions_and_oversized_pending_stay_source() {
    for source in [r"\frac{\frac{a}{b}}{c}", r"\frac{a}{\frac{b}{c}}"] {
        assert_eq!(render(source, /*display*/ true), None);
    }
    let source = format!("$$\n{}", "x".repeat(/*n*/ 5000));
    assert_eq!(
        MathMarkdown::new(&source, Options::empty(), Some(80)).pending_start,
        None
    );
}

#[test]
fn unicode_math_oversized_display_stays_literal() {
    for (open, close) in [("$$", "$$"), ("\\[", "\\]")] {
        for ending in ["", close] {
            let source = format!("{open}\n{}{ending}", "# x\n- y\n".repeat(/*n*/ 600));
            assert_eq!(plain(&source, /*width*/ 80), source.trim_end());
        }
    }
}

#[test]
fn unicode_math_pending_source_and_pid_boundaries() {
    for (open, close) in [("$$", "$$"), ("\\[", "\\]")] {
        assert_eq!(
            plain(&format!("{open}\nx^2\n\n+y^2\n{close}"), /*width*/ 80),
            "x² +y²"
        );
        let source = format!("{open}\nx\n{close} trailing\n{close}\n\nAfter $\\alpha$.");
        assert_eq!(
            plain(&source, /*width*/ 80),
            format!("{open}\nx\n{close} trailing\n{close}\n\nAfter α.")
        );
    }
    for source in ["$$\n# x\n", "$$\n- x\n", "\\[\n# x\n", "\\[\n- x\n"] {
        assert_eq!(plain(source, /*width*/ 80), source.trim_end());
    }
    let source = format!("{}After $\\alpha$.", "prose \\[\n".repeat(/*n*/ 1000));
    let math = MathMarkdown::new(&source, Options::empty(), Some(80));
    assert_eq!(math.display_ranges.len(), 1);
    assert!(math.replacements.is_empty());
    assert_eq!(
        plain(
            "echo $$\n\n$$\nx^2\n$$\n\nAfter $\\alpha$.",
            /*width*/ 80
        ),
        "echo $$\n\nx²\n\nAfter α."
    );
    assert_eq!(
        plain(
            "$$\nprice=\\$$$\n\nAfter $\\alpha$.\n\n$$\\beta$$",
            /*width*/ 80
        ),
        "$$\nprice=\\$$$\n\nAfter α.\n\nβ"
    );
}
