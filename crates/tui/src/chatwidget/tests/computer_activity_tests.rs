use super::*;

fn computer_item(
    id: &str,
    status: codex_app_server_protocol::McpToolCallStatus,
) -> AppServerThreadItem {
    AppServerThreadItem::McpToolCall {
        id: id.to_string(),
        server: "cua_repl".to_string(),
        tool: "js".to_string(),
        status,
        arguments: json!({"title": format!("Inspect page {id}"), "code": "await cua.getState()"}),
        app_context: None,
        mcp_app_resource_uri: None,
        mcp_app_ui: None,
        plugin_id: None,
        read_only_hint: None,
        result: Some(Box::new(codex_app_server_protocol::McpToolCallResult {
            content: vec![json!({"type": "text", "text": format!("Full output for {id}")})],
            structured_content: None,
            meta: None,
        })),
        error: None,
        duration_ms: Some(5),
    }
}


#[tokio::test]
async fn computer_activity_preserves_boundaries_and_expanded_output() {
    use codex_app_server_protocol::McpToolCallStatus;
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.on_mcp_tool_call_completed(computer_item("1", McpToolCallStatus::Completed));
    chat.prepare_assistant_message();
    let cells = drain_insert_history(&mut rx);
    assert!(lines_to_single_string(&cells[0]).contains("Used computer · 1 action"));
    chat.on_mcp_tool_call_completed(computer_item("2", McpToolCallStatus::Completed));
    let mut other = computer_item("other", McpToolCallStatus::Completed);
    if let AppServerThreadItem::McpToolCall { server, .. } = &mut other {
        *server = "another_server".to_string();
    }
    chat.on_mcp_tool_call_completed(other);
    let cells = drain_insert_history(&mut rx);
    assert_eq!(cells.len(), 2);
    assert!(lines_to_single_string(&cells[0]).contains("Used computer · 1 action"));
    assert!(lines_to_single_string(&cells[1]).contains("another_server"));
    chat.on_mcp_tool_call_completed(computer_item("3", McpToolCallStatus::Completed));
    let transcript = chat.active_cell_transcript_lines(/*width*/ 100).unwrap();
    assert!(lines_to_single_string(&transcript).contains("Full output for 3"));
    assert!(lines_to_single_string(&transcript).contains("await cua.getState()"));
}
