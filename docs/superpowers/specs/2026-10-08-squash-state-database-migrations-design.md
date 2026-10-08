# State Database Migration Squash Design

## Goal

未リリースのため、`crates/state` が持つ5系列のSQLiteマイグレーション履歴を、各系列の現在の最終状態を表す単一の初期マイグレーションへ集約する。

対象は次の5系列とする。

- state DB
- logs DB
- goals DB
- queue DB
- thread history DB

既存の開発用DBは互換維持の対象外とし、変更後は再作成する。

## Context and constraints

- これは未リリースのGenex forkであり、旧バイナリや外部利用者向けの移行互換性は不要である。
- 現在のマイグレーターは5つの独立した`sqlx::Migrator`として登録されている。
- 旧マイグレーションのproducer/consumerを監査し、最終的な現在スキーマでコードが参照するテーブル、列、index、trigger、初期行は保持する。
- OpenAI endpoint固有の名前や第三者の外部契約は今回の変更対象ではない。
- 旧マイグレーションを単純に連結して残すのではなく、空のDBに対する最終スキーマを直接定義する。

## Proposed design

各ディレクトリを次の1ファイルへ置き換える。

| Series | New file | Version |
|---|---|---:|
| state | `crates/state/migrations/0001_initial.sql` | 1 |
| logs | `crates/state/logs_migrations/0001_initial.sql` | 1 |
| goals | `crates/state/goals_migrations/0001_initial.sql` | 1 |
| queue | `crates/state/queue_migrations/0001_initial.sql` | 1 |
| thread history | `crates/state/thread_history_migrations/0001_initial.sql` | 1 |

既存の61個のSQLファイルは削除する。`migrations.rs`の5つの`sqlx::migrate!`登録は、同じディレクトリを参照し続けるため変更しない。

### State DB baseline

最終状態として次を保持する。

- `threads`, `thread_dynamic_tools`, `thread_spawn_edges`, `thread_sections`, `projects`, `project_roots`, `project_idempotency_keys`, `thread_attachments`, `backfill_state`
- 現行コードが利用する全列、foreign key、CHECK制約、UNIQUE制約、default値
- 現行の全indexとpartial index
- timestamp/recencyの補完trigger
- 初期のPinned section行
- `backfill_state`の初期行（実行時刻は`strftime`で生成）

過去に存在したが現在は削除済みの`logs`、memory tables、agent job tables、device key binding、thread goalsはbaselineへ含めない。

### Auxiliary baselines

- logs: 現行の`feedback_log_body`列を持つ`logs`テーブルと現行index
- goals: `thread_goals`と`thread_goal_continuation_deferrals`
- queue: queue tables、unique index、revision trigger。過去データのbackfill INSERTは不要なため、空DBでは実行しない
- thread history: 最終列、page/user-message/update/end index

## Equivalence guarantee

「実行結果が同じ」は、旧履歴を空DBへ順番に適用した結果と、新しい`0001_initial.sql`を空DBへ適用した結果について、以下が一致することと定義する。

1. ユーザー定義のtable名とtable SQLの意味
2. 全列の順序、型、NULL制約、default値
3. primary key、UNIQUE、foreign key、CHECK制約
4. 全index名、列順、sort順、partial index条件
5. 全trigger名、対象イベント、WHEN条件、trigger body
6. 移行が意図的に作成する初期行（実行時刻など実行ごとに変わる値は、同じ生成式と値の範囲を比較）

確認では、旧SQLチェーンを一時的なin-memory SQLiteへ適用し、新baselineを別のin-memory SQLiteへ適用して、`sqlite_master`の正規化済み定義と対象初期行を比較する。`backfill_state.updated_at`のような動的値は、両方が`strftime('%s', 'now')`相当の現在時刻を生成し、比較時点の許容範囲に入ることを確認する。比較で差分が出た場合は、SQL削除・Rust変更・テスト更新を進めない。

さらに、実装後はstate crateのmigration testsで、全baselineが適用可能であること、主要な列/index/trigger/初期行が存在すること、runtimeがbaseline migratorを利用できることを検証する。

## Runtime and compatibility behavior

- 新しい開発DBは各系列のversion 1から開始する。
- 旧開発DBは自動変換しない。ユーザーのローカルDBを削除・再作成する必要がある。
- `runtime_migrator`の「DB側にある未認識の新しいversionを許容する」挙動は、今回の履歴圧縮とは独立した現在のruntime挙動として維持する。
- version 39専用の`repair_legacy_recency_migration_version`と、それだけを検証する履歴修復テストは、旧履歴がなくなるため削除する。
- 旧version 44/50/51などを再現する段階的互換テストは削除し、baselineの構造同値性と現行runtime契約のテストへ置き換える。

## Documentation and inventory

`docs/pre-release-compatibility-inventory.md`に、次をproducer/consumer監査付きで記録する。

- 旧61ファイルの適用履歴: `remove`（未リリースで既存DB互換不要）
- 5つの現行スキーマ: `migrate-in-place`
- 旧DBの自動変換: `remove`

## Verification

- 変更前に旧チェーンと新baselineをin-memory SQLiteで比較する。
- `git diff --check`
- `just check`
- Rustコード、migration testsを変更するため`just check --tests`
- checked-in generated artifactに影響があると判断した場合は`just generate`を実行し、差分を確認する。
- 実装完了前に、変更後の5系列がそれぞれ1ファイルだけであること、旧version番号へのproduction参照が残っていないことを確認する。

## Non-goals

- 外部利用者やリリース済みDBの互換性追加
- 現在のstate/logs/goals/queue/thread history APIの挙動変更
- provider固有機能の削除やGenex命名変更
- rolloutファイル、config、protocol schemaの別作業
