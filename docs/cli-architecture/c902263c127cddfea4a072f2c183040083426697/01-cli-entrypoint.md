# CLI エントリポイント

## ルートパーサー

実行ファイルは [`crates/cli/src/main.rs`](../../../crates/cli/src/main.rs) で定義されています。[104 行目](../../../crates/cli/src/main.rs#L104) の `MultitoolCli` は、次の要素を組み合わせます。

- `CliConfigOverrides` と機能トグル。
- 共有リモートオプション。
- 既定の対話経路に適用するオプションを持つ [`codex_tui::Cli`](../../../crates/tui/src/cli.rs#L11)。
- 任意の [`Subcommand`](../../../crates/cli/src/main.rs#L122)。

サブコマンドがない場合、ルートパーサーはオプションを対話 TUI に渡します。Clap はサブコマンドが存在するときに既定引数の必須性を無効化するよう設定されているため、各コマンドのパーサーが必要な値を管理できます。

## 起動と正規化

`main`（[922 行目](../../../crates/cli/src/main.rs#L922)）はビルド情報を初期化し、`codex_arg0::arg0_dispatch_or_else` を使ってランチャー/代替実行ファイルのディスパッチをサポートします。続いて `cli_main`（[930 行目](../../../crates/cli/src/main.rs#L930)）は次を行います。

1. ルートコマンドを解析する。
2. 削除済みまたはサポートされない組み合わせを拒否する。
3. `--enable` / `--disable` を設定の上書きにまとめる。
4. リモートエンドポイントと認証トークンの環境変数選択を解決する。
5. `agents`、プロファイル、strict-config、worktree、リモートモードを検証する。
6. 選択されたコマンドをディスパッチする。

検証ヘルパーは各コマンドに実装するのではなく、[1962～2150 行付近](../../../crates/cli/src/main.rs#L1962) にまとめられています。[設定](common/configuration.md)、[トランスポート](common/transport-and-server.md)、[機能フラグ](common/feature-flags.md)も参照してください。

## ディスパッチマップ

| ルート入力 | ディスパッチ先 |
| --- | --- |
| サブコマンドなし / `agents` | `run_interactive_tui` と `codex-tui` |
| `exec` | `genex_exec::run_main` |
| `review` | exec のレビューコマンド経路 |
| `mcp` | `McpCli::run` in `crates/cli/src/mcp_cmd.rs` |
| `plugin` | `PluginCli::run` in `crates/cli/src/plugin_cmd.rs` |
| `app-server` | `codex_app_server::run_main` またはデーモン/プロキシ/スキーマヘルパー |
| `resume`, `fork` | TUI wrapper state followed by `run_interactive_tui` |
| `queue`、`archive`、`delete`、`unarchive` | セッション/デーモンのヘルパー経路 |
| `login`、`logout` | `genex-login` の認証マネージャーヘルパー |
| `doctor` | `doctor::run_doctor` |
| `sandbox` | ホスト固有の `genex-cli` サンドボックスランナー |
| `debug`、`execpolicy`、`stdio-to-uds`、`exec-server`、`features` | ユーティリティ固有のディスパッチアーム |

match は [981 行目](../../../crates/cli/src/main.rs#L981) から始まります。コマンド enum とネストしたコマンドの形が正式な一覧であり、[ソースマップ](reference/source-map.md) にスナップショット時点の行番号アンカーを記録しています。

## ディスパッチで渡される共有状態

ルートのディスパッチは、次の値を複数のコマンド経路に渡します。

- 解析済みの設定上書きと任意のプロファイル。
- リモートエンドポイントとトークン環境変数名。
- 現在の作業ディレクトリと worktree オプション。
- strict-config の動作。
- ランチャー実行ファイルのパス。
- TUI の共有オプションと機能トグルの上書き。

そのため、各コマンドを独立したバイナリとして扱うのではなく、コマンドのページから共通の設定・トランスポートのページへリンクしています。
