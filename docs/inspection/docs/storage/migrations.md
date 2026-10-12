---
title: MigrationとBackfill
---

# MigrationとBackfill

## Migrationの単位

SQLiteは1つの巨大なMigration列ではなく、DB種別ごとに別のSQLx Migratorを持ちます。

| Migrator | ディレクトリ | 現在の主なテーブル |
| --- | --- | --- |
| State | `crates/state/migrations` | `threads`、Project、Section、Attachmentなど |
| Logs | `crates/state/logs_migrations` | `logs` |
| Goals | `crates/state/goals_migrations` | Goal tables |
| Queue | `crates/state/queue_migrations` | Queue tables |
| Thread History | `crates/state/thread_history_migrations` | Turn、Item、Projection state |

[`migrations.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/migrations.rs) は `sqlx::migrate!` で各ディレクトリを埋め込み、Runtime用に `ignore_missing` を設定したMigratorを作ります。既知のMigrationのChecksumは検証しつつ、並行する新しいBinaryが適用した将来Versionを古いBinaryが直ちに拒否しないための設定です。

## Provider選択の削除例

初期State Schemaの `threads` には `model_provider` とIndexがありました。現在は [`0002_remove_thread_model_provider.sql`](https://github.com/edge-m/codex-fork/blob/main/crates/state/migrations/0002_remove_thread_model_provider.sql) がIndexと列を削除します。

これは名前を消しただけではなく、Thread PersistenceからProvider Routingを取り除くMigrationです。ProviderはRuntime境界に残り、ThreadはProviderを選択しません。

## Rollout Backfill

既存のRollout JSONLからState DBへMetadataを取り込むBackfillがあります。

1. State DBの `backfill_state` を読む
2. 未完了ならLeaseを取得する
3. Rolloutを走査し、Metadataに影響するItemを抽出する
4. WatermarkをCheckpointする
5. 完了状態にする
6. 他プロセスが実行中なら待機し、タイムアウト時は起動失敗にする

Backfillは「DBを作る」処理ではなく、Canonical Rolloutと検索用Projectionの差を埋める処理です。

## 壊れたDBと再構築

State RuntimeはSQLite Integrity Check、Corruption Error判定、Fresh Start向けBackupを持ちます。Thread HistoryとState MetadataがRolloutから再構築可能であることが、障害復旧の前提です。

## 主なソース

- [`crates/state/src/migrations.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/migrations.rs)
- [`crates/state/src/runtime/backfill.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/runtime/backfill.rs)
- [`crates/rollout/src/state_db.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/rollout/src/state_db.rs)
- [`crates/state/src/runtime/recovery.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/runtime/recovery.rs)
