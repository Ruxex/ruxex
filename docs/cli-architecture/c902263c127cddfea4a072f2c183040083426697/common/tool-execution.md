# Tool execution

## Normalized path

```text
model response tool call
  -> tool name/argument routing
  -> ToolExecutor handler or CoreToolRuntime
  -> approval decision
  -> sandbox construction
  -> process/network/filesystem operation
  -> ToolOutput / protocol event
```

The generic tool boundary is [`codex_tools::ToolExecutor`](../reference/trait-index.md#toolexecutor). Core's registry adds [`CoreToolRuntime`](../reference/trait-index.md#coretoolruntime), while concrete execution runtimes implement [`ToolRuntime`](../reference/trait-index.md#toolruntime) together with approval and sandbox capabilities.

`TurnItemEmitter` and `ToolOutput` carry tool results/events back toward the turn and output layers. MCP, plugins, code mode, shell, patch, and built-in tools share this routing shape but differ in handler and policy details.

## Important distinction

The `codex exec` CLI is not the same thing as an agent-issued exec tool. The former is a top-level non-interactive command; the latter is a tool runtime inside a turn. They can share core execution primitives while having different input, approval, and output boundaries.
