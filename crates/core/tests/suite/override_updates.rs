use anyhow::Result;
use ruxex_core::config::Constrained;
use ruxex_protocol::config_types::CollaborationMode;
use ruxex_protocol::config_types::ModeKind;
use ruxex_protocol::config_types::Settings;
use ruxex_protocol::protocol::AskForApproval;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::Op;
use core_test_support::TempDirExt;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_ruxex::local_selections;
use core_test_support::test_ruxex::test_ruxex;
use core_test_support::wait_for_event;
use tempfile::TempDir;

fn collab_mode_with_instructions(instructions: Option<&str>) -> CollaborationMode {
    CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model: "legacy-model".to_string(),
            reasoning_effort: None,
            developer_instructions: instructions.map(str::to_string),
        },
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_settings_update_without_user_turn_does_not_record_permissions_update() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let mut builder = test_ruxex().with_config(|config| {
        config.permissions.approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
    });
    let test = builder.build(&server).await?;

    core_test_support::submit_thread_settings(
        &test.ruxex,
        ruxex_protocol::protocol::ThreadSettingsOverrides {
            approval_policy: Some(AskForApproval::Never),
            ..Default::default()
        },
    )
    .await?;

    test.ruxex.submit(Op::Shutdown).await?;
    wait_for_event(&test.ruxex, |ev| matches!(ev, EventMsg::ShutdownComplete)).await;

    let rollout_path = test.ruxex.rollout_path().expect("rollout path");
    assert!(
        !rollout_path.exists(),
        "did not expect a rollout before a new user turn"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_settings_update_without_user_turn_does_not_record_environment_update() -> Result<()>
{
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let test = test_ruxex().build(&server).await?;
    let new_cwd = TempDir::new()?;

    core_test_support::submit_thread_settings(
        &test.ruxex,
        ruxex_protocol::protocol::ThreadSettingsOverrides {
            environments: Some(local_selections(new_cwd.abs())),
            ..Default::default()
        },
    )
    .await?;

    test.ruxex.submit(Op::Shutdown).await?;
    wait_for_event(&test.ruxex, |ev| matches!(ev, EventMsg::ShutdownComplete)).await;

    let rollout_path = test.ruxex.rollout_path().expect("rollout path");
    assert!(
        !rollout_path.exists(),
        "did not expect a rollout before a new user turn"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thread_settings_update_without_user_turn_does_not_record_collaboration_update()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let test = test_ruxex().build(&server).await?;
    let collab_text = "override collaboration instructions";
    let collaboration_mode = collab_mode_with_instructions(Some(collab_text));

    core_test_support::submit_thread_settings(
        &test.ruxex,
        ruxex_protocol::protocol::ThreadSettingsOverrides {
            collaboration_mode: Some(collaboration_mode),
            ..Default::default()
        },
    )
    .await?;

    test.ruxex.submit(Op::Shutdown).await?;
    wait_for_event(&test.ruxex, |ev| matches!(ev, EventMsg::ShutdownComplete)).await;

    let rollout_path = test.ruxex.rollout_path().expect("rollout path");
    assert!(
        !rollout_path.exists(),
        "did not expect a rollout before a new user turn"
    );

    Ok(())
}
