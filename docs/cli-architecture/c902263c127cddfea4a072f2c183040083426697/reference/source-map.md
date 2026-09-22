# Source map

All references below describe commit `c902263c127cddfea4a072f2c183040083426697`.

| Capability | CLI definition / dispatch | Main downstream owner |
| --- | --- | --- |
| Root parser | [`MultitoolCli` and `Subcommand`](../../../../crates/cli/src/main.rs#L104), dispatch at [L930](../../../../crates/cli/src/main.rs#L930) | `codex-cli` |
| Interactive TUI | dispatch at [`None | Agents`](../../../../crates/cli/src/main.rs#L981) | `codex-tui`, `codex-core` |
| `exec` | [`ExecCli` field and dispatch](../../../../crates/cli/src/main.rs#L1034) | `codex-exec` |
| `review` | [`ReviewCommand` and dispatch](../../../../crates/cli/src/main.rs#L266) | `codex-exec`, `codex-core` |
| Session commands | [`ResumeCommand`–`ForkCommand`](../../../../crates/cli/src/main.rs#L304) and dispatch at [L1284](../../../../crates/cli/src/main.rs#L1284) | `codex-tui`, `codex-app-server`, `codex-thread-store` |
| MCP | [`McpCli`](../../../../crates/cli/src/mcp_cmd.rs#L56), run at [L223](../../../../crates/cli/src/mcp_cmd.rs#L223) | `codex-cli`, MCP/core crates |
| Plugins | [`PluginCli`](../../../../crates/cli/src/plugin_cmd.rs#L39) | `codex-cli`, `codex-plugin`, app-server |
| App-server | [`AppServerCommand`](../../../../crates/cli/src/main.rs#L473), dispatch at [L1119](../../../../crates/cli/src/main.rs#L1119) | `codex-app-server`, daemon, protocol |
| Auth | [`LoginCommand` / `LogoutCommand`](../../../../crates/cli/src/main.rs#L452), dispatch at [L1392](../../../../crates/cli/src/main.rs#L1392) | `codex-login` |
| Diagnostics | [`DoctorCommand` dispatch](../../../../crates/cli/src/main.rs#L1428) | `codex-cli` doctor modules, `codex-api`, core config |
| Sandbox | [`HostSandboxArgs` and dispatch](../../../../crates/cli/src/main.rs#L414) | `codex-cli`, `codex-sandboxing`, platform crates |
| Debug | [`DebugSubcommand`](../../../../crates/cli/src/main.rs#L210), dispatch at [L1498](../../../../crates/cli/src/main.rs#L1498) | `codex-cli`, models/trace/core helpers |
| Exec policy | [`ExecpolicySubcommand`](../../../../crates/cli/src/main.rs#L439), dispatch at [L1546](../../../../crates/cli/src/main.rs#L1546) | `codex-execpolicy`, core policy |
| Utilities | completion/features/exec-server/internal transports at [L1420–1617](../../../../crates/cli/src/main.rs#L1420) | command-specific crates |

## Nested command inventory

The root enum at [`main.rs#L122`](../../../../crates/cli/src/main.rs#L122) contains `Agents`, hidden `TcpTunnel`, `Exec`, `Review`, `Login`, `Logout`, `Mcp`, `Plugin`, `AppServer`, `Completion`, `Doctor`, `Sandbox`, `Debug`, hidden `Execpolicy`, `Resume`, `Queue`, `Archive`, `Delete`, `Unarchive`, `Fork`, hidden `StdioToUds`, `ExecServer`, and `Features`.

Nested inventories are defined by:

- `DebugSubcommand` at [`main.rs#L210`](../../../../crates/cli/src/main.rs#L210);
- `AppServerSubcommand` and `AppServerDaemonSubcommand` at [`main.rs#L551`](../../../../crates/cli/src/main.rs#L551);
- `McpSubcommand` at [`mcp_cmd.rs#L65`](../../../../crates/cli/src/mcp_cmd.rs#L65);
- `PluginSubcommand` at [`plugin_cmd.rs#L48`](../../../../crates/cli/src/plugin_cmd.rs#L48);
- `MarketplaceSubcommand` at [`marketplace_cmd.rs#L44`](../../../../crates/cli/src/marketplace_cmd.rs#L44);
- `Command` for `codex exec` at [`exec/cli.rs#L149`](../../../../crates/exec/src/cli.rs#L149).
