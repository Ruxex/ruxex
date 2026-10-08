use crate::RolloutItem;
use crate::config::RolloutConfig;
use crate::config::RolloutConfigView;
use crate::list::Cursor;
use crate::list::SortDirection;
use crate::list::ThreadSortKey;
use crate::metadata;
use crate::sqlite_metrics;
use anyhow::Context;
use chrono::DateTime;
use chrono::Utc;
use genex_protocol::ThreadId;
use genex_protocol::protocol::SessionSource;
pub use genex_state::LogEntry;
use genex_state::SqliteConfig;
use genex_state::ThreadMetadataBuilder;
use genex_utils_path::normalize_for_path_comparison;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tracing::info;
use tracing::warn;

/// Core-facing handle to the SQLite-backed state runtime.
pub type StateDbHandle = Arc<genex_state::StateRuntime>;

#[cfg(not(test))]
const STARTUP_BACKFILL_POLL_INTERVAL: Duration = Duration::from_secs(1);
#[cfg(test)]
const STARTUP_BACKFILL_POLL_INTERVAL: Duration = Duration::from_millis(10);
#[cfg(not(test))]
const STARTUP_BACKFILL_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
const STARTUP_BACKFILL_WAIT_TIMEOUT: Duration = Duration::from_secs(2);

/// Initialize the state runtime for thread state persistence.
///
/// This is the process entry point for local state: it opens the SQLite-backed
/// runtime, applies rollout metadata backfills as needed, and returns the
/// initialized handle.
pub async fn init(config: &impl RolloutConfigView) -> Option<StateDbHandle> {
    let config = RolloutConfig::from_view(config);
    match try_init_with_roots(config.genex_home, config.sqlite, config.model_provider_id).await {
        Ok(runtime) => Some(runtime),
        Err(err) => {
            emit_startup_warning(&format!("failed to initialize state runtime: {err:#}"));
            None
        }
    }
}

/// Initialize the state runtime and return any initialization error to the caller.
///
/// Prefer [`init`] unless the caller needs to surface the exact failure after
/// tracing or UI setup has completed.
pub async fn try_init(config: &impl RolloutConfigView) -> anyhow::Result<StateDbHandle> {
    let config = RolloutConfig::from_view(config);
    try_init_with_roots(config.genex_home, config.sqlite, config.model_provider_id).await
}

async fn try_init_with_roots(
    genex_home: PathBuf,
    sqlite: SqliteConfig,
    default_model_provider_id: String,
) -> anyhow::Result<StateDbHandle> {
    try_init_with_roots_inner(
        genex_home,
        sqlite,
        default_model_provider_id,
        /*backfill_lease_seconds*/ None,
    )
    .await
}

#[cfg(test)]
async fn try_init_with_roots_and_backfill_lease(
    genex_home: PathBuf,
    sqlite: SqliteConfig,
    default_model_provider_id: String,
    backfill_lease_seconds: i64,
) -> anyhow::Result<StateDbHandle> {
    try_init_with_roots_inner(
        genex_home,
        sqlite,
        default_model_provider_id,
        Some(backfill_lease_seconds),
    )
    .await
}

async fn try_init_with_roots_inner(
    genex_home: PathBuf,
    sqlite: SqliteConfig,
    default_model_provider_id: String,
    backfill_lease_seconds: Option<i64>,
) -> anyhow::Result<StateDbHandle> {
    let runtime =
        genex_state::StateRuntime::init(sqlite.clone(), default_model_provider_id.clone())
            .await
            .with_context(|| {
                format!(
                    "failed to initialize state runtime at {}",
                    sqlite.home().display()
                )
            })?;
    let backfill_gate_started = Instant::now();
    let backfill_gate_result = wait_for_backfill_gate(
        runtime.as_ref(),
        genex_home.as_path(),
        default_model_provider_id.as_str(),
        backfill_lease_seconds,
    )
    .await;
    genex_state::record_backfill_gate(
        /*telemetry*/ None,
        backfill_gate_started.elapsed(),
        &backfill_gate_result,
    );
    if let Err(err) = backfill_gate_result {
        runtime.close().await;
        return Err(err);
    }
    Ok(runtime)
}

async fn wait_for_backfill_gate(
    runtime: &genex_state::StateRuntime,
    genex_home: &Path,
    default_model_provider_id: &str,
    backfill_lease_seconds: Option<i64>,
) -> anyhow::Result<()> {
    let wait_started = Instant::now();
    let mut reported_wait = false;
    loop {
        let backfill_state = runtime.get_backfill_state().await.map_err(|err| {
            anyhow::anyhow!(
                "failed to read backfill state at {}: {err}",
                genex_home.display()
            )
        })?;
        if backfill_state.status == genex_state::BackfillStatus::Complete {
            return Ok(());
        }

        if let Some(backfill_lease_seconds) = backfill_lease_seconds {
            metadata::backfill_sessions_with_lease(
                runtime,
                genex_home,
                default_model_provider_id,
                backfill_lease_seconds,
            )
            .await;
        } else {
            metadata::backfill_sessions(runtime, genex_home, default_model_provider_id).await;
        }
        let backfill_state = runtime.get_backfill_state().await.map_err(|err| {
            anyhow::anyhow!(
                "failed to read backfill state at {} after startup backfill: {err}",
                genex_home.display()
            )
        })?;
        if backfill_state.status == genex_state::BackfillStatus::Complete {
            return Ok(());
        }
        if wait_started.elapsed() >= STARTUP_BACKFILL_WAIT_TIMEOUT {
            return Err(anyhow::anyhow!(
                "timed out waiting for state db backfill at {} after {:?} (status: {})",
                genex_home.display(),
                STARTUP_BACKFILL_WAIT_TIMEOUT,
                backfill_state.status.as_str()
            ));
        }

        let message = format!(
            "state db backfill is {} at {}; waiting up to {:?} before retrying startup initialization",
            backfill_state.status.as_str(),
            genex_home.display(),
            STARTUP_BACKFILL_WAIT_TIMEOUT,
        );
        if reported_wait {
            info!("{message}");
        } else {
            emit_startup_warning(&message);
            reported_wait = true;
        }
        tokio::time::sleep(STARTUP_BACKFILL_POLL_INTERVAL).await;
    }
}

fn emit_startup_warning(message: &str) {
    warn!("{message}");
    if !tracing::dispatcher::has_been_set() {
        #[allow(clippy::print_stderr)]
        {
            eprintln!("{message}");
        }
    }
}

/// Open the DB if it exists and its startup backfill has already completed.
///
/// Unlike [`init`], this helper does not run rollout backfill. It is for
/// optional local reads from non-owning contexts such as remote app-server mode.
pub async fn get_state_db(config: &impl RolloutConfigView) -> Option<StateDbHandle> {
    let state_path = config.sqlite_config().state_db_path();
    if !tokio::fs::try_exists(&state_path).await.unwrap_or(false) {
        genex_state::record_fallback(
            "get_state_db",
            "db_unavailable",
            /*telemetry_override*/ None,
        );
        return None;
    }
    let runtime = match genex_state::StateRuntime::init(
        config.sqlite_config().clone(),
        config.model_provider_id().to_string(),
    )
    .await
    {
        Ok(runtime) => runtime,
        Err(_) => {
            genex_state::record_fallback(
                "get_state_db",
                "db_error",
                /*telemetry_override*/ None,
            );
            return None;
        }
    };
    require_backfill_complete(runtime, config.sqlite_config().home()).await
}

/// Build a SQLite telemetry recorder backed by an OTEL metrics client.
pub fn sqlite_telemetry_recorder(
    metrics: genex_otel::MetricsClient,
    originator: &str,
) -> genex_state::DbTelemetryHandle {
    sqlite_metrics::recorder(metrics, originator)
}

async fn require_backfill_complete(
    runtime: StateDbHandle,
    genex_home: &Path,
) -> Option<StateDbHandle> {
    match runtime.get_backfill_state().await {
        Ok(state) if state.status == genex_state::BackfillStatus::Complete => Some(runtime),
        Ok(state) => {
            warn!(
                "state db backfill not complete at {} (status: {})",
                genex_home.display(),
                state.status.as_str()
            );
            genex_state::record_fallback(
                "get_state_db",
                "backfill_incomplete",
                /*telemetry_override*/ None,
            );
            None
        }
        Err(err) => {
            warn!(
                "failed to read backfill state at {}: {err}",
                genex_home.display()
            );
            genex_state::record_fallback(
                "get_state_db",
                "db_error",
                /*telemetry_override*/ None,
            );
            None
        }
    }
}

fn cursor_to_anchor(cursor: Option<&Cursor>) -> Option<genex_state::Anchor> {
    let cursor = cursor?;
    let millis = cursor.timestamp().unix_timestamp_nanos() / 1_000_000;
    let millis = i64::try_from(millis).ok()?;
    let ts = chrono::DateTime::<Utc>::from_timestamp_millis(millis)?;
    Some(genex_state::Anchor {
        ts,
        id: cursor.thread_id(),
    })
}

pub fn normalize_cwd_for_state_db(cwd: &Path) -> PathBuf {
    normalize_for_path_comparison(cwd).unwrap_or_else(|_| cwd.to_path_buf())
}

/// List thread ids from SQLite for parity checks without rollout scanning.
#[allow(clippy::too_many_arguments)]
pub async fn list_thread_ids_db(
    context: Option<&genex_state::StateRuntime>,
    sqlite: &genex_state::SqliteConfig,
    page_size: usize,
    cursor: Option<&Cursor>,
    sort_key: ThreadSortKey,
    allowed_sources: &[SessionSource],
    archived_only: bool,
    stage: &str,
) -> Option<Vec<ThreadId>> {
    let ctx = context?;
    if ctx.sqlite() != sqlite {
        warn!(
            "state db SQLite home mismatch: expected {}, got {}",
            sqlite.home().display(),
            ctx.sqlite().home().display()
        );
        return None;
    }

    let anchor = cursor_to_anchor(cursor);
    let allowed_sources: Vec<String> = allowed_sources
        .iter()
        .map(|value| match serde_json::to_value(value) {
            Ok(Value::String(s)) => s,
            Ok(other) => other.to_string(),
            Err(_) => String::new(),
        })
        .collect();
    match ctx
        .list_thread_ids(
            page_size,
            anchor.as_ref(),
            match sort_key {
                ThreadSortKey::CreatedAt => genex_state::SortKey::CreatedAt,
                ThreadSortKey::UpdatedAt => genex_state::SortKey::UpdatedAt,
                ThreadSortKey::RecencyAt => genex_state::SortKey::RecencyAt,
            },
            allowed_sources.as_slice(),
            archived_only,
        )
        .await
    {
        Ok(ids) => Some(ids),
        Err(err) => {
            warn!("state db list_thread_ids failed during {stage}: {err}");
            None
        }
    }
}

/// List thread metadata from SQLite without rollout directory traversal.
#[allow(clippy::too_many_arguments)]
pub async fn list_threads_db(
    context: Option<&genex_state::StateRuntime>,
    sqlite: &genex_state::SqliteConfig,
    page_size: usize,
    cursor: Option<&Cursor>,
    sort_key: ThreadSortKey,
    sort_direction: SortDirection,
    allowed_sources: &[SessionSource],
    cwd_filters: Option<&[PathBuf]>,
    relation_filter: Option<genex_state::ThreadRelationFilter>,
    archived: bool,
    section: Option<Option<&str>>,
    project_id: Option<Option<&str>>,
    search_term: Option<&str>,
) -> Option<genex_state::ThreadsPage> {
    let ctx = context?;
    if ctx.sqlite() != sqlite {
        warn!(
            "state db SQLite home mismatch: expected {}, got {}",
            sqlite.home().display(),
            ctx.sqlite().home().display()
        );
        return None;
    }

    let mut anchor = cursor_to_anchor(cursor);
    let allowed_sources: Vec<String> = allowed_sources
        .iter()
        .map(|value| match serde_json::to_value(value) {
            Ok(Value::String(s)) => s,
            Ok(other) => other.to_string(),
            Err(_) => String::new(),
        })
        .collect();
    let normalized_cwd_filters = cwd_filters.map(|filters| {
        filters
            .iter()
            .map(|cwd| normalize_cwd_for_state_db(cwd))
            .collect::<Vec<_>>()
    });
    let state_sort_key = match sort_key {
        ThreadSortKey::CreatedAt => genex_state::SortKey::CreatedAt,
        ThreadSortKey::UpdatedAt => genex_state::SortKey::UpdatedAt,
        ThreadSortKey::RecencyAt => genex_state::SortKey::RecencyAt,
    };
    let state_sort_direction = match sort_direction {
        SortDirection::Asc => genex_state::SortDirection::Asc,
        SortDirection::Desc => genex_state::SortDirection::Desc,
    };

    if let Some(relation_filter) = relation_filter {
        let filters = genex_state::ThreadFilterOptions {
            archived_only: archived,
            section,
            allowed_sources: allowed_sources.as_slice(),
            cwd_filters: normalized_cwd_filters.as_deref(),
            project_id,
            anchor: anchor.as_ref(),
            sort_key: state_sort_key,
            sort_direction: state_sort_direction,
            search_term,
        };
        return match ctx
            .list_threads_by_relation(page_size, relation_filter, filters)
            .await
        {
            Ok(page) => Some(page),
            Err(err) => {
                warn!("state db list_threads failed: {err}");
                None
            }
        };
    }

    let mut items = Vec::with_capacity(page_size);
    let mut next_anchor = None;
    let mut num_scanned_rows = 0usize;
    while items.len() < page_size {
        let filters = genex_state::ThreadFilterOptions {
            archived_only: archived,
            section,
            allowed_sources: allowed_sources.as_slice(),
            cwd_filters: normalized_cwd_filters.as_deref(),
            project_id,
            anchor: anchor.as_ref(),
            sort_key: state_sort_key,
            sort_direction: state_sort_direction,
            search_term,
        };
        let page = match ctx.list_threads(page_size - items.len(), filters).await {
            Ok(page) => page,
            Err(err) => {
                warn!("state db list_threads failed: {err}");
                return None;
            }
        };
        num_scanned_rows = num_scanned_rows.saturating_add(page.num_scanned_rows);
        next_anchor = page.next_anchor;

        for item in page.items {
            if let Some(existing_path) =
                crate::compression::existing_rollout_path(item.rollout_path.as_path()).await
            {
                let mut item = item;
                item.rollout_path = existing_path;
                items.push(item);
            } else {
                warn!(
                    "state db list_threads returned stale rollout path for thread {}: {}",
                    item.id,
                    item.rollout_path.display()
                );
                warn!("state db discrepancy during list_threads_db: stale_db_path_retained");
            }
        }

        if items.len() == page_size || next_anchor.is_none() {
            break;
        }
        anchor = next_anchor.clone();
    }
    Some(genex_state::ThreadsPage {
        items,
        parent_thread_ids: Default::default(),
        next_anchor,
        num_scanned_rows,
    })
}

/// Look up the rollout path for a thread id using SQLite.
pub async fn find_rollout_path_by_id(
    context: Option<&genex_state::StateRuntime>,
    thread_id: ThreadId,
    archived_only: Option<bool>,
    stage: &str,
) -> Option<PathBuf> {
    let ctx = context?;
    ctx.find_rollout_path_by_id(thread_id, archived_only)
        .await
        .unwrap_or_else(|err| {
            warn!("state db find_rollout_path_by_id failed during {stage}: {err}");
            None
        })
}

/// Reconcile rollout items into SQLite, falling back to scanning the rollout file.
pub async fn reconcile_rollout(
    context: Option<&genex_state::StateRuntime>,
    rollout_path: &Path,
    default_provider: &str,
    builder: Option<&ThreadMetadataBuilder>,
    items: &[RolloutItem],
    archived_only: Option<bool>,
) {
    let Some(ctx) = context else {
        return;
    };
    if builder.is_some() || !items.is_empty() {
        apply_rollout_items(
            Some(ctx),
            rollout_path,
            default_provider,
            builder,
            items,
            "reconcile_rollout",
            /*updated_at_override*/ None,
        )
        .await;
        return;
    }
    let outcome =
        match metadata::extract_metadata_from_rollout(rollout_path, default_provider).await {
            Ok(outcome) => outcome,
            Err(err) => {
                warn!(
                    "state db reconcile_rollout extraction failed {}: {err}",
                    rollout_path.display()
                );
                return;
            }
        };
    let mut metadata = outcome.metadata;
    metadata.cwd = normalize_cwd_for_state_db(&metadata.cwd);
    let existing_metadata = ctx.get_thread(metadata.id).await.ok().flatten();
    // Filesystem repair may seed a missing row, but it must not change an existing row's
    // selected rollout path. After `thread/revert`, a scan can find multiple immutable
    // rollouts for one thread and cannot know which one SQLite selected.
    if existing_metadata
        .as_ref()
        .is_some_and(|existing| existing.rollout_path.as_path() != rollout_path)
    {
        return;
    }
    if let Some(existing_metadata) = existing_metadata.as_ref() {
        metadata.prefer_existing_git_info(existing_metadata);
        metadata.prefer_existing_explicit_title(existing_metadata);
    }
    match archived_only {
        Some(true) if metadata.archived_at.is_none() => {
            metadata.archived_at = Some(metadata.updated_at);
        }
        Some(false) => {
            metadata.archived_at = None;
        }
        Some(true) | None => {}
    }
    if let Err(err) = ctx.upsert_thread(&metadata).await {
        warn!(
            "state db reconcile_rollout upsert failed {}: {err}",
            rollout_path.display()
        );
        return;
    }
}

/// Repair a thread's rollout path after filesystem fallback succeeds.
pub async fn read_repair_rollout_path(
    context: Option<&genex_state::StateRuntime>,
    thread_id: Option<ThreadId>,
    archived_only: Option<bool>,
    rollout_path: &Path,
) {
    let Some(ctx) = context else {
        return;
    };

    // Fast path: update an existing metadata row in place, but avoid writes when
    // read-repair computes no effective change.
    let mut saw_existing_metadata = false;
    if let Some(thread_id) = thread_id
        && let Ok(Some(metadata)) = ctx.get_thread(thread_id).await
    {
        // Filesystem repair may seed a missing row, but it must not change an existing row's
        // selected rollout path. After `thread/revert`, a scan can find multiple immutable
        // rollouts for one thread and cannot know which one SQLite selected.
        if metadata.rollout_path.as_path() != rollout_path {
            return;
        }
        saw_existing_metadata = true;
        let mut repaired = metadata.clone();
        repaired.rollout_path = rollout_path.to_path_buf();
        repaired.cwd = normalize_cwd_for_state_db(&repaired.cwd);
        match archived_only {
            Some(true) if repaired.archived_at.is_none() => {
                repaired.archived_at = Some(repaired.updated_at);
            }
            Some(false) => {
                repaired.archived_at = None;
            }
            Some(true) | None => {}
        }
        if repaired == metadata {
            return;
        }
        warn!("state db discrepancy during read_repair_rollout_path: upsert_needed (fast path)");
        if let Err(err) = ctx.upsert_thread(&repaired).await {
            warn!(
                "state db read-repair upsert failed for {}: {err}",
                rollout_path.display()
            );
        } else {
            return;
        }
    }

    // Slow path: when the row is missing/unreadable (or direct upsert failed),
    // rebuild metadata from rollout contents and reconcile it into SQLite.
    if !saw_existing_metadata {
        warn!("state db discrepancy during read_repair_rollout_path: upsert_needed (slow path)");
    }
    let default_provider = crate::list::read_session_meta_line(rollout_path)
        .await
        .ok()
        .and_then(|meta| meta.meta.model_provider)
        .unwrap_or_default();
    reconcile_rollout(
        Some(ctx),
        rollout_path,
        default_provider.as_str(),
        /*builder*/ None,
        &[],
        archived_only,
    )
    .await;
}

/// Apply rollout items incrementally to SQLite.
#[allow(clippy::too_many_arguments)]
pub async fn apply_rollout_items(
    context: Option<&genex_state::StateRuntime>,
    rollout_path: &Path,
    default_provider: &str,
    builder: Option<&ThreadMetadataBuilder>,
    items: &[RolloutItem],
    stage: &str,
    updated_at_override: Option<DateTime<Utc>>,
) {
    let Some(ctx) = context else {
        return;
    };
    let mut builder = match builder {
        Some(builder) => builder.clone(),
        None => match metadata::builder_from_items(items, rollout_path) {
            Some(builder) => builder,
            None => {
                warn!(
                    "state db apply_rollout_items missing builder during {stage}: {}",
                    rollout_path.display()
                );
                warn!("state db discrepancy during apply_rollout_items: {stage}, missing_builder");
                return;
            }
        },
    };
    if builder.model_provider.is_none() {
        builder.model_provider = Some(default_provider.to_string());
    }
    builder.rollout_path = rollout_path.to_path_buf();
    builder.cwd = normalize_cwd_for_state_db(&builder.cwd);
    if let Err(err) = ctx
        .apply_rollout_items(&builder, items, updated_at_override)
        .await
    {
        warn!(
            "state db apply_rollout_items failed during {stage} for {}: {err}",
            rollout_path.display()
        );
    }
}

pub async fn touch_thread_updated_at(
    context: Option<&genex_state::StateRuntime>,
    thread_id: Option<ThreadId>,
    updated_at: DateTime<Utc>,
    stage: &str,
) -> bool {
    let Some(ctx) = context else {
        return false;
    };
    let Some(thread_id) = thread_id else {
        return false;
    };
    ctx.touch_thread_updated_at(thread_id, updated_at)
        .await
        .unwrap_or_else(|err| {
            warn!("state db touch_thread_updated_at failed during {stage} for {thread_id}: {err}");
            false
        })
}

#[cfg(test)]
#[path = "state_db_tests.rs"]
mod tests;
