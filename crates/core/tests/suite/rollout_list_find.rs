#![allow(clippy::unwrap_used)]
use ruxex_utils_absolute_path::test_support::PathExt;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use chrono::Utc;
use ruxex_core::RolloutRecorder;
use ruxex_core::RolloutRecorderParams;
use ruxex_core::config::ConfigBuilder;
use ruxex_core::find_archived_thread_path_by_id_str;
use ruxex_core::find_thread_path_by_id_str;
use ruxex_protocol::ThreadId;
use ruxex_protocol::models::BaseInstructions;
use ruxex_protocol::protocol::SessionSource;
use ruxex_rollout::find_thread_meta_by_name_str;
use ruxex_rollout::StateDbHandle;
use ruxex_state::StateRuntime;
use ruxex_state::ThreadMetadataBuilder;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use uuid::Uuid;

/// Create <subdir>/YYYY/MM/DD and write a minimal rollout file containing the
/// provided conversation id in the SessionMeta line. Returns the absolute path.
fn write_minimal_rollout_with_id_in_subdir(ruxex_home: &Path, subdir: &str, id: Uuid) -> PathBuf {
    let sessions = ruxex_home.join(subdir).join("2024/01/01");
    std::fs::create_dir_all(&sessions).unwrap();

    let file = sessions.join(format!("rollout-2024-01-01T00-00-00-{id}.jsonl"));
    write_minimal_rollout_with_id_at_path(&file, id);

    file
}

fn write_minimal_rollout_with_id_at_path(file: &Path, id: Uuid) {
    let mut f = std::fs::File::create(file).unwrap();
    // Minimal first line: session_meta with the id so content search can find it
    writeln!(
        f,
        "{}",
        serde_json::json!({
            "timestamp": "2024-01-01T00:00:00.000Z",
            "type": "session_meta",
            "payload": {
                "session_id": id,
                "id": id,
                "timestamp": "2024-01-01T00:00:00Z",
                "cwd": ".",
                "originator": "test",
                "cli_version": "test",
            }
        })
    )
    .unwrap();
}

/// Create sessions/YYYY/MM/DD and write a minimal rollout file containing the
/// provided conversation id in the SessionMeta line. Returns the absolute path.
fn write_minimal_rollout_with_id(ruxex_home: &Path, id: Uuid) -> PathBuf {
    write_minimal_rollout_with_id_in_subdir(ruxex_home, "sessions", id)
}

async fn upsert_thread_metadata(
    ruxex_home: &Path,
    thread_id: ThreadId,
    rollout_path: PathBuf,
) -> StateDbHandle {
    let runtime = StateRuntime::init(
        ruxex_state::SqliteConfig::new_for_testing(ruxex_home.abs()),
    )
    .await
    .unwrap();
    runtime
        .mark_backfill_complete(/*last_watermark*/ None)
        .await
        .unwrap();
    let mut builder = ThreadMetadataBuilder::new(
        thread_id,
        rollout_path,
        Utc::now(),
        SessionSource::default(),
    );
    builder.cwd = ruxex_home.to_path_buf();
    let metadata = builder.build("test-provider");
    runtime.upsert_thread(&metadata).await.unwrap();
    runtime
}

#[tokio::test]
async fn find_locates_rollout_file_by_id() {
    let home = TempDir::new().unwrap();
    let id = Uuid::new_v4();
    let expected = write_minimal_rollout_with_id(home.path(), id);

    let found =
        find_thread_path_by_id_str(home.path(), &id.to_string(), /*state_db_ctx*/ None)
            .await
            .unwrap();

    assert_eq!(found.unwrap(), expected);
}

#[tokio::test]
async fn find_handles_gitignore_covering_ruxex_home_directory() {
    let repo = TempDir::new().unwrap();
    let ruxex_home = repo.path().join(".ruxex");
    std::fs::create_dir_all(&ruxex_home).unwrap();
    std::fs::write(repo.path().join(".gitignore"), ".ruxex/**\n").unwrap();
    let id = Uuid::new_v4();
    let expected = write_minimal_rollout_with_id(&ruxex_home, id);

    let found =
        find_thread_path_by_id_str(&ruxex_home, &id.to_string(), /*state_db_ctx*/ None)
            .await
            .unwrap();

    assert_eq!(found, Some(expected));
}

#[tokio::test]
async fn find_prefers_sqlite_path_by_id() {
    let home = TempDir::new().unwrap();
    let id = Uuid::new_v4();
    let thread_id = ThreadId::from_string(&id.to_string()).unwrap();
    let db_path = home.path().join(format!(
        "sessions/2030/12/30/rollout-2030-12-30T00-00-00-{id}.jsonl"
    ));
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    write_minimal_rollout_with_id_at_path(&db_path, id);
    write_minimal_rollout_with_id(home.path(), id);
    let state_db = upsert_thread_metadata(home.path(), thread_id, db_path.clone()).await;

    let found = find_thread_path_by_id_str(home.path(), &id.to_string(), Some(&state_db))
        .await
        .unwrap();

    assert_eq!(found, Some(db_path));
}

#[tokio::test]
async fn find_falls_back_to_filesystem_when_sqlite_has_no_match() {
    let home = TempDir::new().unwrap();
    let id = Uuid::new_v4();
    let expected = write_minimal_rollout_with_id(home.path(), id);
    let unrelated_id = Uuid::new_v4();
    let unrelated_thread_id = ThreadId::from_string(&unrelated_id.to_string()).unwrap();
    let unrelated_path = home
        .path()
        .join("sessions/2030/12/30/rollout-2030-12-30T00-00-00-unrelated.jsonl");
    let state_db = upsert_thread_metadata(home.path(), unrelated_thread_id, unrelated_path).await;

    let found = find_thread_path_by_id_str(home.path(), &id.to_string(), Some(&state_db))
        .await
        .unwrap();

    assert_eq!(found, Some(expected));
}

#[tokio::test]
async fn find_ignores_granular_gitignore_rules() {
    let home = TempDir::new().unwrap();
    let id = Uuid::new_v4();
    let expected = write_minimal_rollout_with_id(home.path(), id);
    std::fs::write(home.path().join("sessions/.gitignore"), "*.jsonl\n").unwrap();

    let found =
        find_thread_path_by_id_str(home.path(), &id.to_string(), /*state_db_ctx*/ None)
            .await
            .unwrap();

    assert_eq!(found, Some(expected));
}

#[tokio::test]
async fn find_locates_rollout_file_written_by_recorder() -> std::io::Result<()> {
    // Ensures the name-based finder locates a rollout produced by the real recorder.
    let home = TempDir::new().unwrap();
    let config = ConfigBuilder::default()
        .ruxex_home(home.path().to_path_buf())
        .build()
        .await?;
    let thread_id = ThreadId::new();
    let thread_name = "named thread";
    let recorder = RolloutRecorder::new(
        &config,
        RolloutRecorderParams::new(
            thread_id,
            /*forked_from_id*/ None,
            /*parent_thread_id*/ None,
            SessionSource::Exec,
            /*thread_source*/ None,
            "test_originator".to_string(),
            BaseInstructions::default(),
            Vec::new(),
        ),
    )
    .await?;
    recorder.persist().await?;
    recorder.flush().await?;

    let index_path = home.path().join("session_index.jsonl");
    std::fs::write(
        &index_path,
        format!(
            "{}\n",
            serde_json::json!({
                "id": thread_id,
                "thread_name": thread_name,
                "updated_at": "2024-01-01T00:00:00Z"
            })
        ),
    )?;

    let found =
        find_thread_meta_by_name_str(home.path(), thread_name, /*state_db_ctx*/ None).await?;

    let (path, session_meta) = found.expect("expected rollout path to be found");
    assert_eq!(session_meta.meta.id, thread_id);
    assert!(path.exists());
    let contents = std::fs::read_to_string(&path)?;
    assert!(contents.contains(&thread_id.to_string()));
    recorder.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn find_archived_locates_rollout_file_by_id() {
    let home = TempDir::new().unwrap();
    let id = Uuid::new_v4();
    let expected = write_minimal_rollout_with_id_in_subdir(home.path(), "archived_sessions", id);

    let found = find_archived_thread_path_by_id_str(
        home.path(),
        &id.to_string(),
        /*state_db_ctx*/ None,
    )
    .await
    .unwrap();

    assert_eq!(found, Some(expected));
}
