use super::QuickCheckOutcome;
use super::SqliteQuickCheckManager;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path;
use std::time::Duration;

#[tokio::test]
async fn quick_check_runs_once_per_database_path() -> anyhow::Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    let manager = SqliteQuickCheckManager::new();
    let path = Path::new("/tmp/ruxex-validation.sqlite");

    assert_eq!(
        manager
            .quick_check_once(&pool, path, Duration::from_secs(1))
            .await?,
        QuickCheckOutcome::Complete
    );
    assert_eq!(
        manager
            .quick_check_once(&pool, path, Duration::from_secs(1))
            .await?,
        QuickCheckOutcome::Skipped
    );
    Ok(())
}
