# ソースマップ

以下の参照はすべてコミット `c902263c127cddfea4a072f2c183040083426697` を説明するものです。

| 機能 | CLI の定義/ディスパッチ | 主な下流の担当 |
| --- | --- | --- |
| ルートパーサー | [`MultitoolCli` と `Subcommand`](../../../../crates/cli/src/main.rs#L104)、[L930](../../../../crates/cli/src/main.rs#L930) でディスパッチ | `codex-cli` |
| 対話 TUI | [`None | Agents`](../../../../crates/cli/src/main.rs#L981) でディスパッチ | `codex-tui`、`codex-core` |
| `exec` | [`ExecCli` のフィールドとディスパッチ](../../../../crates/cli/src/main.rs#L1034) | `codex-exec` |
| `review` | [`ReviewCommand` とディスパッチ](../../../../crates/cli/src/main.rs#L266) | `codex-exec`、`codex-core` |
| セッションコマンド | [`ResumeCommand`～`ForkCommand`](../../../../crates/cli/src/main.rs#L304) と [L1284](../../../../crates/cli/src/main.rs#L1284) でのディスパッチ | `codex-tui`、`codex-app-server`、`codex-thread-store` |
| MCP | [`McpCli`](../../../../crates/cli/src/mcp_cmd.rs#L56)、[L223](../../../../crates/cli/src/mcp_cmd.rs#L223) で実行 | `codex-cli`、MCP/Core クレート |
| プラグイン | [`PluginCli`](../../../../crates/cli/src/plugin_cmd.rs#L39) | `codex-cli`、`codex-plugin`、app-server |
| App-server | [`AppServerCommand`](../../../../crates/cli/src/main.rs#L473)、[L1119](../../../../crates/cli/src/main.rs#L1119) でディスパッチ | `codex-app-server`、デーモン、プロトコル |
| 認証 | [`LoginCommand` / `LogoutCommand`](../../../../crates/cli/src/main.rs#L452)、[L1392](../../../../crates/cli/src/main.rs#L1392) でディスパッチ | `codex-login` |
| 診断 | [`DoctorCommand` のディスパッチ](../../../../crates/cli/src/main.rs#L1428) | `codex-cli` の doctor モジュール、`codex-api`、Core 設定 |
| サンドボックス | [`HostSandboxArgs` とディスパッチ](../../../../crates/cli/src/main.rs#L414) | `codex-cli`、`codex-sandboxing`、プラットフォーム用クレート |
| デバッグ | [`DebugSubcommand`](../../../../crates/cli/src/main.rs#L210)、[L1498](../../../../crates/cli/src/main.rs#L1498) でディスパッチ | `codex-cli`、モデル/トレース/Core ヘルパー |
| 実行ポリシー | [`ExecpolicySubcommand`](../../../../crates/cli/src/main.rs#L439)、[L1546](../../../../crates/cli/src/main.rs#L1546) でディスパッチ | `codex-execpolicy`、Core ポリシー |
| ユーティリティ | [L1420～1617](../../../../crates/cli/src/main.rs#L1420) の completion/features/exec-server/内部トランスポート | コマンド固有のクレート |

## ネストしたコマンド一覧

[`main.rs#L122`](../../../../crates/cli/src/main.rs#L122) のルート enum には、`Agents`、非表示の `TcpTunnel`、`Exec`、`Review`、`Login`、`Logout`、`Mcp`、`Plugin`、`AppServer`、`Completion`、`Doctor`、`Sandbox`、`Debug`、非表示の `Execpolicy`、`Resume`、`Queue`、`Archive`、`Delete`、`Unarchive`、`Fork`、非表示の `StdioToUds`、`ExecServer`、`Features` が含まれます。

ネストした一覧は次で定義されています。

- [`main.rs#L210`](../../../../crates/cli/src/main.rs#L210) の `DebugSubcommand`。
- [`main.rs#L551`](../../../../crates/cli/src/main.rs#L551) の `AppServerSubcommand` と `AppServerDaemonSubcommand`。
- [`mcp_cmd.rs#L65`](../../../../crates/cli/src/mcp_cmd.rs#L65) の `McpSubcommand`。
- [`plugin_cmd.rs#L48`](../../../../crates/cli/src/plugin_cmd.rs#L48) の `PluginSubcommand`。
- [`marketplace_cmd.rs#L44`](../../../../crates/cli/src/marketplace_cmd.rs#L44) の `MarketplaceSubcommand`。
- [`exec/cli.rs#L149`](../../../../crates/exec/src/cli.rs#L149) の `codex exec` 用 `Command`。
