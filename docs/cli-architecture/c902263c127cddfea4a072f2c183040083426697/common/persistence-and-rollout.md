# Persistence and rollout

## State families

- thread/session metadata and collections;
- queue state for messages sent to existing sessions;
- rollout/history events used to resume, inspect, or reduce state;
- local configuration, plugin, MCP, auth, and model caches;
- optional ephemeral execution state.

`ThreadStore` and `QueueStore` define storage boundaries for app-server/core. Rollout and history crates encode event/state persistence; the CLI chooses whether a command such as `--ephemeral`, archive, delete, or debug trace reduction reads or mutates those stores.

## Resume/fork/archive relationship

Resume reads an existing thread and reconstructs runtime context. Fork copies completed history into a new thread relationship. Archive/unarchive changes collection state; delete removes saved state subject to confirmation. These operations may be mediated by app-server requests even when invoked from a local CLI.

## Trace reduction

`codex debug trace-reduce` consumes a trace bundle and writes reduced state JSON. This is an offline diagnostic transformation, not normal thread persistence. See [debug](../commands/debug.md).
