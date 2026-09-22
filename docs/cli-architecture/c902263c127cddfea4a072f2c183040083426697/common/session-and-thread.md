# Session and thread lifecycle

## Terminology

- A thread is the persisted/conversational unit identified by a thread ID.
- A turn is one user/model interaction within a thread.
- A session is the CLI-facing workflow around starting, resuming, forking, queueing, or displaying a thread.

The CLI wrappers in `codex-cli` select a thread/session operation; `codex-tui`, `codex-exec`, and `codex-app-server` coordinate the runtime.

## Lifecycle

```text
select/create thread
  -> load thread config and persisted context
  -> admit/start turn
  -> queue, steer, interrupt, complete, or fail
  -> persist rollout/thread metadata
  -> resume/fork/archive/delete/unarchive later
```

Thread persistence is abstracted by [`ThreadStore`](../reference/trait-index.md#threadstore), and queued messages by [`QueueStore`](../reference/trait-index.md#queuestore). Core task admission uses the internal [`SessionTask`](../reference/trait-index.md#sessiontask) contract.

## Command distinctions

- `resume` re-enters an existing thread, optionally starting a new turn.
- `fork` creates a new thread from completed source history.
- `queue` sends work to an existing session through a server path.
- archive/unarchive changes collection/state visibility.
- delete removes saved state and requires command-specific confirmation rules.

See [session commands](../commands/sessions.md) and [persistence](persistence-and-rollout.md).
