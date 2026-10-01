# `codex exec`

## エントリ

`codex exec [OPTIONS] [PROMPT]` は [`codex_exec::Cli`](../../../../crates/exec/src/cli.rs#L15) で解析され、`genex-cli` により [main.rs#L1034](../../../../crates/cli/src/main.rs#L1034) でディスパッチされます。ネストしたコマンドがない場合は新しい非対話セッションを実行し、[`Command`](../../../../crates/exec/src/cli.rs#L149) を通じてネストした `resume`、`fork`、`review` 操作にも対応します。

## 入出力モデル

exec パーサーは次のフラグを管理します。

- プロンプト/stdin 入力。
- JSONL イベント出力と最終メッセージのファイル出力。
- 最終応答の出力スキーマ。
- 一時実行と設定/ルール読み込みの制御。
- グローバルモデル、承認のバイパス、フックの信頼、worktree のオプション。

`codex_exec::run_main` は [`exec/src/lib.rs#L244`](../../../../crates/exec/src/lib.rs#L244) にあるランタイムのエントリです。設定を読み込み、app-server/セッション経路を解決または起動し、リクエストを送信し、イベントを処理して、完了/エラー状態をプロセス出力に変換します。

## フロー

```text
codex exec
  -> codex_exec::Cli
  -> codex_exec::run_main
  -> 設定/ブートストラップ
  -> スレッドを開始/再開/フォーク
  -> app-server/Core 経由でプロンプトを送信
  -> イベント処理
  -> テキスト、JSONL、スキーマ形式の出力、または終了コード
```

`--ephemeral` は永続化方法の選択であり、別のモデルプロトコルではありません。[設定](../common/configuration.md)、[セッション/スレッド](../common/session-and-thread.md)、[出力/エラー](../common/output-and-errors.md)、[永続化](../common/persistence-and-rollout.md)も参照してください。

## 関連する型

| 型 | クレート | 役割 |
| --- | --- | --- |
| `Cli`、`Command` | `codex-exec` | コマンド文法 |
| `EventProcessor` | `codex-exec` | 内部のイベントから出力への処理 |
| `ResponseEvent` | `genex-api` | モデル応答ストリームの境界 |
| `ThreadId`、`ClientRequest` | `genex-protocol` / app-server protocol | リクエスト/セッションの識別情報 |
