//! Regression coverage for environment snapshots and pending configuration.

use super::*;
use futures::FutureExt;
use ruxex_exec_server::EnvironmentManager;
use ruxex_exec_server::LOCAL_ENVIRONMENT_ID;
use ruxex_otel::SessionTelemetry;
use ruxex_protocol::ThreadId;
use ruxex_protocol::models::PermissionProfile;
use ruxex_protocol::models::PermissionProfileSnapshot;
use ruxex_protocol::protocol::EnvironmentConfig;
use ruxex_protocol::protocol::EnvironmentConfigState;
use ruxex_protocol::protocol::SessionSource;
use ruxex_protocol::protocol::TurnEnvironmentSelection;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_path_uri::PathUri;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::time::timeout;

fn test_environment_config() -> EnvironmentConfig {
    EnvironmentConfig {
        allow_login_shell: true,
        workspace_roots: Vec::new(),
        use_legacy_landlock: false,
        permission_profile: PermissionProfileSnapshot::legacy(PermissionProfile::workspace_write()),
        shell_environment_policy: Default::default(),
        exec_policy: None,
        mcp_policy: None,
        network_policy: None,
        selected_capability_roots: Vec::new(),
    }
}

#[tokio::test]
async fn unpolled_snapshot_does_not_delay_canceling_a_removed_environment() {
    let cwd = PathUri::from_abs_path(&AbsolutePathBuf::current_dir().expect("cwd"));
    let selection = TurnEnvironmentSelection {
        environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: cwd.clone(),
        workspace_roots: vec![cwd],
        config: EnvironmentConfigState::Pending,
    };
    let config = test_environment_config();
    let environments = ThreadEnvironments::new(
        Arc::new(EnvironmentManager::default_for_tests()),
        crate::shell::default_user_shell(),
        config.clone(),
        ShellSnapshot::disabled(),
        TurnEnvironmentSnapshot::default(),
        /*non_blocking_snapshots*/ false,
    );
    environments.update_selections(&[selection], &config);
    let starting = environments
        .snapshot()
        .await
        .starting()
        .next()
        .expect("pending environment should remain starting")
        .clone();
    let held_snapshot = environments.snapshot();

    environments.update_selections(&[], &config);

    let error = timeout(Duration::from_secs(5), starting.wait_until_ready())
        .await
        .expect("removing the environment should cancel the wait")
        .expect_err("removed pending environment should fail");
    assert!(
        error
            .to_string()
            .contains("environment configuration was canceled")
    );
    assert!(matches!(
        held_snapshot.await.environments.as_slice(),
        [TurnEnvironmentState::Failed { .. }]
    ));
}

#[tokio::test]
async fn credential_refresh_does_not_restore_a_removed_environment() {
    let home = tempfile::tempdir().expect("home");
    let cwd = AbsolutePathBuf::from_absolute_path(home.path()).expect("cwd");
    let cwd_uri = PathUri::from_abs_path(&cwd);
    let session_id = ThreadId::new();
    let telemetry = SessionTelemetry::new(
        session_id,
        "test",
        "test",
        /*account_id*/ None,
        /*account_email*/ None,
        /*auth_mode*/ None,
        "test".to_string(),
        /*log_user_prompts*/ false,
        "test".to_string(),
        SessionSource::Cli,
    );
    let (broker, _) = watch::channel(SnapshotCredentialBrokerState::Starting);
    let shell_snapshot = ShellSnapshot::new(
        cwd,
        session_id,
        telemetry,
        /*state_db*/ None,
        Some(broker),
        /*prefer_executor_snapshots*/ false,
    );
    let config = test_environment_config();
    let environments = Arc::new(ThreadEnvironments::new(
        Arc::new(EnvironmentManager::default_for_tests()),
        crate::shell::default_user_shell(),
        config.clone(),
        shell_snapshot,
        TurnEnvironmentSnapshot::default(),
        /*non_blocking_snapshots*/ false,
    ));
    environments.update_selections(
        &[TurnEnvironmentSelection {
            environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
            cwd: cwd_uri.clone(),
            workspace_roots: vec![cwd_uri],
            config: EnvironmentConfigState::FromThread,
        }],
        &config,
    );
    let resolution = environments.environments.lock().unwrap()[0]
        .resolution
        .clone();
    let resolved = resolution.await.expect("local is ready");
    let (refresh_paused_tx, refresh_paused_rx) = oneshot::channel();
    let (resume_refresh_tx, resume_refresh_rx) = std::sync::mpsc::channel();
    environments.environments.lock().unwrap()[0].resolution = async move {
        refresh_paused_tx.send(()).expect("signal refresh paused");
        resume_refresh_rx.recv().expect("resume refresh");
        Ok(resolved)
    }
    .boxed()
    .shared();

    let refreshing = Arc::clone(&environments);
    let refresh = tokio::task::spawn_blocking(move || {
        refreshing.set_snapshot_credential_broker(SnapshotCredentialBrokerState::Unavailable);
    });
    timeout(Duration::from_secs(5), refresh_paused_rx)
        .await
        .expect("credential refresh should reach the old list")
        .expect("refresh paused");
    let (removal_started_tx, removal_started_rx) = oneshot::channel();
    let removing = Arc::clone(&environments);
    let mut removal = tokio::task::spawn_blocking(move || {
        removal_started_tx.send(()).expect("signal removal started");
        removing.update_selections(&[], &config);
    });
    timeout(Duration::from_secs(5), removal_started_rx)
        .await
        .expect("removal should start")
        .expect("removal started");
    let early_removal = timeout(Duration::from_secs(1), &mut removal).await.ok();
    let removal_finished_early = early_removal.is_some();

    resume_refresh_tx
        .send(())
        .expect("release credential refresh");
    refresh.await.expect("credential refresh finished");
    match early_removal {
        Some(result) => result.expect("removal finished"),
        None => removal.await.expect("removal finished"),
    }
    assert!(environments.selections().is_empty());
    assert!(
        !removal_finished_early,
        "removal should wait until the credential refresh finishes"
    );
}
