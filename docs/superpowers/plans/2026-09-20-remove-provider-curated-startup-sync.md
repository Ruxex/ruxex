# Remove provider curated plugin startup sync

## Goal

Remove the provider-specific GitHub/ChatGPT curated-plugin startup synchronizer
while preserving generic local, Git, npm, and bundled plugin loading.

## Scope

- Remove the `genex-core-plugins` startup sync module and its tests.
- Remove the manager's automatic curated-repository sync entry points.
- Remove the startup-sync telemetry constants and imports.
- Keep ordinary configured marketplace upgrades and plugin cache refreshes.
- Record the producer/consumer audit in
  `docs/pre-release-compatibility-inventory.md` and update the Phase 3 handoff.

## Verification

- `cargo check -p genex-core-plugins`
- `git diff --check`
- `just check`
