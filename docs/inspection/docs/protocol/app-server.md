---
sidebar_position: 2
title: App Server Protocol
---

# App Server Protocol

App Server Protocolは、ClientがThread/Turnを操作し、Serverが進行状態とItemを通知する公開境界です。

## 概念的な流れ

```text
initialize
  -> thread/start or thread/resume
  -> turn/start
  -> turn/started notification
  -> item/started + item/delta + item/completed
  -> turn/completed or turn/aborted
```

Requestの受理やNotificationの配送はApp Serverが担当します。モデルの細かいStream eventをそのまま外へ漏らすのではなく、Thread Item lifecycleやTurn lifecycleへ変換します。

## APIの責務分離

- Thread API: 開始、再開、Fork、Archive、Delete、履歴取得
- Turn API: ユーザー入力、Steer、Interrupt、Queue
- Catalog/Auth/Usage API: 現在のRuntime Providerの状態
- Event Notification: item、tool、approval、error、turn lifecycle

## 主なソース

- [`crates/app-server-protocol/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/app-server-protocol/src/lib.rs)
- [`crates/app-server/src/message_processor.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/app-server/src/message_processor.rs)
- [`crates/app-server/src/request_processors/turn_processor.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/app-server/src/request_processors/turn_processor.rs)
