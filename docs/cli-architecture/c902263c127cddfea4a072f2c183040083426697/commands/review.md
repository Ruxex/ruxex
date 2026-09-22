# `codex review`

## Entry

`codex review` uses [`ReviewCommand`](../../../../crates/cli/src/main.rs#L266), which wraps `codex_exec::ReviewArgs`. The root dispatch enters the same non-interactive execution family as `exec` at [main.rs#L1050](../../../../crates/cli/src/main.rs#L1050).

## Runtime shape

The review command supplies review-specific instructions and target selection to the exec runtime. It still shares configuration, provider/model selection, tool execution, approval/policy, and output handling with `codex exec`; the difference is the request construction and review prompt semantics.

```text
codex review [OPTIONS]
  -> ReviewCommand + ReviewArgs
  -> codex-exec review path
  -> repository/context collection
  -> model turn and tool calls
  -> non-interactive result output
```

Use the [exec page](exec.md) for the common execution lifecycle, and [provider/model](../common/provider-and-model.md) and [tool execution](../common/tool-execution.md) for shared boundaries.

## Boundary note

The CLI crate owns the top-level spelling and strict-config wrapper. `codex-exec` owns review argument interpretation and execution orchestration; `codex-core` owns the agent/runtime behavior used after the request is admitted.
