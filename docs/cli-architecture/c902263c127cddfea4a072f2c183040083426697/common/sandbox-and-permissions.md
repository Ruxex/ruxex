# Sandbox, approval, and permissions

## Four layers

1. CLI-level sandbox wrappers run an explicitly supplied child command.
2. Tool approval decides whether a model-issued operation needs human confirmation.
3. Execpolicy evaluates command rules and can contribute an allow/deny/ask decision.
4. Platform sandboxing constrains the admitted child process; user verification can add another security gate.

Core composes the tool-level layers through [`Approvable`](../reference/trait-index.md#approvable), [`Sandboxable`](../reference/trait-index.md#sandboxable), and [`ToolRuntime`](../reference/trait-index.md#toolruntime) in [`core/src/tools/sandboxing.rs`](../../../../crates/core/src/tools/sandboxing.rs#L309). `codex sandbox` uses platform constructors in `codex-sandboxing` and `codex-cli` debug sandbox helpers.

## Safety invariant

Approval and sandboxing are not interchangeable. An approval decision may allow an operation to proceed, but the resulting process still runs within the selected sandbox/profile. Conversely, a sandbox restriction does not tell the user why approval was or was not requested.

Platform selection is compile-time/host-specific: macOS Seatbelt, Linux Landlock, and Windows sandbox paths are selected by `HostSandboxArgs`. Unsupported platforms fail explicitly.
