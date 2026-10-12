---
sidebar_position: 1
title: 読み方
---

# 読み方

## 推奨する順序

### 1. 全体の地図を作る

最初に [システムマップ](./system-map) と [用語集](./glossary) を読みます。ここで覚えるべきなのはクレート名の暗記ではなく、次の実行単位です。

```text
プロセス
  └─ AppServer / Runtime
       └─ Thread
            └─ Turn（ユーザーから見た1回の依頼）
                 └─ Step / Sampling（モデルへの1回の要求）
                      └─ Response Item / Tool Call
```

### 2. Runtimeの境界を理解する

次に [実行モデル](../runtime/execution-model) と [起動処理](../runtime/startup) を読みます。CLI、TUI、App Serverが別々のアプリケーションに見えても、最終的には同じCore、Provider、Thread Store、Rolloutへ接続します。

### 3. チャットの1回を追う

[チャットループ](../chat/chat-loop) を中心に、[コンテキスト収集](../chat/context-assembly)、[ストリーミング](../chat/streaming)、[Tool Loop](../chat/tool-loop) の順で読みます。ここがGenexの動作を理解する中心です。

### 4. 状態の保存先を追う

[保存領域全体図](../storage/storage-map) から始めて、[SQLite](../storage/sqlite)、[テーブル](../storage/tables)、[Rollout](../storage/rollouts) の順に読みます。「会話の正規記録」と「一覧・検索を速くするProjection」を分けて考えるのがポイントです。

### 5. 境界と例外を読む

最後にProvider、Tool/MCP、Protocol、認証、Sandbox、観測性を読めば、通常系だけでなく、ログアウト、キャンセル、DB障害、再接続時の振る舞いも説明できるようになります。

## 読み方のルール

各ページでは、次の順番で確認してください。

1. その部品が所有する責務
2. 呼び出されるタイミング
3. 入力と出力
4. メモリ上だけの状態か、永続化される状態か
5. 他の部品との境界
6. 主なソースファイル

行番号は実装変更で動くため、行番号だけを正解だと思わず、ファイル内の型名・関数名も併せて検索します。

## 3つの重要な問い

このサイトで特に重視するのは、次の3つです。

- [チャットは何を繰り返しているのか](../chat/chat-loop)
- [DBは何個あり、何を保存しているのか](../storage/sqlite)
- [チャット前に何を掻き集めているのか](../chat/context-assembly)
