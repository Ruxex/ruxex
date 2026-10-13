use std::ffi::OsString;
use std::time::Duration;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::PidBackend;
use super::PidFileState;
use super::PidLogTail;
use super::PidRecord;
use super::read_stderr_log_tail;
use super::stderr_log_file_for_pid_file;
use super::try_lock_file;

#[tokio::test]
async fn locked_empty_pid_file_is_treated_as_active_reservation() {
    let temp = TempDir::new().expect("temp dir");
    let pid_file = temp.path().join("app-server.pid");
    tokio::fs::write(&pid_file, "")
        .await
        .expect("write pid file");
    let backend = PidBackend::new(temp.path().join("genex"), pid_file.clone(), None);
    let reservation = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&backend.lock_file)
        .await
        .expect("open pid lock file");
    assert!(try_lock_file(&reservation).expect("lock reservation"));

    assert_eq!(
        backend.read_pid_file_state().await.expect("read pid"),
        PidFileState::Starting
    );
    assert!(pid_file.exists());
}

#[tokio::test]
async fn unlocked_empty_pid_file_is_treated_as_stale_reservation() {
    let temp = TempDir::new().expect("temp dir");
    let pid_file = temp.path().join("app-server.pid");
    tokio::fs::write(&pid_file, "")
        .await
        .expect("write pid file");
    let backend = PidBackend::new(temp.path().join("genex"), pid_file.clone(), None);

    assert_eq!(
        backend.read_pid_file_state().await.expect("read pid"),
        PidFileState::Missing
    );
    assert!(!pid_file.exists());
}

#[tokio::test]
async fn stale_record_cleanup_preserves_a_newer_record() {
    let temp = TempDir::new().expect("temp dir");
    let pid_file = temp.path().join("app-server.pid");
    let backend = PidBackend::new(temp.path().join("genex"), pid_file.clone(), None);
    let stale = PidRecord {
        pid: 1,
        process_start_time: "old".to_string(),
        process_identity: None,
        executable_identity: None,
    };
    let current = PidRecord {
        pid: 2,
        process_start_time: "new".to_string(),
        process_identity: None,
        executable_identity: None,
    };
    tokio::fs::write(
        &pid_file,
        serde_json::to_vec(&current).expect("serialize pid"),
    )
    .await
    .expect("write pid file");

    assert_eq!(
        backend
            .refresh_after_stale_record(&stale)
            .await
            .expect("cleanup"),
        PidFileState::Running(current)
    );
}

#[tokio::test]
async fn read_stderr_log_tail_returns_recent_complete_lines() {
    let temp = TempDir::new().expect("temp dir");
    let pid_file = temp.path().join("app-server.pid");
    let log_file = stderr_log_file_for_pid_file(&pid_file);
    let contents = format!("{}\nrecent error\nusage", "x".repeat(4100));
    tokio::fs::write(&log_file, contents)
        .await
        .expect("write stderr log");

    assert_eq!(
        read_stderr_log_tail(&pid_file)
            .await
            .expect("read log")
            .expect("tail"),
        PidLogTail {
            path: log_file,
            contents: "recent error\nusage".to_string(),
        }
    );
}

#[tokio::test]
async fn stop_waits_for_a_live_reservation_to_resolve() {
    let temp = TempDir::new().expect("temp dir");
    let pid_file = temp.path().join("app-server.pid");
    tokio::fs::write(&pid_file, "")
        .await
        .expect("write pid file");
    let backend = PidBackend::new(temp.path().join("genex"), pid_file.clone(), None);
    let reservation = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&backend.lock_file)
        .await
        .expect("open pid lock file");
    assert!(try_lock_file(&reservation).expect("lock reservation"));
    let cleanup = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(reservation);
        tokio::fs::remove_file(pid_file)
            .await
            .expect("remove pid file");
    });

    backend.stop().await.expect("stop");
    cleanup.await.expect("cleanup task");
}

#[test]
fn host_command_args_replace_the_managed_app_server_command() {
    let temp = TempDir::new().expect("temp dir");
    let backend = PidBackend::new(
        temp.path().join("genex"),
        temp.path().join("app-server.pid"),
        Some(vec![
            OsString::from("__genex-app-server-daemon"),
            OsString::from("--listen"),
            OsString::from("unix://"),
        ]),
    );

    assert_eq!(
        backend.command_args(),
        vec![
            OsString::from("__genex-app-server-daemon"),
            OsString::from("--listen"),
            OsString::from("unix://"),
        ]
    );
}
