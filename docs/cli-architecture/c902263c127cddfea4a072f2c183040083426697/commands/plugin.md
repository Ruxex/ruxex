# `codex plugin`

## Entry

Plugins are parsed by [`PluginCli` and `PluginSubcommand`](../../../../crates/cli/src/plugin_cmd.rs#L39). The root dispatch enters the plugin runner at [main.rs#L1083](../../../../crates/cli/src/main.rs#L1083).

Subcommands are `add`, `list`, `marketplace`, and `remove`. Marketplace has its own `add`, `list`, `upgrade`, and `remove` commands in [`marketplace_cmd.rs`](../../../../crates/cli/src/marketplace_cmd.rs#L35).

## Flow

```text
codex plugin <operation>
  -> plugin selector / marketplace arguments
  -> config and plugin input loading
  -> marketplace resolution
  -> install, list, upgrade, or remove
  -> human or JSON output
```

`add` resolves `PLUGIN@MARKETPLACE` or a plugin plus `--marketplace`, then installs into the local plugin/cache state. `list --available --json` can include marketplace plugins that are not installed. These operations mutate local plugin state but do not themselves start a model turn.

## Boundary

The command grammar is in `codex-cli`; plugin lifecycle and input/config semantics are in `codex-plugin` and app-server integration. See [configuration](../common/configuration.md), [transport/server](../common/transport-and-server.md), and [output/errors](../common/output-and-errors.md).
