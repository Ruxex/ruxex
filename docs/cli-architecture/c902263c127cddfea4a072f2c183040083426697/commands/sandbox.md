# `codex sandbox`

## Entry

`HostSandboxArgs` is selected at compile time for macOS Seatbelt, Linux Landlock, or Windows sandbox implementations in [`cli/src/main.rs`](../../../../crates/cli/src/main.rs#L414). The root dispatch is at [main.rs#L1442](../../../../crates/cli/src/main.rs#L1442), and platform-specific runners live in `crates/cli/src/debug_sandbox.rs`.

## Flow

```text
codex sandbox [PROFILE OPTIONS] -- COMMAND
  -> host-specific parser
  -> config/profile and permission profile resolution
  -> platform command construction
  -> child process execution under sandbox
  -> child exit status propagation
```

The sandbox command is a local process wrapper, not the same as the approval policy applied to model-issued tools. Both eventually use platform policy primitives, but the caller and lifecycle differ. See [sandbox and permissions](../common/sandbox-and-permissions.md), [policy](policy.md), and [output/errors](../common/output-and-errors.md).

Remote mode is rejected. Unsupported operating systems fail explicitly rather than silently running unsandboxed.
