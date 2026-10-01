//! Permission selection through the existing app-server settings API.

use super::*;
use pretty_assertions::assert_eq;


#[tokio::test]
async fn matching_builtin_permission_selection_submits_pending_prompt() -> Result<()> {
    let (mut app, mut events, mut ops) = make_test_app_with_channels().await;
    let mut server = start_config_write_test_app_server(&app).await?;
    let started = server.start_thread(&app.config).await?;
    app.enqueue_primary_thread_session(started.session, started.turns)
        .await?;
    app.chat_widget.initial_user_message =
        create_initial_user_message(Some("review this".to_string()), Vec::new(), Vec::new());
    let config = app.chat_widget.config_ref();
    let selection = PermissionProfileSelection {
        profile_id: config.permissions.active_permission_profile().unwrap().id,
        approval_policy: Some(config.permissions.approval_policy.value().into()),
        approvals_reviewer: Some(config.approvals_reviewer),
        display_label: "Current permissions".to_string(),
    };
    while events.try_recv().is_ok() {}
    app.select_permission_profile(&mut server, selection).await;
    assert!(app.chat_widget.initial_user_message.is_none());
    assert!(app.pending_server_profiles.is_empty());
    let Op::UserTurn { items, .. } = next_user_turn_op(&mut ops) else {
        panic!("expected initial user turn");
    };
    assert_eq!(
        items,
        vec![AppServerUserInput::Text {
            text: "review this".to_string(),
            text_elements: Vec::new(),
        }]
    );
    server.shutdown().await?;
    Ok(())
}
