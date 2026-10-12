---
sidebar_position: 2
title: クレートマップ
---

# クレートマップ

すべてのクレートを同じ深さで読む必要はありません。まず次の責務の層を押さえます。

| 層 | クレート | 読む理由 |
| --- | --- | --- |
| Entry | `cli`、`tui`、`exec` | CLI引数、対話、非対話の入口 |
| Server | `app-server`、`app-server-protocol`、`app-server-transport` | Client Request、Notification、Transport |
| Agent Core | `core`、`core-api`、`core-plugins` | Thread、Turn、Context、Tool、Hook、Approval |
| Provider | `model-provider`、`model-provider-info`、`genex-api`、`genex-client` | Provider契約、adapter、外部Endpoint |
| Protocol | `protocol` | Thread/Turn Item、Event、共有ドメイン型 |
| Persistence | `rollout`、`thread-store`、`state`、`history` | JSONL、Thread lifecycle、SQLite、履歴再構築 |
| Model config | `models-manager`、`config`、`config-schema` | Model Catalog、設定解決、Schema |
| Capability | `tools`、`genex-mcp`、`skills`、`plugin` | Tool、MCP、Skill、Plugin |
| Security | `execpolicy`、`sandboxing`、`network-proxy`、`user-verification` | 実行許可、隔離、Approval |
| Host | `genex-home`、`login`、`genex-client` | Data Root、Credential、Client接続 |

## Coreを読む順番

1. `core/src/lib.rs` で公開境界を見る
2. `core/src/session/turn.rs` でTurnの制御フローを見る
3. `core/src/session/mod.rs` でSession StateとContext captureを見る
4. `core/src/session/step_context.rs` で1 RequestのSnapshotを見る
5. `core/src/stream_events_utils.rs` でProvider ItemをTool/UI/Historyへ分岐する処理を見る
6. `core/src/tools/*` でTool計画と実行を見る
7. `core/src/compact.rs` で履歴圧縮を見る

## Persistenceを読む順番

1. `state/src/sqlite.rs` でDBファイルと接続を確認する
2. `state/src/runtime.rs` でDBの初期化とStoreを確認する
3. `state/*_migrations/*.sql` で実際のテーブルを確認する
4. `rollout/src/recorder.rs` でCanonical JSONLの書き込みを見る
5. `thread-store/src/local/live_writer.rs` でRolloutとProjectionの順序を見る
6. `thread-store/src/local/thread_history_materialization.rs` で履歴Projectionを見る

## Providerを読む順番

1. `model-provider/src/contract.rs` でCoreが見る契約を確認する
2. `model-provider/src/provider.rs` でRuntime adapterを見る
3. `login/src/auth/manager.rs` でCredential lifecycleを見る
4. `app-server/src/runtime.rs` でProvider注入とHome mountを見る
5. `genex-api` / `genex-client` で外部Endpointとの通信を見る
