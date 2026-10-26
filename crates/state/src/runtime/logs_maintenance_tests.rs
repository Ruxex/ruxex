//! Exercise age/size pruning and automatic cleanup of an idle runtime.

use super::super::test_support::unique_temp_dir;
use super::StateRuntime;
use super::prune_by_age_and_size;
use crate::SqliteConfig;
use crate::migrations::LOGS_MIGRATOR;
use chrono::Utc;
use ruxex_utils_absolute_path::test_support::PathExt;
use sqlx::SqlitePool;
use std::time::Duration;

const LOG_RETENTION_SECONDS: i64 = 10 * 24 * 60 * 60;
const NOW: i64 = 2_000_000_000;
const DAY: i64 = 24 * 60 * 60;

async fn database_with_logs(ages: &[i64]) -> (SqlitePool, std::path::PathBuf) {
    let directory = unique_temp_dir();
    tokio::fs::create_dir_all(&directory).await.unwrap();
    let pool = SqliteConfig::new_for_testing(directory.as_path().abs())
        .open_read_write_pool(&directory.join("logs.sqlite"))
        .await
        .unwrap();
    LOGS_MIGRATOR.run(&pool).await.unwrap();
    for age in ages {
        sqlx::query(
            "INSERT INTO logs (ts, ts_nanos, level, target, feedback_log_body) \
             VALUES (?, 0, 'INFO', 'retention-test', ?)",
        )
        .bind(NOW - age)
        .bind("x".repeat(128 * 1024))
        .execute(&pool)
        .await
        .unwrap();
    }
    (pool, directory)
}

async fn retained_ages(pool: &SqlitePool) -> Vec<i64> {
    sqlx::query_scalar("SELECT ? - ts FROM logs ORDER BY ts DESC")
        .bind(NOW)
        .fetch_all(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn keeps_ten_day_boundary_when_under_budget() {
    let (pool, directory) =
        database_with_logs(&[0, LOG_RETENTION_SECONDS, LOG_RETENTION_SECONDS + 1]).await;
    prune_by_age_and_size(&pool, NOW, /*budget_bytes*/ 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(retained_ages(&pool).await, vec![0, LOG_RETENTION_SECONDS]);
    pool.close().await;
    tokio::fs::remove_dir_all(directory).await.unwrap();
}

#[tokio::test]
async fn periodic_cleanup_prunes_without_new_log_writes_or_restart() {
    let directory = unique_temp_dir();
    let runtime = StateRuntime::init(SqliteConfig::new_for_testing(directory.as_path().abs()))
        .await
        .unwrap();
    let now = Utc::now().timestamp();
    for timestamp in [now - 11 * DAY, now] {
        sqlx::query("INSERT INTO logs (ts, ts_nanos, level, target) VALUES (?, 0, 'INFO', 'test')")
            .bind(timestamp)
            .execute(runtime.logs_pool.as_ref())
            .await
            .unwrap();
    }
    runtime.start_periodic_logs_maintenance(Duration::from_millis(10));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let timestamps: Vec<i64> = sqlx::query_scalar("SELECT ts FROM logs ORDER BY ts")
                .fetch_all(runtime.logs_pool.as_ref())
                .await
                .unwrap();
            if timestamps == vec![now] {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("periodic cleanup should remove expired logs while idle");
    runtime.close().await;
    tokio::fs::remove_dir_all(directory).await.unwrap();
}
