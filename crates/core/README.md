# genex-core

This crate implements the business logic for Genex. It is designed to be used by the various Genex UIs written in Rust.

## Dependencies

Note that `genex-core` makes some assumptions about certain helper utilities being available in the environment. Currently, this support matrix is:

### macOS

Expects `/usr/bin/sandbox-exec` to be present.

When using the workspace-write sandbox policy, the Seatbelt profile allows
writes under the configured writable roots while keeping `.git` (directory or
pointer file), the resolved `gitdir:` target, and `.genex` read-only.

Network access and filesystem read/write roots are controlled by
`SandboxPolicy`. Seatbelt consumes the resolved policy and enforces it.

Seatbelt also keeps the legacy default preferences read access
(`user-preference-read`) needed for cfprefs-backed macOS behavior.

### Linux

Expects the binary containing `genex-core` to run the equivalent of `genex sandbox` when `arg0` is `genex-linux-sandbox`. See the `genex-arg0` crate for details.

Legacy `SandboxPolicy` / `sandbox_mode` configs are still supported on Linux.
They can continue to use the legacy Landlock path when the split filesystem
policy is sandbox-equivalent to the legacy model after `cwd` resolution.
Split filesystem policies that need direct `FileSystemSandboxPolicy`
enforcement, such as read-only or denied carveouts under a broader writable
root, automatically route through bubblewrap. The legacy Landlock path is used
only when the split filesystem policy round-trips through the legacy
`SandboxPolicy` model without changing semantics. That includes overlapping
cases like `/repo = write`, `/repo/a = none`, `/repo/a/b = write`, where the
more specific writable child must reopen under a denied parent.

The Linux sandbox helper prefers the first `bwrap` found on `PATH` outside the
current working directory whenever it is available. If `bwrap` is present but
too old to support `--argv0`, the helper keeps using system bubblewrap and
switches to a no-`--argv0` compatibility path for the inner re-exec. If
`bwrap` is missing, it falls back to the bundled `genex-resources/bwrap`
binary shipped with Genex and Genex surfaces a startup warning through its
normal notification path instead of printing directly from the sandbox helper.
Genex also surfaces a startup warning when bubblewrap cannot create user
namespaces. WSL2 uses the normal Linux bubblewrap path. WSL1 is not supported
for bubblewrap sandboxing because it cannot create the required user
namespaces, so Genex rejects sandboxed shell commands that would enter the
bubblewrap path before invoking `bwrap`.

### All Platforms

Expects the binary containing `genex-core` to simulate the virtual
`apply_patch` CLI when `arg1` is `--genex-run-as-apply-patch`. See the
`genex-arg0` crate for details.
