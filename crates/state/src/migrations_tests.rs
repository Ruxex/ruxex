use genex_utils_absolute_path::test_support::PathExt;
use pretty_assertions::assert_eq;
use super::GOALS_MIGRATOR;
use super::LOGS_MIGRATOR;
use super::QUEUE_MIGRATOR;
use super::STATE_MIGRATOR;
use super::THREAD_HISTORY_MIGRATOR;
use crate::PINNED_THREAD_SECTION_ID;
use crate::PINNED_THREAD_SECTION_NAME;

#[test]
fn all_database_migrators_are_single_version_baselines() {
    for migrator in [
        &STATE_MIGRATOR,
        &LOGS_MIGRATOR,
        &GOALS_MIGRATOR,
        &QUEUE_MIGRATOR,
        &THREAD_HISTORY_MIGRATOR,
    ] {
        assert_eq!(
            migrator
                .migrations
                .iter()
                .map(|migration| migration.version)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }
}

#[tokio::test]
async fn state_baseline_contains_current_schema_and_seed_data() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("state database should open");
    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("state baseline should apply");

    let tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .expect("state tables should load");
    assert_eq!(
        tables,
        vec![
            "backfill_state",
            "project_idempotency_keys",
            "project_roots",
            "projects",
            "thread_attachments",
            "thread_dynamic_tools",
            "thread_sections",
            "thread_spawn_edges",
            "threads",
        ]
    );

    let indexes = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'index' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .expect("state indexes should load");
    assert_eq!(
        indexes,
        vec![
            "idx_projects_position",
            "idx_thread_attachments_thread_created_id",
            "idx_thread_dynamic_tools_thread",
            "idx_thread_spawn_edges_parent_status",
            "idx_threads_archived",
            "idx_threads_archived_cwd_created_at_ms",
            "idx_threads_archived_cwd_recency_at_ms",
            "idx_threads_archived_cwd_updated_at_ms",
            "idx_threads_created_at",
            "idx_threads_created_at_ms",
            "idx_threads_pinned_recency_at_ms",
            "idx_threads_project_id",
            "idx_threads_project_recency",
            "idx_threads_provider",
            "idx_threads_recency_at_ms",
            "idx_threads_section_position",
            "idx_threads_section_recency_at_ms",
            "idx_threads_source",
            "idx_threads_updated_at",
            "idx_threads_updated_at_ms",
            "idx_threads_visible_created_at_ms",
            "idx_threads_visible_recency_at_ms",
            "idx_threads_visible_updated_at_ms",
        ]
    );

    let triggers = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'trigger' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .expect("state triggers should load");
    assert_eq!(
        triggers,
        vec![
            "threads_created_at_ms_after_insert",
            "threads_created_at_ms_after_update",
            "threads_recency_at_after_insert",
            "threads_updated_at_ms_after_insert",
            "threads_updated_at_ms_after_update",
        ]
    );

    let pinned = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT id, name, appearance FROM thread_sections",
    )
    .fetch_one(&pool)
    .await
    .expect("pinned section should be seeded");
    assert_eq!(
        pinned,
        (
            PINNED_THREAD_SECTION_ID.to_string(),
            PINNED_THREAD_SECTION_NAME.to_string(),
            None,
        )
    );

    let backfill = sqlx::query_as::<_, (i64, String, Option<String>, Option<i64>)>(
        "SELECT id, status, last_watermark, last_success_at FROM backfill_state",
    )
    .fetch_one(&pool)
    .await
    .expect("backfill state should be seeded");
    assert_eq!(backfill, (1, "pending".to_string(), None, None));
    let updated_at = sqlx::query_scalar::<_, i64>("SELECT updated_at FROM backfill_state")
        .fetch_one(&pool)
        .await
        .expect("backfill timestamp should be seeded");
    assert!(updated_at > 0);

    pool.close().await;
}

#[tokio::test]
async fn state_baseline_triggers_derived_timestamps_and_recency() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());
    let pool = sqlite
        .open_read_write_pool(&sqlite.state_db_path())
        .await
        .expect("state database should open");
    STATE_MIGRATOR
        .run(&pool)
        .await
        .expect("state baseline should apply");

    sqlx::query(
        "INSERT INTO threads (id, rollout_path, created_at, updated_at, source, model_provider, cwd, title, sandbox_policy, approval_mode) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("thread-1")
    .bind("rollout.jsonl")
    .bind(1_700_000_000_i64)
    .bind(1_700_000_100_i64)
    .bind("cli")
    .bind("provider")
    .bind("/tmp")
    .bind("")
    .bind("read-only")
    .bind("on-request")
    .execute(&pool)
    .await
    .expect("thread should insert through the baseline schema");

    let timestamps = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        "SELECT created_at_ms, updated_at_ms, recency_at, recency_at_ms FROM threads WHERE id = ?",
    )
    .bind("thread-1")
    .fetch_one(&pool)
    .await
    .expect("derived timestamps should load");
    assert_eq!(timestamps, (1_700_000_000_000, 1_700_000_100_000, 1_700_000_100, 1_700_000_100_000));

    sqlx::query("UPDATE threads SET created_at = ?, updated_at = ? WHERE id = ?")
        .bind(1_700_000_200_i64)
        .bind(1_700_000_300_i64)
        .bind("thread-1")
        .execute(&pool)
        .await
        .expect("thread timestamps should update");
    let updated = sqlx::query_as::<_, (i64, i64)>(
        "SELECT created_at_ms, updated_at_ms FROM threads WHERE id = ?",
    )
    .bind("thread-1")
    .fetch_one(&pool)
    .await
    .expect("updated derived timestamps should load");
    assert_eq!(updated, (1_700_000_200_000, 1_700_000_300_000));

    pool.close().await;
}

#[tokio::test]
async fn auxiliary_baselines_preserve_current_schema_contracts() {
    let sqlite_home = crate::runtime::test_support::unique_temp_dir();
    tokio::fs::create_dir_all(&sqlite_home)
        .await
        .expect("sqlite home should be created");
    let _cleanup = scopeguard::guard(sqlite_home.clone(), |sqlite_home| {
        let _ = std::fs::remove_dir_all(sqlite_home);
    });
    let sqlite = crate::SqliteConfig::new_for_testing(sqlite_home.as_path().abs());

    let logs_pool = sqlite
        .open_read_write_pool(&sqlite.logs_db_path())
        .await
        .expect("logs database should open");
    LOGS_MIGRATOR
        .run(&logs_pool)
        .await
        .expect("logs baseline should apply");
    let log_columns = sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('logs')")
        .fetch_all(&logs_pool)
        .await
        .expect("logs columns should load");
    assert!(log_columns.iter().any(|name| name == "feedback_log_body"));
    assert!(!log_columns.iter().any(|name| name == "message"));
    logs_pool.close().await;

    let goals_pool = sqlite
        .open_read_write_pool(&sqlite.goals_db_path())
        .await
        .expect("goals database should open");
    GOALS_MIGRATOR
        .run(&goals_pool)
        .await
        .expect("goals baseline should apply");
    let goal_tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(&goals_pool)
    .await
    .expect("goal tables should load");
    assert_eq!(goal_tables, vec!["thread_goal_continuation_deferrals", "thread_goals"]);
    goals_pool.close().await;

    let queue_pool = sqlite
        .open_read_write_pool(&sqlite.queue_db_path())
        .await
        .expect("queue database should open");
    QUEUE_MIGRATOR
        .run(&queue_pool)
        .await
        .expect("queue baseline should apply");
    sqlx::query(
        "INSERT INTO queued_items (id, thread_id, payload_json, queue_order, created_at_ms, updated_at_ms) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("item-1")
    .bind("thread-1")
    .bind("{}")
    .bind(1_i64)
    .bind(1_i64)
    .bind(1_i64)
    .execute(&queue_pool)
    .await
    .expect("queue item should insert");
    let revision = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM queued_thread_revisions WHERE thread_id = ?",
    )
    .bind("thread-1")
    .fetch_one(&queue_pool)
    .await
    .expect("queue revision should be created");
    assert_eq!(revision, 1);
    queue_pool.close().await;

    let history_pool = sqlite
        .open_read_write_pool(&sqlite.thread_history_db_path())
        .await
        .expect("thread history database should open");
    THREAD_HISTORY_MIGRATOR
        .run(&history_pool)
        .await
        .expect("thread history baseline should apply");
    sqlx::query(
        "INSERT INTO thread_items (thread_id, turn_id, item_id, rollout_ordinal, created_at_ms, item_json, item_type) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind("thread-1")
    .bind("turn-1")
    .bind("item-1")
    .bind(1_i64)
    .bind(1_i64)
    .bind("{}")
    .bind("userMessage")
    .execute(&history_pool)
    .await
    .expect("thread item should insert");
    let updated_at_ordinal = sqlx::query_scalar::<_, i64>(
        "SELECT updated_at_ordinal FROM thread_items WHERE item_id = ?",
    )
    .bind("item-1")
    .fetch_one(&history_pool)
    .await
    .expect("thread item update ordinal should load");
    assert_eq!(updated_at_ordinal, 0);
    history_pool.close().await;
}
