use super::*;
use pretty_assertions::assert_eq;

fn configure_mcp_servers(app: &mut App) {
    let config: genex_config::types::McpServerConfig =
        toml::from_str::<toml::Value>("command = 'true'")
            .expect("test MCP config should parse")
            .try_into()
            .expect("test MCP config should deserialize");
    app.config
        .mcp_servers
        .set(HashMap::from([
            ("eager".to_string(), config.clone()),
            ("deferred".to_string(), config),
        ]))
        .expect("test MCP servers should accept any configuration");
}


#[tokio::test]
async fn resumed_subagent_mcp_startup_settles_while_cached_servers_remain_deferred() {
    let mut app = make_test_app().await;
    configure_mcp_servers(&mut app);
    let subagent_thread_id = ThreadId::new();
    app.primary_thread_id = Some(subagent_thread_id);
    app.active_thread_id = Some(subagent_thread_id);
    app.agent_navigation.mark_parent_owned(subagent_thread_id);
    app.refresh_mcp_startup_expected_servers_from_config();

    for status in [
        McpServerStartupState::Starting,
        McpServerStartupState::Ready,
    ] {
        app.chat_widget.handle_server_notification(
            ServerNotification::McpServerStatusUpdated(McpServerStatusUpdatedNotification {
                thread_id: Some(subagent_thread_id.to_string()),
                name: "eager".to_string(),
                status,
                error: None,
                failure_reason: None,
            }),
            /*replay_kind*/ None,
        );
    }

    assert!(!app.chat_widget.is_task_running_for_test());
}

#[tokio::test]
async fn side_conversations_wait_for_every_configured_mcp_server() {
    let mut app = make_test_app().await;
    configure_mcp_servers(&mut app);
    let root_thread_id = ThreadId::new();
    let side_thread_id = ThreadId::new();
    app.primary_thread_id = Some(root_thread_id);
    app.side_threads
        .insert(side_thread_id, SideThreadState::new(root_thread_id));
    app.active_thread_id = Some(side_thread_id);
    app.refresh_mcp_startup_expected_servers_from_config();

    for status in [
        McpServerStartupState::Starting,
        McpServerStartupState::Ready,
    ] {
        app.chat_widget.handle_server_notification(
            ServerNotification::McpServerStatusUpdated(McpServerStatusUpdatedNotification {
                thread_id: Some(side_thread_id.to_string()),
                name: "eager".to_string(),
                status,
                error: None,
                failure_reason: None,
            }),
            /*replay_kind*/ None,
        );
    }

    assert!(app.chat_widget.is_task_running_for_test());
}
