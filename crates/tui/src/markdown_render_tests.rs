use super::MarkdownStyles;
use pretty_assertions::assert_eq;
use ratatui::style::Modifier;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use std::path::Path;

use crate::markdown_render::render_markdown_lines_with_width_and_cwd;
use crate::markdown_render::render_markdown_text;
use crate::markdown_render::render_markdown_text_with_width;
use crate::markdown_render::render_markdown_text_with_width_and_cwd;

fn render_markdown_text_for_cwd(input: &str, cwd: &Path) -> Text<'static> {
    render_markdown_text_with_width_and_cwd(input, /*width*/ None, Some(cwd))
}

fn plain_lines(text: &Text<'_>) -> Vec<String> {
    text.lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.clone())
                .collect::<String>()
        })
        .collect()
}


#[test]
fn table_url_with_tilde_keeps_complete_hyperlink() {
    let destination = "https://www.cs.tufts.edu/~nr/cs257/archive/olin-shivers/dissertation.pdf";
    let markdown = format!("| URL |\n| --- |\n| {destination} |\n");
    let lines = render_markdown_lines_with_width_and_cwd(
        &markdown,
        /*width*/ Some(32),
        /*cwd*/ None,
    );
    let destinations = lines
        .iter()
        .flat_map(|line| line.hyperlinks.iter())
        .map(|link| link.destination.as_str())
        .collect::<Vec<_>>();

    assert!(!destinations.is_empty());
    assert_eq!(destinations, vec![destination; destinations.len()]);
}

#[test]
fn merged_text_events_preserve_entity_decoding() {
    let source = "https://example.com/a&amp;b~c";
    let destination = "https://example.com/a&b~c";
    let lines = render_markdown_lines_with_width_and_cwd(
        source,
        /*width*/ Some(80),
        /*cwd*/ None,
    );
    let rendered = lines
        .iter()
        .map(|line| {
            let text = line
                .line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();
            let hyperlinks = line
                .hyperlinks
                .iter()
                .map(|link| (link.columns.clone(), link.destination.as_str()))
                .collect::<Vec<_>>();
            (text, hyperlinks)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec![(
            destination.to_string(),
            vec![(0..destination.len(), destination)],
        )],
    );
}

#[test]
fn empty() {
    assert_eq!(render_markdown_text(""), Text::default());
}

#[test]
fn paragraph_single() {
    assert_eq!(
        render_markdown_text("Hello, world!"),
        Text::from("Hello, world!")
    );
}

#[test]
fn paragraph_soft_break() {
    assert_eq!(
        render_markdown_text("Hello\nWorld"),
        Text::from_iter(["Hello", "World"])
    );
}

#[test]
fn paragraph_multiple() {
    assert_eq!(
        render_markdown_text("Paragraph 1\n\nParagraph 2"),
        Text::from_iter(["Paragraph 1", "", "Paragraph 2"])
    );
}

#[test]
fn headings() {
    let md = "# Heading 1\n## Heading 2\n### Heading 3\n#### Heading 4\n##### Heading 5\n###### Heading 6\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["# ".bold().underlined(), "Heading 1".bold().underlined()]),
        Line::default(),
        Line::from_iter(["## ".bold(), "Heading 2".bold()]),
        Line::default(),
        Line::from_iter(["### ".bold().italic(), "Heading 3".bold().italic()]),
        Line::default(),
        Line::from_iter(["#### ".italic(), "Heading 4".italic()]),
        Line::default(),
        Line::from_iter(["##### ".italic(), "Heading 5".italic()]),
        Line::default(),
        Line::from_iter(["###### ".italic(), "Heading 6".italic()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn blockquote_single() {
    let text = render_markdown_text("> Blockquote");
    let expected = Text::from(Line::from_iter(["> ", "Blockquote"]).green());
    assert_eq!(text, expected);
}

#[test]
fn blockquote_soft_break() {
    // Soft break via lazy continuation should render as a new line in blockquotes.
    let text = render_markdown_text("> This is a blockquote\nwith a soft break\n");
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "> This is a blockquote".to_string(),
            "> with a soft break".to_string()
        ]
    );
}

#[test]
fn blockquote_multiple_with_break() {
    let text = render_markdown_text("> Blockquote 1\n\n> Blockquote 2\n");
    let expected = Text::from_iter([
        Line::from_iter(["> ", "Blockquote 1"]).green(),
        Line::default(),
        Line::from_iter(["> ", "Blockquote 2"]).green(),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn blockquote_three_paragraphs_short_lines() {
    let md = "> one\n>\n> two\n>\n> three\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["> ", "one"]).green(),
        Line::from_iter(["> "]).green(),
        Line::from_iter(["> ", "two"]).green(),
        Line::from_iter(["> "]).green(),
        Line::from_iter(["> ", "three"]).green(),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn blockquote_nested_two_levels() {
    let md = "> Level 1\n>> Level 2\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["> ", "Level 1"]).green(),
        Line::from_iter(["> "]).green(),
        Line::from_iter(["> ", "> ", "Level 2"]).green(),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn blockquote_with_list_items() {
    let md = "> - item 1\n> - item 2\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["> ", "- ", "item 1"]).green(),
        Line::from_iter(["> ", "- ", "item 2"]).green(),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn blockquote_with_ordered_list() {
    let md = "> 1. first\n> 2. second\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(vec![
            Span::from("> "),
            "1. ".light_blue(),
            Span::from("first"),
        ])
        .green(),
        Line::from_iter(vec![
            Span::from("> "),
            "2. ".light_blue(),
            Span::from("second"),
        ])
        .green(),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn blockquote_list_then_nested_blockquote() {
    let md = "> - parent\n>   > child\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["> ", "- ", "parent"]).green(),
        Line::from_iter(["> ", "  ", "> ", "child"]).green(),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn list_item_with_inline_blockquote_on_same_line() {
    let md = "1. > quoted\n";
    let text = render_markdown_text(md);
    let mut lines = text.lines.iter();
    let first = lines.next().expect("one line");
    // Expect content to include the ordered marker, a space, "> ", and the text
    let s: String = first.spans.iter().map(|sp| sp.content.clone()).collect();
    assert_eq!(s, "1. > quoted");
}

#[test]
fn blockquote_surrounded_by_blank_lines() {
    let md = "foo\n\n> bar\n\nbaz\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "foo".to_string(),
            "".to_string(),
            "> bar".to_string(),
            "".to_string(),
            "baz".to_string(),
        ]
    );
}

#[test]
fn blockquote_in_ordered_list_on_next_line() {
    // Blockquote begins on a new line within an ordered list item; it should
    // render inline on the same marker line.
    let md = "1.\n   > quoted\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["1. > quoted".to_string()]);
}

#[test]
fn blockquote_in_unordered_list_on_next_line() {
    // Blockquote begins on a new line within an unordered list item; it should
    // render inline on the same marker line.
    let md = "-\n  > quoted\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["- > quoted".to_string()]);
}

#[test]
fn blockquote_two_paragraphs_inside_ordered_list_has_blank_line() {
    // Two blockquote paragraphs inside a list item should be separated by a blank line.
    let md = "1.\n   > para 1\n   >\n   > para 2\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "1. > para 1".to_string(),
            "   > ".to_string(),
            "   > para 2".to_string(),
        ],
        "expected blockquote content to stay aligned after list marker"
    );
}

#[test]
fn blockquote_inside_nested_list() {
    let md = "1. A\n    - B\n      > inner\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["1. A", "    - B", "      > inner"]);
}

#[test]
fn list_item_text_then_blockquote() {
    let md = "1. before\n   > quoted\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["1. before", "   > quoted"]);
}

#[test]
fn list_item_blockquote_then_text() {
    let md = "1.\n   > quoted\n   after\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["1. > quoted", "   > after"]);
}

#[test]
fn list_item_text_blockquote_text() {
    let md = "1. before\n   > quoted\n   after\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["1. before", "   > quoted", "   > after"]);
}

#[test]
fn blockquote_with_heading_and_paragraph() {
    let md = "> # Heading\n> paragraph text\n";
    let text = render_markdown_text(md);
    // Validate on content shape; styling is handled elsewhere
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "> # Heading".to_string(),
            "> ".to_string(),
            "> paragraph text".to_string(),
        ]
    );
}

#[test]
fn blockquote_heading_inherits_heading_style() {
    let text = render_markdown_text("> # test header\n> in blockquote\n");
    assert_eq!(
        text.lines,
        [
            Line::from_iter([
                "> ".into(),
                "# ".bold().underlined(),
                "test header".bold().underlined(),
            ])
            .green(),
            Line::from_iter(["> "]).green(),
            Line::from_iter(["> ", "in blockquote"]).green(),
        ]
    );
}

#[test]
fn blockquote_with_code_block() {
    let md = "> ```\n> code\n> ```\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["> code".to_string()]);
}

#[test]
fn blockquote_with_multiline_code_block() {
    let md = "> ```\n> first\n> second\n> ```\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["> first", "> second"]);
}

#[test]
fn nested_blockquote_with_inline_and_fenced_code() {
    /*
    let md = \"> Nested quote with code:\n\
    > > Inner quote and `inline code`\n\
    > >\n\
    > > ```\n\
    > > # fenced code inside a quote\n\
    > > echo \"hello from a quote\"\n\
    > > ```\n";
    */
    let md = r#"> Nested quote with code:
> > Inner quote and `inline code`
> >
> > ```
> > # fenced code inside a quote
> > echo "hello from a quote"
> > ```
"#;
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "> Nested quote with code:".to_string(),
            "> ".to_string(),
            "> > Inner quote and inline code".to_string(),
            "> > ".to_string(),
            "> > # fenced code inside a quote".to_string(),
            "> > echo \"hello from a quote\"".to_string(),
        ]
    );
}

#[test]
fn list_unordered_single() {
    let text = render_markdown_text("- List item 1\n");
    let expected = Text::from_iter([Line::from_iter(["- ", "List item 1"])]);
    assert_eq!(text, expected);
}

#[test]
fn list_unordered_multiple() {
    let text = render_markdown_text("- List item 1\n- List item 2\n");
    let expected = Text::from_iter([
        Line::from_iter(["- ", "List item 1"]),
        Line::from_iter(["- ", "List item 2"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn list_ordered() {
    let text = render_markdown_text("1. List item 1\n2. List item 2\n");
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "List item 1".into()]),
        Line::from_iter(["2. ".light_blue(), "List item 2".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn list_nested() {
    let text = render_markdown_text("- List item 1\n  - Nested list item 1\n");
    let expected = Text::from_iter([
        Line::from_iter(["- ", "List item 1"]),
        Line::from_iter(["    - ", "Nested list item 1"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn list_ordered_custom_start() {
    let text = render_markdown_text("3. First\n4. Second\n");
    let expected = Text::from_iter([
        Line::from_iter(["3. ".light_blue(), "First".into()]),
        Line::from_iter(["4. ".light_blue(), "Second".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn nested_unordered_in_ordered() {
    let md = "1. Outer\n    - Inner A\n    - Inner B\n2. Next\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "Outer".into()]),
        Line::from_iter(["    - ", "Inner A"]),
        Line::from_iter(["    - ", "Inner B"]),
        Line::default(),
        Line::from_iter(["2. ".light_blue(), "Next".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn nested_ordered_in_unordered() {
    let md = "- Outer\n    1. One\n    2. Two\n- Last\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["- ", "Outer"]),
        Line::from_iter(["    1. ".light_blue(), "One".into()]),
        Line::from_iter(["    2. ".light_blue(), "Two".into()]),
        Line::default(),
        Line::from_iter(["- ", "Last"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn loose_list_item_multiple_paragraphs() {
    let md = "1. First paragraph\n\n   Second paragraph of same item\n\n2. Next item\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "First paragraph".into()]),
        Line::default(),
        Line::from_iter(["   ", "Second paragraph of same item"]),
        Line::default(),
        Line::from_iter(["2. ".light_blue(), "Next item".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn tight_item_with_soft_break() {
    let md = "- item line1\n  item line2\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["- ", "item line1"]),
        Line::from_iter(["  ", "item line2"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn deeply_nested_mixed_three_levels() {
    let md = "1. A\n    - B\n        1. C\n2. D\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "A".into()]),
        Line::from_iter(["    - ", "B"]),
        Line::from_iter(["        1. ".light_blue(), "C".into()]),
        Line::default(),
        Line::from_iter(["2. ".light_blue(), "D".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn loose_items_due_to_blank_line_between_items() {
    let md = "1. First\n\n2. Second\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "First".into()]),
        Line::from_iter(["2. ".light_blue(), "Second".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn mixed_tight_then_loose_in_one_list() {
    let md = "1. Tight\n\n2.\n   Loose\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "Tight".into()]),
        Line::from_iter(["2. ".light_blue(), "Loose".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn ordered_item_with_indented_continuation_is_tight() {
    let md = "1. Foo\n   Bar\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "Foo".into()]),
        Line::from_iter(["   ", "Bar"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn inline_code() {
    let text = render_markdown_text("Example of `Inline code`");
    let expected = Line::from_iter([
        "Example of ".into(),
        Span::styled("Inline code", MarkdownStyles::default().code),
    ])
    .into();
    assert_eq!(text, expected);
}


#[test]
fn strong() {
    assert_eq!(
        render_markdown_text("**Strong**"),
        Text::from(Line::from("Strong".bold()))
    );
}

#[test]
fn emphasis() {
    assert_eq!(
        render_markdown_text("*Emphasis*"),
        Text::from(Line::from("Emphasis".italic()))
    );
}

#[test]
fn strikethrough() {
    assert_eq!(
        render_markdown_text("~~Strikethrough~~"),
        Text::from(Line::from("Strikethrough".crossed_out()))
    );
}

#[test]
fn strong_emphasis() {
    let text = render_markdown_text("**Strong *emphasis***");
    let expected = Text::from(Line::from_iter([
        "Strong ".bold(),
        "emphasis".bold().italic(),
    ]));
    assert_eq!(text, expected);
}

#[test]
fn link() {
    let text = render_markdown_text("[Link](https://example.com)");
    let expected = Text::from(Line::from_iter([
        "Link".cyan().underlined(),
        " (".into(),
        "https://example.com".cyan().underlined(),
        ")".into(),
    ]));
    assert_eq!(text, expected);
}

#[test]
fn web_link_labels_use_link_style_and_preserve_inline_formatting() {
    for (label, expected_label) in [
        ("plain", "plain".cyan().underlined()),
        ("`code`", "code".cyan().underlined()),
        ("**bold**", "bold".cyan().bold().underlined()),
        ("*italic*", "italic".cyan().italic().underlined()),
    ] {
        let text = render_markdown_text(&format!(
            "before [{label}](https://example.com) after `code`"
        ));
        let expected = Text::from(Line::from_iter([
            "before ".into(),
            expected_label,
            " (".into(),
            "https://example.com".cyan().underlined(),
            ")".into(),
            " after ".into(),
            Span::styled("code", MarkdownStyles::default().code),
        ]));
        assert_eq!(text, expected, "label: {label}");
    }
}

#[test]
fn web_link_labels_keep_link_style_in_wrapped_prose_and_tables() {
    let links =
        "[plain](https://example.com) [`code`](https://example.com) [<b>](https://example.com)";
    for markdown in [
        links.to_string(),
        format!("| Links |\n| --- |\n| {links} |"),
    ] {
        for width in [32, 80] {
            let text = render_markdown_text_with_width(&markdown, Some(width));
            let labels = text
                .lines
                .iter()
                .flat_map(|line| &line.spans)
                .filter(|span| matches!(span.content.as_ref(), "plain" | "code" | "<b>"))
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(
                labels,
                vec![
                    "plain".cyan().underlined(),
                    "code".cyan().underlined(),
                    "<b>".cyan().underlined()
                ]
            );
        }
    }
}


#[test]
fn file_link_hides_destination() {
    let text = render_markdown_text_for_cwd(
        "[/Users/example/code/ruxex/ruxex-rs/tui/src/My%20File.rs](/Users/example/code/ruxex/ruxex-rs/tui/src/My%20File.rs)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/My File.rs",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}




#[test]
fn file_link_decodes_percent_encoded_bare_path_destination() {
    let text = render_markdown_text_for_cwd(
        "[open Example Folder/Résumé/report.md](/Users/example/code/ruxex/Example%20Folder/R%C3%A9sum%C3%A9/report.md)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([
        "open Example Folder/Résumé/report.md".into(),
        " (".into(),
        Span::styled(
            "Example Folder/Résumé/report.md",
            MarkdownStyles::default().code,
        ),
        ")".into(),
    ]));
    assert_eq!(text, expected);
}




#[test]
fn file_link_appends_line_number_when_label_lacks_it() {
    let text = render_markdown_text_for_cwd(
        "[markdown_render.rs](/Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs:74)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/markdown_render.rs:74",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}

#[test]
fn file_link_keeps_absolute_paths_outside_cwd() {
    let text = render_markdown_text_for_cwd(
        "[README.md:74](/Users/example/code/ruxex/README.md:74)",
        Path::new("/Users/example/code/ruxex/ruxex-rs/tui"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "/Users/example/code/ruxex/README.md:74",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}


#[test]
fn file_link_appends_hash_anchor_when_label_lacks_it() {
    let text = render_markdown_text_for_cwd(
        "[markdown_render.rs](file:///Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs#L74C3)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/markdown_render.rs:74:3",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}

#[test]
fn file_link_uses_target_path_for_hash_anchor() {
    let text = render_markdown_text_for_cwd(
        "[markdown_render.rs#L74C3](file:///Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs#L74C3)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/markdown_render.rs:74:3",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}

#[test]
fn file_link_appends_range_when_label_lacks_it() {
    let text = render_markdown_text_for_cwd(
        "[markdown_render.rs](/Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs:74:3-76:9)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/markdown_render.rs:74:3-76:9",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}

#[test]
fn file_link_uses_target_path_for_range() {
    let text = render_markdown_text_for_cwd(
        "[markdown_render.rs:74:3-76:9](/Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs:74:3-76:9)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/markdown_render.rs:74:3-76:9",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}

#[test]
fn file_link_appends_hash_range_when_label_lacks_it() {
    let text = render_markdown_text_for_cwd(
        "[markdown_render.rs](file:///Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs#L74C3-L76C9)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/markdown_render.rs:74:3-76:9",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}

#[test]
fn multiline_file_link_label_after_styled_prefix_does_not_panic() {
    let text = render_markdown_text_for_cwd(
        "**bold** plain [foo\nbar](file:///Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs#L74C3)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([
        "bold".bold(),
        " plain ".into(),
        "foo".into(),
        " ".into(),
        "bar".into(),
        " (".into(),
        Span::styled(
            "ruxex-rs/tui/src/markdown_render.rs:74:3",
            MarkdownStyles::default().code,
        ),
        ")".into(),
    ]));
    assert_eq!(text, expected);
}

#[test]
fn file_link_uses_target_path_for_hash_range() {
    let text = render_markdown_text_for_cwd(
        "[markdown_render.rs#L74C3-L76C9](file:///Users/example/code/ruxex/ruxex-rs/tui/src/markdown_render.rs#L74C3-L76C9)",
        Path::new("/Users/example/code/ruxex"),
    );
    let expected = Text::from(Line::from_iter([Span::styled(
        "ruxex-rs/tui/src/markdown_render.rs:74:3-76:9",
        MarkdownStyles::default().code,
    )]));
    assert_eq!(text, expected);
}

#[test]
fn url_link_shows_destination() {
    let text = render_markdown_text("[docs](https://example.com/docs)");
    let expected = Text::from(Line::from_iter([
        "docs".cyan().underlined(),
        " (".into(),
        "https://example.com/docs".cyan().underlined(),
        ")".into(),
    ]));
    assert_eq!(text, expected);
}


#[test]
fn unordered_list_local_file_link_stays_inline_with_following_text() {
    let text = render_markdown_text_with_width_and_cwd(
        "- [binary](/Users/example/code/ruxex/ruxex-rs/README.md:93): core is the agent/business logic, tui is the terminal UI, exec is the headless automation surface, and cli is the top-level multitool binary.",
        Some(72),
        Some(Path::new("/Users/example/code/ruxex")),
    );
    let rendered = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec![
            "- binary (ruxex-rs/README.md:93): core is the agent/business logic, tui",
            "  is the terminal UI, exec is the headless automation surface, and cli",
            "  is the top-level multitool binary.",
        ]
    );
}

#[test]
fn unordered_list_local_file_link_soft_break_before_colon_stays_inline() {
    let text = render_markdown_text_with_width_and_cwd(
        "- [binary](/Users/example/code/ruxex/ruxex-rs/README.md:93)\n  : core is the agent/business logic.",
        Some(72),
        Some(Path::new("/Users/example/code/ruxex")),
    );
    let rendered = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec!["- binary (ruxex-rs/README.md:93): core is the agent/business logic.",]
    );
}

#[test]
fn consecutive_unordered_list_local_file_links_do_not_detach_paths() {
    let text = render_markdown_text_with_width_and_cwd(
        "- [binary](/Users/example/code/ruxex/ruxex-rs/README.md:93)\n  : cli is the top-level multitool binary.\n- [expectations](/Users/example/code/ruxex/ruxex-rs/core/README.md:1)\n  : ruxex-core owns the real runtime behavior.",
        Some(72),
        Some(Path::new("/Users/example/code/ruxex")),
    );
    let rendered = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec![
            "- binary (ruxex-rs/README.md:93): cli is the top-level multitool binary.",
            "- expectations (ruxex-rs/core/README.md:1): ruxex-core owns the real",
            "  runtime behavior.",
        ]
    );
}

#[test]
fn code_block_known_lang_has_syntax_colors() {
    let text = render_markdown_text("```rust\nfn main() {}\n```\n");
    let content: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    // Content should be preserved; ignore trailing empty line from highlighting.
    let content: Vec<&str> = content
        .iter()
        .map(std::string::String::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(content, vec!["fn main() {}"]);

    // At least one span should have non-default style (syntax highlighting).
    let has_colored_span = text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .any(|sp| sp.style.fg.is_some());
    assert!(
        has_colored_span,
        "expected syntax-highlighted spans with color"
    );
}

#[test]
fn code_block_unknown_lang_plain() {
    let text = render_markdown_text("```xyzlang\nhello world\n```\n");
    let content: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    let content: Vec<&str> = content
        .iter()
        .map(std::string::String::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(content, vec!["hello world"]);

    // No syntax coloring for unknown language — all spans have default style.
    let has_colored_span = text
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .any(|sp| sp.style.fg.is_some());
    assert!(
        !has_colored_span,
        "expected no syntax coloring for unknown lang"
    );
}

#[test]
fn code_block_no_lang_plain() {
    let text = render_markdown_text("```\nno lang specified\n```\n");
    let content: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    let content: Vec<&str> = content
        .iter()
        .map(std::string::String::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(content, vec!["no lang specified"]);
}

#[test]
fn code_block_multiple_lines_root() {
    let md = "```\nfirst\nsecond\n```\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["", "first"]),
        Line::from_iter(["", "second"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn code_block_indented() {
    let md = "    function greet() {\n      console.log(\"Hi\");\n    }\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["    ", "function greet() {"]),
        Line::from_iter(["    ", "  console.log(\"Hi\");"]),
        Line::from_iter(["    ", "}"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn horizontal_rule_renders_em_dashes() {
    let md = "Before\n\n---\n\nAfter\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["Before", "", "———", "", "After"]);
}

#[test]
fn code_block_with_inner_triple_backticks_outer_four() {
    let md = r#"````text
Here is a code block that shows another fenced block:

```md
# Inside fence
- bullet
- `inline code`
```
````
"#;
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    // Filter empty trailing lines for stability; the code block may or may
    // not emit a trailing blank depending on the highlighting path.
    let trimmed: Vec<&str> = {
        let mut v: Vec<&str> = lines.iter().map(std::string::String::as_str).collect();
        while v.last() == Some(&"") {
            v.pop();
        }
        v
    };
    assert_eq!(
        trimmed,
        vec![
            "Here is a code block that shows another fenced block:",
            "",
            "```md",
            "# Inside fence",
            "- bullet",
            "- `inline code`",
            "```",
        ]
    );
}

#[test]
fn code_block_inside_unordered_list_item_is_indented() {
    let md = "- Item\n\n  ```\n  code line\n  ```\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["- Item", "", "  code line"]);
}

#[test]
fn code_block_multiple_lines_inside_unordered_list() {
    let md = "- Item\n\n  ```\n  first\n  second\n  ```\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["- Item", "", "  first", "  second"]);
}

#[test]
fn code_block_inside_unordered_list_item_multiple_lines() {
    let md = "- Item\n\n  ```\n  first\n  second\n  ```\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(lines, vec!["- Item", "", "  first", "  second"]);
}


#[test]
fn outer_list_item_after_nested_code_block_keeps_blank_separator() {
    let md =
        "1. First:\n   - Nested:\n\n     ```rust\n     fn first() {}\n     ```\n\n2. Second:\n";
    let text = render_markdown_text(md);
    let lines = plain_lines(&text);
    assert_eq!(
        lines,
        vec![
            "1. First:",
            "    - Nested:",
            "",
            "      fn first() {}",
            "",
            "2. Second:",
        ]
    );
}

#[test]
fn list_item_after_simple_item_stays_compact() {
    let md = "1. First\n\n2. Second\n";
    let text = render_markdown_text(md);
    assert_eq!(plain_lines(&text), vec!["1. First", "2. Second"]);
}


#[test]
fn wrapped_list_item_is_separated_from_next_sibling() {
    let md = "1. This item wraps onto another visible rendered line\n2. Next item\n";
    let text = render_markdown_text_with_width(md, Some(/*width*/ 24));
    assert_eq!(
        plain_lines(&text),
        vec![
            "1. This item wraps onto",
            "   another visible",
            "   rendered line",
            "",
            "2. Next item",
        ]
    );
}



#[test]
fn ordered_item_with_code_block_and_nested_bullet() {
    let md = "1. **item 1**\n\n2. **item 2**\n   ```\n   code\n   ```\n   - `PROCESS_START` (a `OnceLock<Instant>`) keeps the start time for the entire process.\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "1. item 1".to_string(),
            "2. item 2".to_string(),
            String::new(),
            "   code".to_string(),
            "    - PROCESS_START (a OnceLock<Instant>) keeps the start time for the entire process.".to_string(),
        ]
    );
}

#[test]
fn nested_five_levels_mixed_lists() {
    let md = "1. First\n   - Second level\n     1. Third level (ordered)\n        - Fourth level (bullet)\n          - Fifth level to test indent consistency\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "First".into()]),
        Line::from_iter(["    - ", "Second level"]),
        Line::from_iter(["        1. ".light_blue(), "Third level (ordered)".into()]),
        Line::from_iter(["            - ", "Fourth level (bullet)"]),
        Line::from_iter([
            "                - ",
            "Fifth level to test indent consistency",
        ]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn html_inline_is_verbatim() {
    let md = "Hello <span>world</span>!";
    let text = render_markdown_text(md);
    let expected: Text = Line::from_iter(["Hello ", "<span>", "world", "</span>", "!"]).into();
    assert_eq!(text, expected);
}

#[test]
fn html_block_is_verbatim_multiline() {
    let md = "<div>\n  <span>hi</span>\n</div>\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["<div>"]),
        Line::from_iter(["  <span>hi</span>"]),
        Line::from_iter(["</div>"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn html_in_tight_ordered_item_soft_breaks_with_space() {
    let md = "1. Foo\n   <i>Bar</i>\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "Foo".into()]),
        Line::from_iter(["   ", "<i>", "Bar", "</i>"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn html_continuation_paragraph_in_unordered_item_indented() {
    let md = "- Item\n\n  <em>continued</em>\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["- ", "Item"]),
        Line::default(),
        Line::from_iter(["  ", "<em>", "continued", "</em>"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn unordered_item_continuation_paragraph_is_indented() {
    let md = "- Intro\n\n  Continuation paragraph line 1\n  Continuation paragraph line 2\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "- Intro".to_string(),
            String::new(),
            "  Continuation paragraph line 1".to_string(),
            "  Continuation paragraph line 2".to_string(),
        ]
    );
}

#[test]
fn ordered_item_continuation_paragraph_is_indented() {
    let md = "1. Intro\n\n   More details about intro\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "Intro".into()]),
        Line::default(),
        Line::from_iter(["   ", "More details about intro"]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn nested_item_continuation_paragraph_is_indented() {
    let md = "1. A\n    - B\n\n      Continuation for B\n2. C\n";
    let text = render_markdown_text(md);
    let expected = Text::from_iter([
        Line::from_iter(["1. ".light_blue(), "A".into()]),
        Line::from_iter(["    - ", "B"]),
        Line::default(),
        Line::from_iter(["      ", "Continuation for B"]),
        Line::default(),
        Line::from_iter(["2. ".light_blue(), "C".into()]),
    ]);
    assert_eq!(text, expected);
}

#[test]
fn code_block_preserves_trailing_blank_lines() {
    // A fenced code block with an intentional trailing blank line must keep it.
    let md = "```rust\nfn main() {}\n\n```\n";
    let text = render_markdown_text(md);
    let content: Vec<String> = text
        .lines
        .iter()
        .map(|l| {
            l.spans
                .iter()
                .map(|s| s.content.clone())
                .collect::<String>()
        })
        .collect();
    // Should have: "fn main() {}" then "" (the blank line).
    // Filter only to content lines (skip leading/trailing empty from rendering).
    assert!(
        content.iter().any(|c| c == "fn main() {}"),
        "expected code line, got {content:?}"
    );
    // The trailing blank line inside the fence should be preserved.
    let code_start = content.iter().position(|c| c == "fn main() {}").unwrap();
    assert!(
        content.len() > code_start + 1,
        "expected a line after 'fn main() {{}}' but content ends: {content:?}"
    );
    assert_eq!(
        content[code_start + 1],
        "",
        "trailing blank line inside code fence was lost: {content:?}"
    );
}

#[test]
fn table_renders_app_style_rows_with_themed_bold_header() {
    let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.clone()).collect())
        .collect();

    assert_eq!(
        lines,
        vec![
            " A      B".to_string(),
            "━━━━━  ━━━━━".to_string(),
            " 1      2".to_string(),
        ]
    );
    assert!(text.lines[0].style.add_modifier.contains(Modifier::BOLD));
    assert!(
        text.lines[0].style.fg.is_some(),
        "expected the syntax theme to provide a table header accent"
    );
    assert!(
        text.lines[1].spans[0]
            .style
            .add_modifier
            .contains(Modifier::DIM)
    );
    assert!(!text.lines[2].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn table_alignment_respects_markers() {
    let md = "| Left | Center | Right |\n|:-----|:------:|------:|\n| a | b | c |\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.clone()).collect())
        .collect();

    assert_eq!(lines[0], " Left    Center    Right");
    assert_eq!(lines[2], " a         b           c");
}

#[test]
fn table_separates_logical_rows_after_wrapped_content() {
    let md = "| Key | Description |\n| --- | --- |\n| -v | Enable very verbose logging output for debugging |\n| -q | Quiet output |\n";
    let text = crate::markdown_render::render_markdown_text_with_width(md, Some(30));
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.clone()).collect())
        .collect();

    assert!(
        lines
            .iter()
            .any(|line| line.contains("Enable very verbose"))
            && lines.iter().any(|line| line.contains("logging output")),
        "expected wrapped row content: {lines:?}"
    );
    let separator_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            ((line.contains('━') || line.contains('─'))
                && line.chars().all(|ch| matches!(ch, '━' | '─' | ' ')))
            .then_some(idx)
        })
        .collect();
    let wrapped_row_end = lines
        .iter()
        .position(|line| line.contains("logging output"))
        .expect("expected final wrapped line");
    assert_eq!(separator_indices.len(), 2);
    assert!(separator_indices[1] > wrapped_row_end);
    assert!(
        !lines
            .last()
            .is_some_and(|line| line.contains('━') || line.contains('─'))
    );
}







#[test]
fn table_inside_blockquote_has_quote_prefix() {
    let md = "> | A | B |\n> |---|---|\n> | 1 | 2 |\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.clone()).collect())
        .collect();

    assert!(lines.iter().all(|line| line.starts_with("> ")));
    assert!(lines.iter().any(|line| line.contains("━━━━━  ━━━━━")));
}

#[test]
fn escaped_pipes_render_in_table_cells() {
    let md = "| Col |\n| --- |\n| a \\| b |\n";
    let text = render_markdown_text(md);
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.clone()).collect())
        .collect();

    assert!(lines.iter().any(|line| line.contains("a | b")));
}

#[test]
fn table_falls_back_to_key_value_records_if_grid_cannot_fit() {
    let md = "| c1 | c2 | c3 | c4 | c5 | c6 | c7 | c8 | c9 | c10 |\n|---|---|---|---|---|---|---|---|---|---|\n| 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |\n";
    let text = crate::markdown_render::render_markdown_text_with_width(md, Some(/*width*/ 20));
    let lines: Vec<String> = text
        .lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.clone()).collect())
        .collect();

    assert!(lines.first().is_some_and(|line| line.contains("c1")));
    assert!(
        lines
            .iter()
            .any(|line| line.contains("c10") && line.contains("10"))
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with('|') || line.contains('━') || line.contains('─'))
    );
}

#[test]
fn table_key_value_fallback_preserves_rich_values_and_themed_labels() {
    let md = "| Key | Content | Extra | More |\n|---|---|---|---|\n| item | [link](https://example.com) | **bold** | `code` |\n";
    let text = crate::markdown_render::render_markdown_text_with_width(md, Some(/*width*/ 16));
    let lines = plain_lines(&text);

    assert!(lines.iter().any(|line| line.contains("Key")));
    assert!(lines.iter().any(|line| line.contains("item")));
    assert!(lines.iter().any(|line| line.contains("link")));
    assert!(lines.iter().any(|line| line.contains("bold")));
    assert!(lines.iter().any(|line| line.contains("code")));
    assert!(
        text.lines[0]
            .spans
            .iter()
            .any(|span| span.content.contains("Key")
                && span.style.add_modifier.contains(Modifier::BOLD)
                && span.style.fg.is_some())
    );
    assert!(text.lines.iter().any(|line| {
        line.spans
            .iter()
            .any(|span| span.style.add_modifier.contains(Modifier::UNDERLINED))
    }));
}
