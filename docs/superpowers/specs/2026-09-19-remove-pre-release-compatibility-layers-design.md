# Pre-release Compatibility-layer Removal Design

## Context

This repository is a pre-release fork of `codex-rs` with zero external users.
The current code still contains compatibility paths for upstream Codex history,
legacy rollout formats, old protocol fields, deprecated configuration and CLI
surfaces, and older app-server clients. Those paths make the new agent
foundation larger and preserve contracts that this fork has not released.

Tracking documents: [implementation plan](../plans/2026-09-19-remove-pre-release-compatibility-layers.md),
[compatibility inventory](../../pre-release-compatibility-inventory.md), and
[provider decoupling handoff](../../provider-decoupling-plan.md).

## Decision

Perform a repository-wide compatibility cleanup before removing the next
provider-specific integration. Remove code whose sole purpose is to accept,
produce, migrate, replay, or test historical contracts from before this fork.

Do not interpret every `legacy`-named symbol as removable. Keep a compatibility
path when it is required by the current runtime, a currently used in-repository
client, platform support, security behavior, or a provider-neutral capability.
The cleanup begins with an inventory that records this distinction for every
candidate.

After the cleanup, old local rollout/config files are unsupported. The
implementation must not delete those files automatically; it only removes the
code that reads, migrates, or preserves obsolete formats.

## Compatibility categories to remove

- Persisted rollout and thread-history migrations for obsolete event or item
  shapes.
- Legacy event aliases, old attribution/config state, and historical replay
  projections with no current producer.
- Deprecated app-server fields, old protocol aliases, and compatibility-only
  response shapes when no in-repository client still needs them.
- Deprecated CLI commands, config aliases, and import paths that are not part
  of the current product surface.
- UI replay, analytics, telemetry, trace, and tests that exist only to preserve
  those obsolete shapes.

### Build-system compatibility and experimental V8 scaffolding

The `codex-v8-poc` crate is an unintegrated proof-of-concept and is not used by
the product path. The actual code-mode runtime depends on `rusty_v8` directly,
so the POC crate may be removed independently. Its workspace registration,
workspace dependency, cargo-shear exclusion, lockfile entry, and generated
dependency-graph references must be removed together.

Bazel build infrastructure is not maintained in this fork, but Bazel
compatibility branches still exist in shared test and resource-resolution
helpers. Remove those branches in stages rather than deleting shared helpers
or Cargo behavior wholesale:

- Keep Cargo binary and resource resolution in `genex-utils-cargo-bin` while
  removing its runfiles-only branch and the `runfiles` dependency.
- Remove the Bazel-only `codex_bazel` protocol path after the canonical Cargo
  protobuf path is confirmed as the sole supported path.
- Remove Bazel-specific sandbox binary lookup and test-environment handling
  after confirming that local Cargo tests retain their current behavior.
- Remove the `bazel_native` lint feature only after preserving the normal
  Dylint source-build path used by the repository's lint commands.
- Remove stale Bazel metadata, comments, fixtures, and generated references
  after the code paths are gone.

This slice does not remove `rusty_v8`, `genex-code-mode-runtime`, or the
provider-neutral code-mode capability. It removes only the unintegrated POC
crate and build-system compatibility that has no maintained Bazel consumer.

## Compatibility categories to review, not remove automatically

- Platform fallbacks such as Windows sandbox behavior when still required by
  current supported environments.
- Active protocol versions or fields used by current in-repository clients
  until those clients are migrated in the same change.
- Provider-neutral image/audio/media handling and current tool behavior.
- Security or data-loss safeguards whose purpose is current operation rather
  than historical compatibility.

## Success criteria

- A tracked inventory identifies every compatibility candidate and its decision.
- Obsolete compatibility code is removed before the next provider-specific
  integration cleanup.
- Generic current behavior remains available and is not removed merely because
  a nearby symbol contains `legacy`, `deprecated`, or `fallback`.
- Old local data is not silently deleted, but the unsupported-data policy is
  explicit in project documentation.
- `codex-v8-poc` is removed without removing the active V8/code-mode runtime.
- Bazel compatibility is removed without breaking Cargo builds, Cargo tests,
  sandbox behavior, or current test-resource resolution.
- The workspace passes the applicable `cargo check` and `just check` gates.
