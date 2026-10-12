---
title: SQLite
---

# SQLite

## DBはいくつあるか

`SqliteConfig` はRuntime DBとして5つのパスを定義します。

| ファイル | DB種別 | 主な用途 | 初期化タイミング |
| --- | --- | --- | --- |
| `state_5.sqlite` | State | Thread、Project、Section、Attachment、関係 | `StateRuntime::init` |
| `logs_2.sqlite` | Logs | ログ行と診断情報 | `StateRuntime::init` |
| `goals_1.sqlite` | Goals | Thread Goalと使用量 | `StateRuntime::init` |
| `queue_1.sqlite` | Queue | 保留ユーザー入力、Queue Revision | `StateRuntime::init` |
| `thread_history_1.sqlite` | Thread History | Turn/ItemのページングProjection | Thread History所有者が必要時に開く |

「5 DB」と答えるのが正確ですが、通常の `StateRuntime::init` が最初に開くのはState、Logs、Goals、Queueの4つです。Thread Historyは [`open_thread_history_db`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/runtime.rs) を通じて、履歴Projectionの所有者が別に開きます。

## 起動時の処理

[`StateRuntime::init`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/runtime.rs) は概ね次の処理をします。

1. SQLite Homeディレクトリを作る
2. State DBを開いてMigrationする
3. Logs DBを開いてMigrationする
4. Goals DBを開いてMigrationする
5. Queue DBを開いてMigrationする
6. Backfill Stateの初期行を保証する
7. Threadの更新時刻・Recency時刻を読み込む
8. どこかで失敗したら、すでに開いたPoolを閉じて失敗を返す

各DBはWAL、Incremental Auto Vacuum、Busy Timeoutを使う共通のSQLite接続Shimから開かれます。Migration失敗時にはPoolを閉じるため、部分的に開いたRuntimeを返さない設計です。

## DB間の分離理由

- Logsを分離して、診断ログがThread更新のLock競合を増やさないようにする
- Queueを分離して、Turn中の入力受付をThread一覧更新から切り離す
- Thread Historyを分離して、履歴ページングとThread一覧の更新を分ける
- Goalsを分離して、Goalの利用量・状態更新を専用Storeにする

## 主なソース

- [`crates/state/src/sqlite.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/sqlite.rs)
- [`crates/state/src/runtime.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/runtime.rs)
- [`crates/state/src/telemetry.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/telemetry.rs)
