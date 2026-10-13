# Remove Fixed-Provider CLI Binary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the repository's fixed-provider `genex` executable after every active host, test, packaging, and child-process consumer has moved to a provider-owned host binary.

**Architecture:** Keep `genex-cli` as a host-support library only where its reusable helpers are still needed. Move the executable-facing startup/command boundary to an explicit provider-owned host, with the same executable reconstructing `P` in AppServer daemon mode. Keep `genex-app-server` as an operational standalone binary, but do not use it as the provider-owned desktop/TUI host.

**Tech Stack:** Rust workspace, Tokio, clap, `genex-app-server-daemon`, TUI, Tauri desktop packaging, Cargo integration tests.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`

## Global Constraints

- One concrete `ModelProvider` is compiled into each host runtime/AppServer.
- Provider instances never cross the process boundary; only serializable startup arguments do.
- The provider descriptor must be validated before credentials, SQLite, sessions, or caches open.
- Preserve provider-neutral agent, MCP, sandbox, security, and platform fallback behavior.
- Preserve external OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers.
- Work directly on `main`; run Rust checks through the repository's memory-limited `systemd-run --user --scope` recipes.

## Review Focus

- Desktop package bootstrap must launch the provider-owned host with the hidden daemon arguments rather than the old `app-server` subcommand.
- Existing CLI integration tests that exercise `exec`, sandbox, app-server, or provider-neutral commands must either move to a dedicated host fixture or be removed with an inventory decision.
- Core/TUI child-process tests must not silently fall back to a missing `genex` binary.
- `genex-cli` library exports must remain available to the example host only when they are not coupled to the deleted executable.
- Package manifests, `justfile`, install documentation, and generated/checked-in audit artifacts must describe the new host binary.

### Task 1: Complete the host-consumer audit

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`
- Create: this plan
- Inspect: `Cargo.toml`, `crates/cli`, `crates/app-server-daemon`, `crates/app-server-test-client`, `crates/core/tests`, `crates/tui/tests`, `apps/desktop`, `justfile`, and package/install docs

- [x] Record `remove`, `retain`, or `migrate-in-place` for each direct `genex` executable producer/consumer.
- [x] Keep the old binary during the transition because the current example host does not implement the full existing CLI subcommand surface.
- [x] Re-run exact-reference searches after each migration wave.

### Task 2: Migrate desktop daemon bootstrap

**Files:**
- Modify: `crates/app-server-daemon/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/scripts/test_desktop_justfile.py`
- Modify: `justfile`
- Modify: `examples/provider-host/src/main.rs`

- [x] Add a package-host bootstrap API that preserves desktop package preparation while launching the supplied host executable with its hidden daemon arguments.
- [x] Make the desktop runtime package build the provider-owned example/host binary and retain the staged `bin/genex` filename only as a package path if required by the desktop package manifest.
- [ ] Verify daemon start/restart/stop and provider-home marker behavior through the desktop host path.

### Task 3: Move or delete binary-only integration tests

**Files:**
- Modify/remove: `crates/cli/tests/**`
- Modify: `crates/core/tests/**`, `crates/tui/tests/**`, `crates/app-server/tests/**`
- Modify: `crates/app-server-test-client/README.md`

- [x] Replace generic child-process launches with the provider-owned host fixture where the exercised behavior remains in scope.
- [x] Remove tests for the deleted CLI-only command surface only after recording their producer/consumer decision.
- [x] Preserve direct AppServer and runtime contract coverage independent of any CLI executable.

### Task 4: Remove the fixed-provider executable target

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Modify/remove: `crates/cli/src/main.rs` and executable-only modules/tests
- Modify: workspace manifests and package/build documentation

- [x] Remove `[[bin]] name = "genex"` only after Tasks 2 and 3 have no active consumers.
- [x] Keep reusable host-library APIs in `genex-cli` without an executable dependency.
- [x] Keep `genex-app-server` binary available as the explicitly retained operational entrypoint.

### Task 5: Verify and finalize

- [x] Run exact `genex` binary/reference scans and inspect all remaining matches as retained external/security/package contracts or historical documentation.
- [x] Run `just check --tests` and `just check` through the required systemd scope.
- [ ] Run `just generate` if package/schema/generated artifacts changed, then inspect the diff.
- [ ] Commit and push the migration as a separate change from the generic host-daemon phase.
