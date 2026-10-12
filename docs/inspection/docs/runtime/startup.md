---
sidebar_position: 2
title: 起動処理
---

# 起動処理

起動時は、ユーザー入力を受け付ける前に「どの設定で」「どのProviderを」「どのデータルートから」動かすかを確定します。Provider HomeのMarker検証は、SQLiteやCredentialを開く前の安全境界です。

## 起動の概念的な順序

```text
argv
  -> CLI解析・コマンド選択
  -> Configの解決
  -> Provider Descriptorの決定
  -> Provider Home Markerの検証・作成
  -> Credential Managerの読み込み
  -> 1つのModelProviderを構築
  -> AppServer Runtimeを構築
  -> SQLite / Thread Store / Rolloutの初期化
  -> ClientまたはTUIの接続
```

Provider Homeが別ProviderのMarkerを持っている場合、既存のDBや認証情報を開かずに起動を拒否します。この順序により、Provider AのデータルートをProvider Bで誤って解釈することを防ぎます。

## CLIの役割

[`crates/cli/src/main.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/cli/src/main.rs) は次を行います。

1. ルート引数を解析する
2. 設定上書き、Profile、Remote、Worktreeなどを検証する
3. サブコマンドをTUI、Exec、App Server、Loginなどへディスパッチする

CLIは「チャットの各Sampling」を実装しません。対話経路ならTUI、サーバー経路ならApp Serverへ制御を渡します。

## TUIとApp Server

- TUIは入力受付、表示、イベントのバッファリングを担当します。
- App ServerはProtocol Request、Thread lifecycle、Turn admission、Client通知を担当します。
- in-process経路では同じプロセス内でCoreを呼びます。
- daemon経路ではClientとServerの間をJSON-RPC/Transportが分けます。

この違いを理解すると、TUIの表示コードを読んでもチャットループが見つからない理由が分かります。実際のTurn処理はCore側にあります。

## 主なソース

- [`crates/cli/src/main.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/cli/src/main.rs)
- [`crates/tui/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/tui/src/lib.rs)
- [`crates/app-server/src/main.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/app-server/src/main.rs)
- [`crates/app-server/src/runtime.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/app-server/src/runtime.rs)
- [`crates/genex-home/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/genex-home/src/lib.rs)
