# セッションとスレッドのライフサイクル

## 用語

- スレッドは、スレッド ID で識別される永続化/会話の単位です。
- ターンは、スレッド内の 1 回のユーザー/モデル対話です。
- セッションは、スレッドの開始、再開、フォーク、キュー投入、表示を取り巻く CLI 向けのワークフローです。

`genex-cli` の CLI ラッパーがスレッド/セッション操作を選択し、`codex-tui`、`codex-exec`、`codex-app-server` がランタイムを調整します。

## ライフサイクル

```text
スレッドを選択/作成
  -> スレッド設定と永続化済みコンテキストを読み込む
  -> ターンを受理/開始
  -> キュー投入、誘導、中断、完了、または失敗
  -> ロールアウト/スレッドメタデータを永続化
  -> 後で resume/fork/archive/delete/unarchive
```

スレッドの永続化は [`ThreadStore`](../reference/trait-index.md#threadstore) が、キューに入ったメッセージは [`QueueStore`](../reference/trait-index.md#queuestore) が抽象化します。Core のタスク受理は内部の [`SessionTask`](../reference/trait-index.md#sessiontask) 契約を使います。

## コマンドの違い

- `resume` は既存のスレッドに再び入り、任意で新しいターンを開始します。
- `fork` は完了済みの元履歴から新しいスレッドを作成します。
- `queue` はサーバー経路を通じて既存セッションに作業を送ります。
- archive/unarchive はコレクション/状態の可視性を変更します。
- delete は保存済み状態を削除し、コマンド固有の確認ルールに従います。

[セッションコマンド](../commands/sessions.md) と [永続化](persistence-and-rollout.md)も参照してください。
