use crate::markdown_render::render_markdown_text_with_width;
use pretty_assertions::assert_eq;

fn markdown_text(source: &str, width: usize) -> String {
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
fn mermaid_fences_use_native_renderer_for_every_family() {
    for source in [
        "%% heading\nflowchart TD; A --> B",
        "graph LR; A --> B",
        "sequenceDiagram; A->>B: request; B-->>A: response",
        "stateDiagram-v2; [*] --> Active; Active --> [*]",
        "stateDiagram; [*] --> Active; Active --> [*]",
        "classDiagram; Order \"1\" *-- \"many\" Item : contains",
        "erDiagram; CUSTOMER ||--o{ ORDER : places",
    ] {
        let markdown = format!("```mermaid title=example\n{source}\n```\n");
        assert_eq!(
            markdown_text(&markdown, /*width*/ 100),
            codex_mermaid::render(source, /*max_width*/ 100).unwrap()
        );
    }
}

#[test]
fn mermaid_unclosed_invalid_unsupported_and_wide_blocks_keep_source() {
    for (source, width) in [
        ("```mermaid\nflowchart LR\nA --> B\n", 80),
        ("````mermaid\nflowchart LR\nA --> B\n```\n", 80),
        ("```mermaid\nflowchart LR\nA[unfinished\n```", 80),
        ("```mermaid\npie\n\"Cats\": 2\n```", 80),
        ("```mermaid\nflowchart LR\nA[Request] --> B[Reply]\n```", 8),
        ("> ```mermaid\n> flowchart LR\n> A --> B\n", 80),
    ] {
        assert_eq!(
            markdown_text(source, width),
            markdown_text(&source.replacen("mermaid", "unknown", /*count*/ 1), width),
            "source: {source:?}",
        );
    }
}
