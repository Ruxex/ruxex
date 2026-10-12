---
sidebar_position: 3
title: ソースマップ
---

# ソースマップ

## チャット1回を追う

| 知りたいこと | 最初に読むファイル | 次に読むファイル |
| --- | --- | --- |
| Turnの全体ループ | `crates/core/src/session/turn.rs` | `session/mod.rs` |
| StepContextの中身 | `crates/core/src/session/step_context.rs` | `session/mod.rs` のContext capture |
| Prompt構築 | `crates/core/src/session/turn.rs` の `build_prompt` | `client_common.rs` |
| Streamイベント処理 | `crates/core/src/stream_events_utils.rs` | `turn.rs` の `run_sampling_request` |
| Tool実行 | `crates/core/src/tools/parallel.rs` | `tools/spec_plan.rs`、`function_tool.rs` |
| Compaction | `crates/core/src/compact.rs` | `compact_remote_v2.rs`、`compact_token_budget.rs` |
| Hook | `crates/core/src/hook_runtime.rs` | `session/turn.rs` |

## 起動とRuntimeを追う

| 知りたいこと | ファイル |
| --- | --- |
| CLI dispatch | `crates/cli/src/main.rs` |
| TUI startup | `crates/tui/src/lib.rs`、`startup_orchestration.rs` |
| App Server startup | `crates/app-server/src/main.rs`、`lib.rs` |
| Runtime/provider injection | `crates/app-server/src/runtime.rs` |
| Provider Home validation | `crates/model-provider/src/home_marker.rs` |
| Credential lifecycle | `crates/login/src/auth/manager.rs`、`change_state.rs` |

## 保存と再開を追う

| 知りたいこと | ファイル |
| --- | --- |
| DB名とOpen設定 | `crates/state/src/sqlite.rs` |
| DB初期化 | `crates/state/src/runtime.rs` |
| Schema | `crates/state/*_migrations/*.sql` |
| Rollout writer | `crates/rollout/src/recorder.rs` |
| Live write barrier | `crates/thread-store/src/local/live_writer.rs` |
| Thread read/resume | `crates/thread-store/src/local/read_thread.rs`、`thread_history.rs` |
| Rollout metadata extraction | `crates/state/src/extract.rs` |

## Protocolを追う

| 知りたいこと | ファイル |
| --- | --- |
| Shared domain event | `crates/protocol/src/protocol.rs`、`models.rs` |
| App Server wire types | `crates/app-server-protocol/src/lib.rs` |
| Request dispatch | `crates/app-server/src/message_processor.rs` |
| Turn request | `crates/app-server/src/request_processors/turn_processor.rs` |
| TUI event buffer | `crates/tui/src/app/thread_events.rs` |

## 検索の起点

```bash
rg -n "run_turn|capture_step_context|run_sampling_request" crates/core/src
rg -n "CREATE TABLE|ALTER TABLE|DROP TABLE" crates/state
rg -n "RolloutRecorder|materialize_to_sqlite|apply_rollout_item" crates/rollout crates/thread-store crates/state
rg -n "trait ModelProvider|ProviderCapabilities|ProviderRequestSnapshot" crates/model-provider
rg -n "turn/start|thread/start|turn/completed" crates/app-server crates/app-server-protocol
```
