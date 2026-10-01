# `codex` の対話モード

## エントリ

サブコマンドを指定しない場合、`codex [OPTIONS] [PROMPT]` は対話経路に入ります。ルートパーサーは [`codex_tui::Cli`](../../../../crates/tui/src/cli.rs#L11) を組み込み、実際のディスパッチは [`cli_main`](../../../../crates/cli/src/main.rs#L930) が、TUI の起動ヘルパーは [`run_interactive_tui`](../../../../crates/cli/src/main.rs#L2195) が担当します。

TUI の引数オブジェクトには、ユーザープロンプト、strict-config、承認モード、ウェブ検索、代替画面、デーモン、共有設定、内部の resume/fork/agents 状態が含まれます。内部フィールドはトップレベルのラッパーコマンドが設定するもので、独立したユーザーフラグではありません。

## フロー

```text
codex [PROMPT]
  -> MultitoolCli + TuiCli
  -> ルートの設定/リモート/worktree 検証
  -> run_interactive_tui
  -> codex_tui の起動オーケストレーション
  -> app-server/直接セッション起動
  -> ターミナルのイベントループとターン
  -> AppExitInfo -> 終了/更新処理
```

TUI はモデルのターン、ツール、承認、永続化を共有ランタイムサービスに委譲します。[設定](../common/configuration.md)、[セッション/スレッド](../common/session-and-thread.md)、[ツール実行](../common/tool-execution.md)、[出力/エラー](../common/output-and-errors.md)も参照してください。

## 重要な制約

- TUI はターミナル機能を検査し、ターミナルでない stdin/stdout を拒否できます。
- `--no-daemon` は起動経路を変えますが、Core のセッションモデルを取り除くものではありません。
- `--ask-for-approval` と危険なバイパスフラグの併用関係は `codex-tui::Cli` が制約します。
- リモートモードと追加ディレクトリの上書きは、起動前にルート CLI で検証されます。

## 関連する担当範囲

| 関心事 | 定義 |
| --- | --- |
| ルートコマンド | `genex-cli::MultitoolCli`, `crates/cli/src/main.rs` |
| TUI 引数 | `codex-tui::Cli`, `crates/tui/src/cli.rs` |
| セッションランタイム | `genex-core`。TUI/app-server の起動経路から到達 |
| UI イベントソース | `codex-tui::EventSource`。[Trait 索引](../reference/trait-index.md#eventsource)を参照 |
