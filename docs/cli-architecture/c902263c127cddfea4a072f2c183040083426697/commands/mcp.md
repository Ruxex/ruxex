# `codex mcp`

## Entry

MCP management is parsed by [`McpCli` and `McpSubcommand`](../../../../crates/cli/src/mcp_cmd.rs#L56). The root dispatch calls `McpCli::run` at [main.rs#L1071](../../../../crates/cli/src/main.rs#L1071), which loads configuration and selects the operation.

Supported subcommands are `list`, `get`, `add`, `remove`, `login`, and `logout`.

## Operation paths

| Command | Behavior |
| --- | --- |
| `list` | Read configured server entries; optionally emit JSON. |
| `get NAME` | Read one configured server; optionally emit JSON. |
| `add NAME` | Persist a stdio launcher or streamable HTTP server configuration. |
| `remove NAME` | Remove the configured entry. |
| `login NAME` | Run MCP OAuth flow, optionally without opening a browser. |
| `logout NAME` | Delete stored MCP OAuth credentials. |

`add` requires exactly one transport family. Stdio uses a command plus environment pairs; streamable HTTP uses a URL and may use bearer-token or OAuth registration options. Configuration persistence is distinct from model/tool execution; a configured server becomes relevant to runtime MCP tool discovery later.

See [configuration](../common/configuration.md), [authentication](../common/authentication.md), [tool execution](../common/tool-execution.md), and [output/errors](../common/output-and-errors.md).

## Ownership

The command grammar and config editing are in `codex-cli`; MCP transport/client behavior is in the RMCP client/core MCP crates. The CLI does not define the MCP protocol itself.
