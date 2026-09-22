# `codex` interactive

## Entry

`codex [OPTIONS] [PROMPT]` enters the interactive path when no subcommand is supplied. The root parser embeds [`codex_tui::Cli`](../../../../crates/tui/src/cli.rs#L11), while the actual dispatch is in [`cli_main`](../../../../crates/cli/src/main.rs#L930) and the TUI startup helper is [`run_interactive_tui`](../../../../crates/cli/src/main.rs#L2195).

The TUI argument object contains user prompt, strict-config, approval mode, web search, alternate-screen, daemon, shared config, and internal resume/fork/agents state. The internal fields are populated by top-level wrapper commands and are not independent user flags.

## Flow

```text
codex [PROMPT]
  -> MultitoolCli + TuiCli
  -> root config/remote/worktree validation
  -> run_interactive_tui
  -> codex_tui startup orchestration
  -> app-server/direct session startup
  -> terminal event loop and turns
  -> AppExitInfo -> exit/update handling
```

The TUI delegates model turns, tools, approvals, and persistence to shared runtime services. See [configuration](../common/configuration.md), [session/thread](../common/session-and-thread.md), [tool execution](../common/tool-execution.md), and [output/errors](../common/output-and-errors.md).

## Important constraints

- The TUI checks terminal capability and can reject non-terminal stdin/stdout.
- `--no-daemon` changes startup routing; it does not remove the core session model.
- `--ask-for-approval` and the dangerous bypass flag are mutually constrained by `codex-tui::Cli`.
- Remote mode and additional-directory overrides are validated in the root CLI before startup.

## Relevant ownership

| Concern | Definition |
| --- | --- |
| Root command | `codex-cli::MultitoolCli`, `crates/cli/src/main.rs` |
| TUI arguments | `codex-tui::Cli`, `crates/tui/src/cli.rs` |
| Session runtime | `codex-core`, reached through TUI/app-server startup |
| UI event source | `codex-tui::EventSource`, see [Trait index](../reference/trait-index.md#eventsource) |
