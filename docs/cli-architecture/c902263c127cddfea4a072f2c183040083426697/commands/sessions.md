# セッションコマンド

## 対象コマンド

`codex resume`、`codex fork`、`codex queue`、`codex archive`、`codex delete`、`codex unarchive` はすべて記録済みのスレッド/セッション状態を操作しますが、実行経路は同一ではありません。

## 対話的な resume/fork

`ResumeCommand` と `ForkCommand` は [`cli/src/main.rs`](../../../../crates/cli/src/main.rs#L304) で定義されています。ルートコマンドは `finalize_resume_interactive` と `finalize_fork_interactive` でそれらのフラグを `genex_tui::Cli` にまとめ、その後 `run_interactive_tui` を呼び出します。

- ID なし: ピッカーモード。
- `--last`: 条件に合う最新の記録済みセッション。
- 明示した ID/名前: セッションを直接選択。
- `--all`: 現在のディレクトリによる絞り込みを無効化。
- 任意のプロンプト/画像: resume/fork 後に新しいターンを送信。

Fork は新しいセッション関係を作成するもので、resume の単純な別名ではありません。[セッション/スレッド](../common/session-and-thread.md)も参照してください。

## archive/delete/unarchive

ルートのディスパッチは操作を `genex_tui::SessionArchiveAction` に変換し、[main.rs#L788](../../../../crates/cli/src/main.rs#L788) の `run_session_archive_cli_command` を呼び出します。操作は [`genex-tui::session_archive_commands`](../../../../crates/tui/src/session_archive_commands.rs#L41) に実装されています。`delete_action` が強制するように、`delete --force` が確認を迂回できるのは対象が UUID の場合だけです。

## queue

`queue` は [`run_queue_command`](../../../../crates/cli/src/queue_cmd.rs#L29) を使います。既存セッションを対象とし、現在のターミナルで対話 TUI を起動するのではなく、app-server のリクエスト経路を通じてメッセージを送ります。

## 状態のルール

- アーカイブ済みセッションは別のコレクションであり、resume/fork の前に unarchive の確認が必要になる場合があります。
- セッション検索には UUID または名前を使えます。コマンドで明記されている場合は UUID が優先されます。
- CWD による絞り込みと非対話セッションの包含はコマンドレベルの選択ルールであり、基礎となるスレッドモデルは共有されます。

[永続化とロールアウト](../common/persistence-and-rollout.md)、[トランスポートとサーバー](../common/transport-and-server.md)、[出力/エラー](../common/output-and-errors.md)も参照してください。
