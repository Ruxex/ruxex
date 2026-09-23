# `codex mcp`

## エントリ

MCP の管理操作は [`McpCli` と `McpSubcommand`](../../../../crates/cli/src/mcp_cmd.rs#L56) で解析されます。ルートのディスパッチは [main.rs#L1071](../../../../crates/cli/src/main.rs#L1071) で `McpCli::run` を呼び出し、設定を読み込んで操作を選択します。

対応するサブコマンドは `list`、`get`、`add`、`remove`、`login`、`logout` です。

## 操作経路

| コマンド | 動作 |
| --- | --- |
| `list` | 設定済みサーバーのエントリを読み込み、任意で JSON を出力します。 |
| `get NAME` | 指定した設定済みサーバーを読み込み、任意で JSON を出力します。 |
| `add NAME` | stdio ランチャーまたはストリーム可能な HTTP サーバーの設定を永続化します。 |
| `remove NAME` | 設定済みエントリを削除します。 |
| `login NAME` | MCP OAuth フローを実行します。ブラウザーを開かないこともできます。 |
| `logout NAME` | 保存済み MCP OAuth 資格情報を削除します。 |

`add` にはトランスポート種別がちょうど 1 つ必要です。Stdio はコマンドと環境変数の組を使い、ストリーム可能な HTTP は URL を使い、ベアラートークンまたは OAuth 登録オプションも利用できます。設定の永続化はモデル/ツール実行とは別であり、設定済みサーバーは後のランタイム MCP ツール検出で利用されます。

[設定](../common/configuration.md)、[認証](../common/authentication.md)、[ツール実行](../common/tool-execution.md)、[出力/エラー](../common/output-and-errors.md)も参照してください。

## 担当範囲

コマンド文法と設定編集は `codex-cli` にあり、MCP のトランスポート/クライアント動作は RMCP クライアント/Core MCP クレートにあります。CLI 自体は MCP プロトコルを定義しません。
