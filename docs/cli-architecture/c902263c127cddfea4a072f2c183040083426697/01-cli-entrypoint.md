# CLI entrypoint

## Root parser

The executable is defined in [`crates/cli/src/main.rs`](../../../crates/cli/src/main.rs). `MultitoolCli` at [line 104](../../../crates/cli/src/main.rs#L104) combines:

- `CliConfigOverrides` and feature toggles;
- shared remote options;
- [`codex_tui::Cli`](../../../crates/tui/src/cli.rs#L11) for options that apply to the default interactive path;
- an optional [`Subcommand`](../../../crates/cli/src/main.rs#L122).

With no subcommand, the root parser forwards options to the interactive TUI. Clap is configured to negate default-argument requirements when a subcommand is present, so command-specific parsers can own their required values.

## Startup and normalization

`main` at [line 922](../../../crates/cli/src/main.rs#L922) initializes build information and uses `codex_arg0::arg0_dispatch_or_else` to support launcher/alternate-executable dispatch. `cli_main` at [line 930](../../../crates/cli/src/main.rs#L930) then:

1. parses the root command;
2. rejects removed or unsupported combinations;
3. folds `--enable` / `--disable` into config overrides;
4. resolves remote endpoint and auth-token environment selection;
5. applies `agents`, profile, strict-config, worktree, and remote-mode validation;
6. dispatches the selected command.

The validation helpers are grouped near [lines 1962–2150](../../../crates/cli/src/main.rs#L1962), rather than being implemented by each command. See [configuration](common/configuration.md), [transport](common/transport-and-server.md), and [feature flags](common/feature-flags.md).

## Dispatch map

| Root input | Dispatch target |
| --- | --- |
| no subcommand / `agents` | `run_interactive_tui` and `codex-tui` |
| `exec` | `codex_exec::run_main` |
| `review` | exec review command path |
| `mcp` | `McpCli::run` in `crates/cli/src/mcp_cmd.rs` |
| `plugin` | `PluginCli::run` in `crates/cli/src/plugin_cmd.rs` |
| `app-server` | `codex_app_server::run_main` or daemon/proxy/schema helpers |
| `resume`, `fork` | TUI wrapper state followed by `run_interactive_tui` |
| `queue`, `archive`, `delete`, `unarchive` | session/daemon helper paths |
| `login`, `logout` | `codex-login` auth manager helpers |
| `doctor` | `doctor::run_doctor` |
| `sandbox` | host-specific `codex-cli` sandbox runner |
| `debug`, `execpolicy`, `stdio-to-uds`, `exec-server`, `features` | utility-specific dispatch arms |

The match begins at [line 981](../../../crates/cli/src/main.rs#L981). The command enum and nested command shapes are the authoritative inventory; the [source map](reference/source-map.md) records the snapshot line anchors.

## Shared state passed through dispatch

The root dispatch carries the following values into multiple command paths:

- parsed config overrides and optional profile;
- remote endpoint and token-environment name;
- current working directory and worktree options;
- strict-config behavior;
- launcher executable paths;
- TUI shared options and feature toggle overrides.

This is why command pages link to common configuration and transport pages instead of treating each command as an isolated binary.
