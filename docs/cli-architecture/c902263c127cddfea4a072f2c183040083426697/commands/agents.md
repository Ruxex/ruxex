# `codex agents`

## エントリ

`codex agents` は `agents_overview = true` を設定して、通常と同じ対話 TUI バイナリを選択します。コマンドの形は [`AgentsCommand`](../../../../crates/cli/src/main.rs#L287) で、分岐は [main.rs#L981](../../../../crates/cli/src/main.rs#L981) の `None | Some(Subcommand::Agents(_))` アームにあります。

この概要画面は、共有ローカル app-server デーモン上のセッションを閲覧するためのものです。初期プロンプトや画像は拒否し、TUI に入る前にリモートエンドポイントの競合を検証します。

## フローと制約

```text
codex agents
  -> AgentsCommand のオプション
  -> エンドポイントとリモートモードの検証
  -> TuiCli.agents_overview = true
  -> run_interactive_tui
  -> デーモン全体のスレッド/セッション概要
```

- 概要画面には共有サーバーが必要で、`--no-daemon` は拒否されます。
- 対応プラットフォームでは、`--remote` がない場合にローカルデーモンのエンドポイントを解決します。
- ワークロード ID と一部の追加ディレクトリ上書きは、この経路と併用できません。
- 概要画面は UI モードであり、第二のスレッド実行エンジンではありません。

[トランスポートとサーバー](../common/transport-and-server.md)、[セッションとスレッド](../common/session-and-thread.md)、[対話モード](interactive.md)も参照してください。
