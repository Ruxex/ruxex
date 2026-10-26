//! Budgeted integrity checks that report confirmed corruption to database owners.

use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Eq, PartialEq)]
pub(super) enum QuickCheckOutcome {
    Complete,
    Incomplete,
    CorruptedNeedsFixed,
    Skipped,
}

/// Keeps track of databases that have already had a startup quick-check attempt.
#[derive(Clone, Debug, Default)]
pub(super) struct SqliteQuickCheckManager(Arc<Mutex<HashSet<PathBuf>>>);

impl SqliteQuickCheckManager {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) async fn quick_check_once(
        &self,
        pool: &SqlitePool,
        path: &Path,
        budget: Duration,
    ) -> anyhow::Result<QuickCheckOutcome> {
        if !self.0.lock().await.insert(path.to_path_buf()) {
            return Ok(QuickCheckOutcome::Skipped);
        }
        quick_check(pool, Instant::now() + budget).await
    }
}

async fn quick_check(pool: &SqlitePool, deadline: Instant) -> anyhow::Result<QuickCheckOutcome> {
    let mut connection = pool.acquire().await?;
    connection
        .lock_handle()
        .await?
        .set_progress_handler(/*num_ops*/ 1_000, move || Instant::now() < deadline);

    let result = sqlx::query_scalar::<_, String>("PRAGMA quick_check(1)")
        .fetch_one(&mut *connection)
        .await;
    connection.lock_handle().await?.remove_progress_handler();
    drop(connection);

    match result.map_err(anyhow::Error::from) {
        Ok(result) if result == "ok" => Ok(QuickCheckOutcome::Complete),
        Ok(_) => Ok(QuickCheckOutcome::CorruptedNeedsFixed),
        Err(error) if crate::is_sqlite_corruption_error(&error) => {
            Ok(QuickCheckOutcome::CorruptedNeedsFixed)
        }
        Err(_) => Ok(QuickCheckOutcome::Incomplete),
    }
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
