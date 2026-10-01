//! Approval-request focused tests extracted from the main chatwidget test file
//! to keep the primary module under blob-size policy limits.

use super::*;
use pretty_assertions::assert_eq;


#[test]
fn app_server_exec_approval_request_splits_shell_wrapped_command() {
    let script = r#"python3 -c 'print("Hello, world!")'"#;
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            kind: Default::default(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: None,
            command: Some(
                shlex::try_join(["/bin/zsh", "-lc", script])
                    .expect("round-trippable shell wrapper"),
            ),
            cwd: Some(test_path_buf("/tmp").abs().into()),
            command_actions: None,
            additional_permissions: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: None,
        },
        &test_path_buf("/tmp").abs(),
    );

    assert_eq!(
        request.command,
        vec![
            "/bin/zsh".to_string(),
            "-lc".to_string(),
            script.to_string(),
        ]
    );
}


#[test]
fn app_server_exec_approval_request_preserves_permissions_context() {
    let read_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/read-only")))
        .expect("absolute read path");
    let write_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/write")))
        .expect("absolute write path");
    let read_api_path = LegacyAppPathString::from_abs_path(&read_path);
    let write_api_path = LegacyAppPathString::from_abs_path(&write_path);
    let request = exec_approval_request_from_params(
        AppServerCommandExecutionRequestApprovalParams {
            kind: Default::default(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "item-1".to_string(),
            started_at_ms: 0,
            approval_id: Some("approval-1".to_string()),
            environment_id: None,
            reason: None,
            network_approval_context: Some(genex_app_server_protocol::NetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: genex_app_server_protocol::NetworkApprovalProtocol::Socks5Tcp,
            }),
            command: Some("ls".to_string()),
            cwd: Some(test_path_buf("/tmp").abs().into()),
            command_actions: None,
            additional_permissions: Some(AppServerAdditionalPermissionProfile {
                network: Some(AppServerAdditionalNetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(AppServerAdditionalFileSystemPermissions {
                    read: Some(vec![read_api_path.clone()]),
                    write: Some(vec![write_api_path.clone()]),
                    glob_scan_max_depth: None,
                    entries: None,
                }),
            }),
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            available_decisions: None,
        },
        &test_path_buf("/tmp").abs(),
    );

    assert_eq!(
        request.network_approval_context,
        Some(genex_app_server_protocol::NetworkApprovalContext {
            host: "example.com".to_string(),
            protocol: genex_app_server_protocol::NetworkApprovalProtocol::Socks5Tcp,
        })
    );
    assert_eq!(
        request.additional_permissions,
        Some(AppServerAdditionalPermissionProfile {
            network: Some(AppServerAdditionalNetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(AppServerAdditionalFileSystemPermissions {
                read: Some(vec![read_api_path]),
                write: Some(vec![write_api_path]),
                glob_scan_max_depth: None,
                entries: None,
            }),
        })
    );
}




#[test]
fn app_server_request_permissions_preserves_file_system_permissions() {
    let read_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/read-only")))
        .expect("absolute read path");
    let write_path = AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp/write")))
        .expect("absolute write path");
    let read_api_path = LegacyAppPathString::from_abs_path(&read_path);
    let write_api_path = LegacyAppPathString::from_abs_path(&write_path);
    let cwd =
        AbsolutePathBuf::try_from(PathBuf::from(test_path_display("/tmp"))).expect("absolute cwd");

    let request = request_permissions_from_params(AppServerPermissionsRequestApprovalParams {
        thread_id: "thread-1".to_string(),
        turn_id: "turn-1".to_string(),
        item_id: "item-1".to_string(),
        environment_id: Some("remote".to_string()),
        started_at_ms: 0,
        cwd: cwd.clone().into(),
        reason: Some("Select a workspace root".to_string()),
        permissions: genex_app_server_protocol::RequestPermissionProfile {
            network: Some(AppServerAdditionalNetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(AppServerAdditionalFileSystemPermissions {
                read: Some(vec![read_api_path]),
                write: Some(vec![write_api_path]),
                glob_scan_max_depth: None,
                entries: None,
            }),
        },
    })
    .expect("API paths should convert to native paths");

    assert_eq!(
        request.permissions,
        RequestPermissionProfile {
            network: Some(NetworkPermissions {
                enabled: Some(true),
            }),
            file_system: Some(FileSystemPermissions::from_read_write_roots(
                Some(vec![read_path]),
                Some(vec![write_path]),
            )),
        }
    );
    assert_eq!(request.cwd, Some(cwd.into()));
    assert_eq!(request.environment_id.as_deref(), Some("remote"));
}

#[tokio::test]
async fn exec_approval_uses_approval_id_when_present() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    handle_exec_approval_request(
        &mut chat,
        "sub-short",
        ExecApprovalRequestEvent {
            kind: Default::default(),
            call_id: "call-parent".into(),
            approval_id: Some("approval-subcommand".into()),
            turn_id: "turn-short".into(),
            environment_id: None,
            command: vec!["bash".into(), "-lc".into(), "echo hello world".into()],
            cwd: AbsolutePathBuf::current_dir().expect("current dir"),
            reason: Some(
                "this is a test reason such as one that would be produced by the model".into(),
            ),
            network_approval_context: None,
            proposed_execpolicy_amendment: None,
            proposed_network_policy_amendments: None,
            additional_permissions: None,
            available_decisions: None,
        },
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    let mut found = false;
    while let Ok(app_ev) = rx.try_recv() {
        if let AppEvent::SubmitThreadOp {
            op: Op::ExecApproval { id, decision, .. },
            ..
        } = app_ev
        {
            assert_eq!(id, "approval-subcommand");
            assert_matches!(
                decision,
                genex_app_server_protocol::CommandExecutionApprovalDecision::Accept
            );
            found = true;
            break;
        }
    }
    assert!(found, "expected ExecApproval op to be sent");
}
