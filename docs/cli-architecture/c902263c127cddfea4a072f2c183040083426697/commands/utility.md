# Utility and internal commands

## `completion`

`codex completion [bash|zsh|fish|...]` generates shell completion scripts using the root clap command. It is local, stateless, and rejects remote mode. The parser is [`CompletionCommand`](../../../../crates/cli/src/main.rs#L197), dispatched at [main.rs#L1420](../../../../crates/cli/src/main.rs#L1420).

## `features`

`codex features list|enable|disable` reads or edits feature configuration. `list` resolves the effective config and prints feature key, stage, and enabled state; enable/disable edit the persistent config. The enum is near [`main.rs#L897`](../../../../crates/cli/src/main.rs#L897), and dispatch is at [L1575](../../../../crates/cli/src/main.rs#L1575). See [feature flags](../common/feature-flags.md).

## `exec-server`

The experimental `codex exec-server` service exposes execution over stdio, with request concurrency controlled by `RequestDispatchMode`. Its CLI wrapper is [`ExecServerCommand`](../../../../crates/cli/src/main.rs#L525); runtime ownership is in `codex-exec-server` and app-server execution support. See [transport/server](../common/transport-and-server.md) and [telemetry](../common/telemetry.md).

## Internal transport helpers

- `stdio-to-uds` relays stdio bytes to a Unix domain socket using `codex-stdio-to-uds`.
- hidden `tcp-tunnel` forwards a local TCP socket through an HTTP/3 CONNECT proxy.

They are transport plumbing, not general-purpose agent commands. Both are dispatched in `cli_main` and reject remote-mode combinations that would make their local ownership ambiguous.

## Internal schema/trace utilities

App-server schema generation and debug trace reduction are documented on their command pages because they expose generated protocol/state artifacts without being ordinary end-user runtime modes.
