//! Exercises compression through local-store ownership, including detached recorder work.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use genex_protocol::ThreadId;
use genex_protocol::protocol::EventMsg;
use genex_protocol::protocol::UserMessageEvent;
use genex_rollout::RolloutItem;
use genex_rollout::RolloutRecorder;
use genex_rollout::WriterLockCoordinator;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

use super::LocalThreadStore;
use super::test_support::test_config;
use super::test_support::write_session_file;
use crate::ResumeThreadParams;
use crate::ThreadPersistenceMetadata;
use crate::ThreadStore;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> TestResult<(TempDir, ThreadId, PathBuf)> {
    let home = TempDir::new()?;
    let id = Uuid::new_v4();
    let path = write_session_file(home.path(), "2025-01-03T12-00-00", id)?;
    Ok((home, ThreadId::from_string(&id.to_string())?, path))
}

fn age(path: &Path) -> TestResult<()> {
    fs::OpenOptions::new().write(true).open(path)?.set_times(
        fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(8 * 86400)),
    )?;
    Ok(())
}

fn resume(thread_id: ThreadId, path: &Path, home: &Path) -> ResumeThreadParams {
    ResumeThreadParams {
        thread_id,
        rollout_path: Some(path.to_path_buf()),
        history: None,
        include_archived: true,
        metadata: ThreadPersistenceMetadata {
            cwd: Some(home.to_path_buf()),
        },
    }
}

fn message() -> RolloutItem {
    RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
        message: "acknowledged write".into(),
        ..Default::default()
    }))
}

async fn wait_until(ready: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("background file work completes");
}

async fn compress(home: &Path) -> TestResult<()> {
    let marker = home.join(".tmp/rollout-compression.lock");
    if marker.exists() {
        fs::remove_file(&marker)?;
    }
    genex_rollout::spawn_rollout_compression_worker(
        home.to_path_buf(),
        genex_rollout::RolloutCompressionTrigger::Startup,
    );
    // The marker proves startup; the maintenance lock proves every blocking job finished.
    wait_until(|| {
        marker.exists()
            && genex_rollout::try_acquire_rollout_maintenance_lock(home)
                .unwrap()
                .is_some()
    })
    .await;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn idle_recorder_retains_stable_thread_ownership_after_store_drop() -> TestResult<()> {
    let (home, thread_id, old_path) = fixture()?;
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let path = old_path.with_file_name(format!(
        "rollout-2025-01-03T12-00-00-{thread_id}_{}.jsonl",
        Uuid::new_v4()
    ));
    fs::rename(old_path, &path)?;
    let cold = write_session_file(home.path(), "2025-01-03T12-00-01", Uuid::new_v4())?;
    age(&cold)?;
    store
        .resume_thread(resume(thread_id, &path, home.path()))
        .await?;
    let recorder = store
        .live_recorders
        .lock()
        .await
        .get(&thread_id)
        .unwrap()
        .recorder
        .clone();
    drop(store);
    age(&path)?;
    let (mut expected, _, _) = RolloutRecorder::load_rollout_items(&path).await?;
    compress(home.path()).await?;
    assert!(path.exists());
    assert!(!path.with_extension("jsonl.zst").exists());
    assert!(cold.with_extension("jsonl.zst").exists());

    recorder.record_canonical_items(&[message()]).await?;
    drop(recorder);
    let locks = Arc::new(WriterLockCoordinator::new(home.path()));
    // No yield since enqueueing: only the background task can own the pending write now.
    assert!(
        matches!(locks.acquire(thread_id), Err(err) if err.kind() == std::io::ErrorKind::WouldBlock)
    );
    wait_until(|| locks.acquire(thread_id).is_ok()).await;
    age(&path)?;
    compress(home.path()).await?;
    assert!(!path.exists());
    expected.push(message());
    let (actual, _, _) = RolloutRecorder::load_rollout_items(&path).await?;
    assert_eq!(json!(actual), json!(expected));
    Ok(())
}
