# Pre-release Compatibility-layer Removal Design

## Context

This repository is a pre-release fork of `codex-rs` with zero external users.
The current code still contains compatibility paths for upstream Codex history,
legacy rollout formats, old protocol fields, deprecated configuration and CLI
surfaces, and older app-server clients. Those paths make the new agent
foundation larger and preserve contracts that this fork has not released.

## Decision

Perform a repository-wide compatibility cleanup before removing the next
OpenAI-specific integration. Remove code whose sole purpose is to accept,
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
- The workspace passes the applicable `cargo check` and `just check` gates.
