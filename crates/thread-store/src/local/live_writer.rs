use std::path::PathBuf;
use std::sync::Arc;

use ruxex_protocol::ThreadId;
use ruxex_protocol::protocol::ThreadHistoryMode;
use ruxex_rollout::RolloutConfig;
use ruxex_rollout::RolloutItem;
use ruxex_rollout::RolloutRecorder;
use ruxex_rollout::RolloutRecorderParams;
use ruxex_rollout::is_persisted_rollout_item;
use tracing::warn;

use super::LocalThreadStore;
use super::create_thread;
use crate::AppendThreadItemsParams;
use crate::CreateThreadParams;
use crate::ResumeThreadParams;
use crate::ThreadStoreError;
use crate::ThreadStoreResult;
use crate::types::canonical_history_mode_from_rollout_items;

const ROLLOUT_SIZE_BYTES_METRIC: &str = "ruxex.rollout.size_bytes";

pub(super) async fn create_thread(
    store: &LocalThreadStore,
    params: CreateThreadParams,
) -> ThreadStoreResult<()> {
    let thread_id = params.thread_id;
    let _live_writer_guard = store.live_writer_locks.lock(thread_id).await;
    let history_mode = params.history_mode;
    store.ensure_live_recorder_absent(thread_id).await?;
    let writer_lock = store.acquire_writer_lock(thread_id)?;
    let recorder = create_thread::create_thread(store, params, writer_lock.clone()).await?;
    store
        .insert_live_recorder(thread_id, recorder, thread_id, history_mode, writer_lock)
        .await
}

pub(super) async fn resume_thread(
    store: &LocalThreadStore,
    params: ResumeThreadParams,
) -> ThreadStoreResult<Arc<Vec<RolloutItem>>> {
    let _live_writer_guard = store.live_writer_locks.lock(params.thread_id).await;
    store.ensure_live_recorder_absent(params.thread_id).await?;
    let writer_lock = store.acquire_writer_lock(params.thread_id)?;
    let rollout_path = match params.rollout_path {
        Some(rollout_path) => rollout_path,
        None => {
            let resolved = if params.include_archived {
                super::thread_rollout_resolver::resolve_current_including_archived(
                    store,
                    params.thread_id,
                )
                .await?
            } else {
                super::thread_rollout_resolver::resolve_current(store, params.thread_id).await?
            };
            resolved
                .ok_or(ThreadStoreError::ThreadNotFound {
                    thread_id: params.thread_id,
                })?
                .path
        }
    };
    let rollout_path =
        super::read_thread::resolve_requested_rollout_path(store, rollout_path).await?;
    if !params.include_archived
        && super::helpers::rollout_path_is_archived(
            store.config.ruxex_home.as_path(),
            rollout_path.as_path(),
        )
    {
        return Err(ThreadStoreError::InvalidRequest {
            message: format!("thread {} is archived", params.thread_id),
        });
    }
    let history = match params.history {
        Some(history)
            if !matches!(
                history.first(),
                Some(RolloutItem::SessionMeta(meta)) if meta.meta.id == params.thread_id
            ) =>
        {
            history
        }
        Some(history)
            if params.history_revision.is_some()
                && params.history_revision
                    == super::history_revision::read(&rollout_path).await =>
        {
            history
        }
        _ => Arc::new(
            super::model_context::load_from_rollout_path(
                store,
                params.thread_id,
                rollout_path.as_path(),
            )
            .await?
            .items,
        ),
    };
    let history_mode = if history.is_empty() {
        super::read_thread::read_thread_by_rollout_path(
            store,
            rollout_path.clone(),
            params.include_archived,
            /*include_history*/ false,
        )
        .await?
        .history_mode
    } else {
        canonical_history_mode_from_rollout_items(&history)
    };
    let cwd = params
        .metadata
        .cwd
        .clone()
        .ok_or_else(|| ThreadStoreError::InvalidRequest {
            message: "local thread store requires a cwd".to_string(),
        })?;
    let config = RolloutConfig {
        ruxex_home: store.config.ruxex_home.clone(),
        sqlite: store.config.sqlite.clone(),
        cwd,
    };
    let rollout_id =
        ruxex_rollout::rollout_id_from_path(rollout_path.as_path()).ok_or_else(|| {
            ThreadStoreError::InvalidRequest {
                message: format!(
                    "rollout path `{}` does not have a canonical rollout filename",
                    rollout_path.display()
                ),
            }
        })?;
    let recorder = RolloutRecorder::new_with_writer_lock(
        &config,
        RolloutRecorderParams::resume(rollout_path),
        writer_lock.clone(),
    )
    .await
    .map_err(|err| ThreadStoreError::Internal {
        message: format!("failed to resume local thread recorder: {err}"),
    })?;
    store
        .insert_live_recorder(
            params.thread_id,
            recorder,
            rollout_id,
            history_mode,
            writer_lock,
        )
        .await?;
    Ok(history)
}

#[tracing::instrument(
    level = "trace",
    skip_all,
    fields(item_count = params.items.len())
)]
pub(super) async fn append_items(
    store: &LocalThreadStore,
    params: AppendThreadItemsParams,
) -> ThreadStoreResult<()> {
    write_and_project(
        store,
        params.thread_id,
        RolloutWriteOp::AppendItems(params.items),
    )
    .await
}

pub(super) async fn persist_thread(
    store: &LocalThreadStore,
    thread_id: ThreadId,
) -> ThreadStoreResult<()> {
    write_and_project(store, thread_id, RolloutWriteOp::Persist).await
}

pub(super) async fn flush_thread(
    store: &LocalThreadStore,
    thread_id: ThreadId,
) -> ThreadStoreResult<()> {
    write_and_project(store, thread_id, RolloutWriteOp::Flush).await
}

pub(super) async fn shutdown_thread(
    store: &LocalThreadStore,
    thread_id: ThreadId,
) -> ThreadStoreResult<()> {
    let mut pending_metadata = store.pending_thread_metadata.lock(thread_id).await;
    let _live_writer_guard = store.live_writer_locks.lock(thread_id).await;
    let (recorder, rollout_id, _history_mode) = live_writer_parts(store, thread_id).await?;
    let rollout_path = recorder.rollout_path().to_path_buf();
    recorder.shutdown().await.map_err(thread_store_io_error)?;
    if let Err(err) = super::thread_history_materialization::materialize_to_sqlite(
        store,
        rollout_id,
        rollout_path.as_path(),
    )
    .await
    {
        warn!("failed to project durable rollout during shutdown for {thread_id}: {err}");
    }
    sync_materialized_rollout_path(store, thread_id, rollout_path.as_path()).await?;
    if let Some(metrics) = ruxex_otel::global()
        && let Ok(metadata) = tokio::fs::metadata(&rollout_path).await
    {
        let size_bytes = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
        let _ = metrics.histogram(ROLLOUT_SIZE_BYTES_METRIC, size_bytes, &[]);
    }
    let rollout_exists = match tokio::fs::try_exists(&rollout_path).await {
        Ok(rollout_exists) => rollout_exists,
        Err(err) => {
            warn!(
                "failed to check rollout path after shutdown for {thread_id}; preserving pending metadata: {err}"
            );
            true
        }
    };
    store.live_recorders.lock().await.remove(&thread_id);
    drop(_live_writer_guard);
    if !rollout_exists && pending_metadata.take().is_some() {
        store.pending_thread_metadata.remove(thread_id).await;
    }
    Ok(())
}

pub(super) async fn discard_thread(
    store: &LocalThreadStore,
    thread_id: ThreadId,
) -> ThreadStoreResult<()> {
    let mut pending_metadata = store.pending_thread_metadata.lock(thread_id).await;
    let _live_writer_guard = store.live_writer_locks.lock(thread_id).await;
    if pending_metadata.take().is_some() {
        store.pending_thread_metadata.remove(thread_id).await;
    }
    let entry = store
        .live_recorders
        .lock()
        .await
        .remove(&thread_id)
        .ok_or(ThreadStoreError::ThreadNotFound { thread_id })?;
    entry
        .recorder
        .discard()
        .await
        .map_err(thread_store_io_error)
}

pub(super) async fn rollout_path(
    store: &LocalThreadStore,
    thread_id: ThreadId,
) -> ThreadStoreResult<PathBuf> {
    Ok(store
        .live_recorders
        .lock()
        .await
        .get(&thread_id)
        .ok_or(ThreadStoreError::ThreadNotFound { thread_id })?
        .recorder
        .rollout_path()
        .to_path_buf())
}

async fn sync_materialized_rollout_path(
    store: &LocalThreadStore,
    thread_id: ThreadId,
    rollout_path: &std::path::Path,
) -> ThreadStoreResult<()> {
    if ruxex_rollout::existing_rollout_path(rollout_path)
        .await
        .is_none()
    {
        return Ok(());
    }
    let Some(state_db) = store.state_db().await else {
        return Ok(());
    };
    let result: ThreadStoreResult<()> = async {
        let Some(mut metadata) =
            state_db
                .get_thread(thread_id)
                .await
                .map_err(|err| ThreadStoreError::Internal {
                    message: format!("failed to read thread metadata for {thread_id}: {err}"),
                })?
        else {
            return Ok(());
        };
        if metadata.rollout_path != rollout_path {
            metadata.rollout_path = rollout_path.to_path_buf();
            state_db
                .upsert_thread(&metadata)
                .await
                .map_err(|err| ThreadStoreError::Internal {
                    message: format!("failed to update thread metadata for {thread_id}: {err}"),
                })?;
        }
        Ok(())
    }
    .await;
    if let Err(err) = result {
        warn!("failed to sync materialized rollout path for thread {thread_id}: {err}");
    }
    Ok(())
}

fn thread_store_io_error(err: std::io::Error) -> ThreadStoreError {
    ThreadStoreError::Internal {
        message: err.to_string(),
    }
}

/// The rollout writer has three distinct lifecycle moments:
/// - `AppendItems` is normal turn/event persistence and adds new rollout records.
/// - `Persist` makes the thread durable before any turn items exist; locally this can write the
///   initial `SessionMeta`.
/// - `Flush` writes any rollout records already queued in the recorder and ensures they are
///   durably persisted.
///
/// Each can advance the rollout JSONL file on disk, so we need to make sure we materialize the
/// new data into the SQLite history tables (turns and items) as necessary.
enum RolloutWriteOp {
    AppendItems(Vec<RolloutItem>),
    Persist,
    Flush,
}

pub(super) async fn live_writer_parts(
    store: &LocalThreadStore,
    thread_id: ThreadId,
) -> ThreadStoreResult<(RolloutRecorder, ThreadId, ThreadHistoryMode)> {
    let live_recorders = store.live_recorders.lock().await;
    let entry = live_recorders
        .get(&thread_id)
        .ok_or(ThreadStoreError::ThreadNotFound { thread_id })?;
    Ok((entry.recorder.clone(), entry.rollout_id, entry.history_mode))
}

async fn write_and_project(
    store: &LocalThreadStore,
    thread_id: ThreadId,
    write_op: RolloutWriteOp,
) -> ThreadStoreResult<()> {
    // Every live write should have a recorder: create/resume installs one, while
    // shutdown/discard/delete removes it. Keep the lookup defensive so late writes fail after
    // teardown.
    let _live_writer_guard = store.live_writer_locks.lock(thread_id).await;
    let (recorder, rollout_id, _history_mode) = live_writer_parts(store, thread_id).await?;
    let sync_rollout_path = matches!(&write_op, RolloutWriteOp::Persist | RolloutWriteOp::Flush);
    let write_op = match write_op {
        RolloutWriteOp::AppendItems(mut items) => {
            items.retain(is_persisted_rollout_item);
            if items.is_empty() {
                return Ok(());
            }
            RolloutWriteOp::AppendItems(items)
        }
        RolloutWriteOp::Persist => RolloutWriteOp::Persist,
        RolloutWriteOp::Flush => RolloutWriteOp::Flush,
    };
    let rollout_path = recorder.rollout_path();
    // SQLite is a rebuildable view. The flush barrier must win before projection starts so it
    // can lag JSONL after failure, but can never get ahead of canonical history.
    durable_write(&recorder, write_op).await?;
    if let Err(err) = super::thread_history_materialization::materialize_to_sqlite(
        store,
        rollout_id,
        rollout_path,
    )
    .await
    {
        warn!("failed to project durable rollout for {thread_id}: {err}");
    }
    if sync_rollout_path {
        sync_materialized_rollout_path(store, thread_id, recorder.rollout_path()).await?;
    }
    Ok(())
}

async fn durable_write(recorder: &RolloutRecorder, write: RolloutWriteOp) -> ThreadStoreResult<()> {
    match write {
        RolloutWriteOp::AppendItems(items) => {
            recorder
                .record_canonical_items(items.as_slice())
                .await
                .map_err(thread_store_io_error)?;
            recorder.flush().await.map_err(thread_store_io_error)
        }
        RolloutWriteOp::Persist => recorder.persist().await.map_err(thread_store_io_error),
        RolloutWriteOp::Flush => recorder.flush().await.map_err(thread_store_io_error),
    }
}
