# `codex exec`

## Entry

`codex exec [OPTIONS] [PROMPT]` is parsed by [`codex_exec::Cli`](../../../../crates/exec/src/cli.rs#L15) and dispatched by `codex-cli` at [main.rs#L1034](../../../../crates/cli/src/main.rs#L1034). The executable supports a new non-interactive session when no nested command is present and nested `resume`, `fork`, and `review` actions through [`Command`](../../../../crates/exec/src/cli.rs#L149).

## Input and output model

The exec parser owns flags for:

- prompt/stdin input;
- JSONL event output and last-message file output;
- output schema for the final response;
- ephemeral execution and config/rules loading controls;
- global model, approval bypass, hook trust, and worktree options.

`codex_exec::run_main` is the runtime entry at [`exec/src/lib.rs#L244`](../../../../crates/exec/src/lib.rs#L244). It loads config, resolves or starts the app-server/session path, sends requests, processes events, and maps completion/error state to process output.

## Flow

```text
codex exec
  -> codex_exec::Cli
  -> codex_exec::run_main
  -> config/bootstrap
  -> start/resume/fork thread
  -> send prompt through app-server/core
  -> event processing
  -> text, JSONL, schema-shaped output, or exit code
```

`--ephemeral` is a persistence choice, not a different model protocol. See [configuration](../common/configuration.md), [session/thread](../common/session-and-thread.md), [output/errors](../common/output-and-errors.md), and [persistence](../common/persistence-and-rollout.md).

## Relevant types

| Type | Crate | Role |
| --- | --- | --- |
| `Cli`, `Command` | `codex-exec` | command grammar |
| `EventProcessor` | `codex-exec` | internal event-to-output processing |
| `ResponseEvent` | `codex-api` | model response stream boundary |
| `ThreadId`, `ClientRequest` | `codex-protocol` / app-server protocol | request/session identity |
