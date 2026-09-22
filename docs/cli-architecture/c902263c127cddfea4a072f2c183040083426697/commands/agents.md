# `codex agents`

## Entry

`codex agents` selects the same interactive TUI binary with `agents_overview = true`. The command shape is [`AgentsCommand`](../../../../crates/cli/src/main.rs#L287), and the branch is in the `None | Some(Subcommand::Agents(_))` arm at [main.rs#L981](../../../../crates/cli/src/main.rs#L981).

The overview is intended to browse sessions on a shared local app-server daemon. It rejects an initial prompt or images and validates remote endpoint conflicts before entering the TUI.

## Flow and constraints

```text
codex agents
  -> AgentsCommand options
  -> endpoint and remote-mode validation
  -> TuiCli.agents_overview = true
  -> run_interactive_tui
  -> daemon-wide thread/session overview
```

- A shared server is required for the overview; `--no-daemon` is rejected.
- On supported platforms, absence of `--remote` resolves the local daemon endpoint.
- Workload identity and some additional-directory overrides are incompatible with this path.
- The overview is a UI mode, not a second thread execution engine.

See [transport and server](../common/transport-and-server.md), [session and thread](../common/session-and-thread.md), and [interactive](interactive.md).
