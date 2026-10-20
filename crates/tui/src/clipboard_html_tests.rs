use super::render_markdown;
use pretty_assertions::assert_eq;

#[test]
fn markdown_clipboard_images_are_inert() {
    assert_eq!(
        render_markdown(
            "![tracking pixel](https://example.com/pixel)\n\n\
             [![**Preview** & details](https://example.com/image.png)](https://example.com/page)"
        ),
        "<p>tracking pixel</p>\n\
         <p><a href=\"https://example.com/page\"><strong>Preview</strong> &amp; details</a></p>\n"
    );
}

#[test]
fn markdown_clipboard_escapes_raw_html_and_rejects_unsafe_destinations() {
    assert_eq!(
        render_markdown("<script>alert(1)</script>\n\nHello <img src=x onerror=alert(1)>!"),
        "&lt;script&gt;alert(1)&lt;/script&gt;\n<p>Hello &lt;img src=x onerror=alert(1)&gt;!</p>\n"
    );
    for (destination, visible_destination) in [
        ("javascript:alert(1)", "javascript:alert(1)"),
        ("JaVaScRiPt:alert(1)", "JaVaScRiPt:alert(1)"),
        ("jav&#x61;script:alert(1)", "javascript:alert(1)"),
        ("data:text/html,unsafe", "data:text/html,unsafe"),
        (
            "javascript:alert(&quot;&lt;&gt;&amp;&quot;)",
            "javascript:alert(\"&lt;&gt;&amp;\")",
        ),
    ] {
        assert_eq!(
            render_markdown(&format!("[link]({destination}) ![image]({destination})")),
            format!("<p>link ({visible_destination}) image</p>\n")
        );
    }
}

#[test]
fn markdown_clipboard_unwraps_table_fences_like_the_tui() {
    let table = "| Item | Result |\n| --- | --- |\n| Copy | Formatted |\n";
    assert_eq!(
        render_markdown(&format!("```markdown\n{table}```")),
        render_markdown(table)
    );
}
