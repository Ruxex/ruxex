---
sidebar_position: 3
title: TUIイベント処理
---

# TUIイベント処理

TUIはチャットループを実装するのではなく、入力をTurnへ送り、Core/App Serverから返るイベントをThreadごとのイベントストアへ蓄積して描画します。

## 表示側の責務

- ComposerからUser Inputを作る
- Turn中のPending InputをQueueへ送る
- Item Deltaをバッファして表示する
- ApprovalやUser Input要求を画面に出す
- Reconnect後にイベントをReplayする
- Turn Completed / Error / Abortedを表示状態へ反映する

したがって、TUIの表示が遅れて見えても、CoreのModel Requestが遅いとは限りません。Thread Event Store、Transport、描画Tickを切り分けます。

## 主なソース

- [`crates/tui/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/tui/src/lib.rs)
- [`crates/tui/src/app/thread_events.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/tui/src/app/thread_events.rs)
- [`crates/tui/src/app/thread_routing.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/tui/src/app/thread_routing.rs)
