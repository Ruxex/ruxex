---
title: SQLiteテーブル
---

# SQLiteテーブル

## `state_5.sqlite`

| テーブル | 保存するもの |
| --- | --- |
| `threads` | Thread ID、Rollout path、時刻、タイトル、CWD、Git情報、Model、Reasoning、権限、Archive、Project、Sectionなど |
| `thread_dynamic_tools` | Threadに紐づく追加Toolの順序、名前、説明、Input Schema |
| `thread_spawn_edges` | 親子ThreadのSpawn関係と状態 |
| `backfill_state` | Rolloutからの初期Backfillの状態、Watermark、Lease情報 |
| `thread_sections` | Thread Sectionの名前・表示設定 |
| `projects` | Projectの名前、Metadata、表示順 |
| `project_roots` | Projectに属するWorkspace Root |
| `project_idempotency_keys` | Project作成の重複防止キー |
| `thread_attachments` | Threadに紐づくAttachment種別、Identity、Payload |

`threads` は現在のMigration適用後には `model_provider` を持ちません。初期Migrationに存在した列とIndexは、[`0002_remove_thread_model_provider.sql`](https://github.com/edge-m/codex-fork/blob/main/crates/state/migrations/0002_remove_thread_model_provider.sql) で削除されます。ThreadはProvider Routingを永続化しないという境界の具体例です。

## `logs_2.sqlite`

| テーブル | 保存するもの |
| --- | --- |
| `logs` | ID、時刻、Level、Target、ログ本文、Thread ID、Process UUID、推定サイズ |

ログはThreadが紐づく場合もありますが、会話履歴ではありません。検索・診断向けのOperational Dataです。

## `goals_1.sqlite`

| テーブル | 保存するもの |
| --- | --- |
| `thread_goals` | ThreadごとのGoal、Objective、Status、Token/Time使用量 |
| `thread_goal_continuation_deferrals` | Goal継続を保留したThread |

## `queue_1.sqlite`

| テーブル | 保存するもの |
| --- | --- |
| `queued_items` | Threadごとの保留入力PayloadとQueue順序 |
| `queued_thread_revisions` | ThreadごとのQueue変更Revision |

Queueはモデル履歴そのものではありません。Turn中に入力を受け付けるための耐久化されたMailboxです。

## `thread_history_1.sqlite`

| テーブル | 保存するもの |
| --- | --- |
| `thread_turns` | Thread/Turn、状態、開始終了時刻、Rollout byte offset |
| `thread_items` | Thread/Turn/Item、Rollout ordinal、JSON、Item type |
| `thread_history_projection_state` | 次にProjectionするRollout位置 |

このDBはRollout JSONLをページング可能な履歴へMaterializeしたものです。Rolloutより速く読めますが、Rolloutより先に進んではいけません。

## テーブルを読むときの境界

DBテーブルは、すべてのモデル入力を一行ずつ保存する汎用ログではありません。

- Promptの最終形は、Thread履歴、StepContext、Tool仕様、Base InstructionsからSampling直前に作られる
- Thread一覧に必要な値はState DBへ抽出される
- 会話のイベントとResponse ItemはRolloutが基準になる
- Logs、Queue、Goalsは別のライフサイクルを持つ

## 主なソース

- [`crates/state/migrations/0001_initial.sql`](https://github.com/edge-m/codex-fork/blob/main/crates/state/migrations/0001_initial.sql)
- [`crates/state/logs_migrations/0001_initial.sql`](https://github.com/edge-m/codex-fork/blob/main/crates/state/logs_migrations/0001_initial.sql)
- [`crates/state/goals_migrations/0001_initial.sql`](https://github.com/edge-m/codex-fork/blob/main/crates/state/goals_migrations/0001_initial.sql)
- [`crates/state/queue_migrations/0001_initial.sql`](https://github.com/edge-m/codex-fork/blob/main/crates/state/queue_migrations/0001_initial.sql)
- [`crates/state/thread_history_migrations/0001_initial.sql`](https://github.com/edge-m/codex-fork/blob/main/crates/state/thread_history_migrations/0001_initial.sql)
