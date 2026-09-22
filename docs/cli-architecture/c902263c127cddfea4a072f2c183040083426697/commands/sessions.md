# Session commands

## Covered commands

`codex resume`, `codex fork`, `codex queue`, `codex archive`, `codex delete`, and `codex unarchive` all operate on recorded thread/session state, but they do not share one identical execution path.

## Interactive resume/fork

`ResumeCommand` and `ForkCommand` are defined in [`cli/src/main.rs`](../../../../crates/cli/src/main.rs#L304). The root command merges their flags into `codex_tui::Cli` in `finalize_resume_interactive` and `finalize_fork_interactive`, then calls `run_interactive_tui`.

- no id: picker mode;
- `--last`: newest eligible recorded session;
- explicit id/name: direct session selection;
- `--all`: disable current-directory filtering;
- optional prompt/images: send a new turn after resume/fork.

Fork creates a new session relationship; it is not a simple alias for resume. See [session/thread](../common/session-and-thread.md).

## Archive/delete/unarchive

The root dispatch converts the operation to `codex_tui::SessionArchiveAction` and calls `run_session_archive_cli_command` at [main.rs#L788](../../../../crates/cli/src/main.rs#L788). The action is implemented in [`codex-tui::session_archive_commands`](../../../../crates/tui/src/session_archive_commands.rs#L41). `delete --force` only bypasses confirmation when the target is a UUID, as enforced by `delete_action`.

## Queue

`queue` uses [`run_queue_command`](../../../../crates/cli/src/queue_cmd.rs#L29). It targets an existing session and routes the message through the app-server request path rather than starting an interactive TUI in the current terminal.

## State rules

- Archived sessions are a distinct collection and may require unarchive confirmation before resume/fork.
- Session lookup can use UUID or name, with UUID precedence where the command documents it.
- CWD filtering and inclusion of non-interactive sessions are command-level selection rules; the underlying thread model is shared.

See [persistence and rollout](../common/persistence-and-rollout.md), [transport and server](../common/transport-and-server.md), and [output/errors](../common/output-and-errors.md).
