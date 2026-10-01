use super::*;
use genex_protocol::mcp::CallToolResult;
use pretty_assertions::assert_eq;
use serde_json::json;

fn call(id: &str, title: &str) -> McpToolCallCell {
    McpToolCallCell::new(
        id.to_string(),
        McpInvocation {
            server: "cua_repl".to_string(),
            tool: "js".to_string(),
            arguments: Some(json!({"title": title, "code": "await cua.getState()"})),
        },
        /*animations_enabled*/ false,
    )
}

fn result(text: &str) -> Result<CallToolResult, String> {
    Ok(CallToolResult {
        content: vec![json!({"type": "text", "text": text})],
        structured_content: None,
        is_error: Some(false),
        meta: None,
    })
}

fn render(cell: &ComputerActivityCell, width: u16) -> String {
    cell.display_lines(width)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
