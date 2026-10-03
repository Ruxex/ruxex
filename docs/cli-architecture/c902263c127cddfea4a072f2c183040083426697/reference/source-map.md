# ソースマップ

以下の参照はすべてコミット `c902263c127cddfea4a072f2c183040083426697` を説明するものです。

| 機能 | CLI の定義/ディスパッチ | 主な下流の担当 |
| --- | --- | --- |
| ルートパーサー | [`MultitoolCli` と `Subcommand`](../../../../crates/cli/src/main.rs#L104)、[L930](../../../../crates/cli/src/main.rs#L930) でディスパッチ | `genex-cli` |
| 対話 TUI | [`None | Agents`](../../../../crates/cli/src/main.rs#L981) でディスパッチ | `genex-tui`、`genex-core` |
| `exec` | [`ExecCli` のフィールドとディスパッチ](../../../../crates/cli/src/main.rs#L1034) | `genex-exec` |
| `review` | [`ReviewCommand` とディスパッチ](../../../../crates/cli/src/main.rs#L266) | `genex-exec`、`genex-core` |
| セッションコマンド | [`ResumeCommand`～`ForkCommand`](../../../../crates/cli/src/main.rs#L304) と [L1284](../../../../crates/cli/src/main.rs#L1284) でのディスパッチ | `genex-tui`、`genex-app-server`、`genex-thread-store` |
| MCP | [`McpCli`](../../../../crates/cli/src/mcp_cmd.rs#L56)、[L223](../../../../crates/cli/src/mcp_cmd.rs#L223) で実行 | `genex-cli`、MCP/Core クレート |
| プラグイン | [`PluginCli`](../../../../crates/cli/src/plugin_cmd.rs#L39) | `genex-cli`、`genex-plugin`、app-server |
| App-server | [`AppServerCommand`](../../../../crates/cli/src/main.rs#L473)、[L1119](../../../../crates/cli/src/main.rs#L1119) でディスパッチ | `genex-app-server`、デーモン、プロトコル |
| 認証 | [`LoginCommand` / `LogoutCommand`](../../../../crates/cli/src/main.rs#L452)、[L1392](../../../../crates/cli/src/main.rs#L1392) でディスパッチ | `genex-login` |
| 診断 | [`DoctorCommand` のディスパッチ](../../../../crates/cli/src/main.rs#L1428) | `genex-cli` の doctor モジュール、`genex-api`、Core 設定 |
| サンドボックス | [`HostSandboxArgs` とディスパッチ](../../../../crates/cli/src/main.rs#L414) | `genex-cli`、`genex-sandboxing`、プラットフォーム用クレート |
| デバッグ | [`DebugSubcommand`](../../../../crates/cli/src/main.rs#L210)、[L1498](../../../../crates/cli/src/main.rs#L1498) でディスパッチ | `genex-cli`、モデル/トレース/Core ヘルパー |
| 実行ポリシー | [`ExecpolicySubcommand`](../../../../crates/cli/src/main.rs#L439)、[L1546](../../../../crates/cli/src/main.rs#L1546) でディスパッチ | `genex-execpolicy`、Core ポリシー |
| ユーティリティ | [L1420～1617](../../../../crates/cli/src/main.rs#L1420) の completion/features/exec-server/内部トランスポート | コマンド固有のクレート |

## ネストしたコマンド一覧

[`main.rs#L122`](../../../../crates/cli/src/main.rs#L122) のルート enum には、`Agents`、非表示の `TcpTunnel`、`Exec`、`Review`、`Login`、`Logout`、`Mcp`、`Plugin`、`AppServer`、`Completion`、`Doctor`、`Sandbox`、`Debug`、非表示の `Execpolicy`、`Resume`、`Queue`、`Archive`、`Delete`、`Unarchive`、`Fork`、非表示の `StdioToUds`、`ExecServer`、`Features` が含まれます。

ネストした一覧は次で定義されています。

- [`main.rs#L210`](../../../../crates/cli/src/main.rs#L210) の `DebugSubcommand`。
- [`main.rs#L551`](../../../../crates/cli/src/main.rs#L551) の `AppServerSubcommand` と `AppServerDaemonSubcommand`。
- [`mcp_cmd.rs#L65`](../../../../crates/cli/src/mcp_cmd.rs#L65) の `McpSubcommand`。
- [`plugin_cmd.rs#L48`](../../../../crates/cli/src/plugin_cmd.rs#L48) の `PluginSubcommand`。
- [`marketplace_cmd.rs#L44`](../../../../crates/cli/src/marketplace_cmd.rs#L44) の `MarketplaceSubcommand`。
- [`exec/cli.rs#L149`](../../../../crates/exec/src/cli.rs#L149) の `genex exec` 用 `Command`。
