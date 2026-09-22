# System map

## Normalized pipeline

```text
argv
  -> clap parsing and root override folding
  -> CODEX_HOME/profile/config resolution
  -> authentication and provider/model selection
  -> local, remote, app-server, or direct transport selection
  -> thread/session admission
  -> turn, command, and tool execution
  -> approval, execpolicy, sandbox, and user verification
  -> rollout/history/thread persistence and telemetry
  -> TUI, JSONL, protocol, or process exit output
```

This is a conceptual normalization, not a single function call. Different commands skip or rearrange stages. For example, `codex completion` ends after CLI parsing, while `codex exec` can use app-server requests and `codex` interactive mode can run through a local daemon.

## Ownership by stage

| Stage | Primary owner | What it owns |
| --- | --- | --- |
| Parse and dispatch | [`codex-cli`](../../../crates/cli/src/main.rs) | `MultitoolCli`, `Subcommand`, root validation, and dispatch |
| Interactive argument model | [`codex-tui`](../../../crates/tui/src/cli.rs) | TUI-specific flags and wrapper state for resume/fork/agents |
| Non-interactive argument model | [`codex-exec`](../../../crates/exec/src/cli.rs) | `codex exec` options and exec subcommands |
| Configuration | [`codex-core`](../../../crates/core/src/config/mod.rs) and [`codex-config`](../../../crates/config/src/lib.rs) | loading, overrides, profiles, validation, and runtime config |
| Agent orchestration | [`codex-core`](../../../crates/core/src/lib.rs) | threads, turns, tools, approvals, model requests, and shared state |
| UI presentation | [`codex-tui`](../../../crates/tui/src/lib.rs) | terminal event loop, history cells, input, and rendering |
| App-server runtime | [`codex-app-server`](../../../crates/app-server/src/lib.rs) | request processing, transport, daemon-facing runtime, and lifecycle |
| Public protocol | [`codex-app-server-protocol`](../../../crates/app-server-protocol/src/lib.rs) and [`codex-protocol`](../../../crates/protocol/src/lib.rs) | wire types, notifications, requests, and shared domain types |
| Provider/API boundary | [`codex-api`](../../../crates/codex-api/src/lib.rs), [`codex-client`](../../../crates/codex-client/src/lib.rs) | provider requests, streaming, retries, auth headers, and transport |
| Execution/policy | [`codex-tools`](../../../crates/tools/src/lib.rs), [`codex-execpolicy`](../../../crates/execpolicy/src/lib.rs), [`codex-sandboxing`](../../../crates/sandboxing/src/lib.rs) | tool registry, command policy, and host sandbox construction |
| Persistence | [`codex-rollout`](../../../crates/rollout/src/lib.rs), [`codex-thread-store`](../../../crates/thread-store/src/lib.rs), [`codex-history`](../../../crates/history/src/lib.rs) | rollout files, thread metadata, history, and state stores |

## Important boundaries

### CLI versus runtime

`codex-cli` owns the user-facing command grammar and chooses a path. It does not own the whole behavior of a command. Runtime work is delegated to `codex-tui`, `codex-exec`, `codex-core`, the app-server, or utility crates.

### Protocol versus server

`codex-app-server-protocol` and `codex-protocol` define serializable shapes and domain values. `codex-app-server` processes requests and coordinates state. A protocol type being defined in a protocol crate does not mean that crate owns the behavior.

### Provider boundary versus agent behavior

`codex-api` and `codex-client` provide request/stream/transport boundaries. Core decides when to request a model response, how to interpret events, and when tools or approvals participate. This separation is the provider-neutral seam used by the rest of the repository.
