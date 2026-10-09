# Close Single-Provider Residuals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining single-provider boundary gaps so every local startup validates the provider-scoped home before sensitive state, and every production model-catalog/debug path uses one host-constructed provider runtime.

**Architecture:** Add one app-server host helper for provider-home mounting and configured runtime construction. Exec, TUI, and CLI debug paths call that helper before state or credentials and pass the resulting runtime/provider downward. Core prompt-debug and catalog code consume an injected `SharedModelProvider`; core no longer constructs a provider for callers.

**Tech Stack:** Rust, Tokio, AppServerRuntime, SharedModelProvider, provider-home marker, repository `just` verification recipes.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`

## Global Constraints

- One `ModelProvider` instance belongs to each Genex runtime/AppServer.
- `genex-home.json` must be validated before opening SQLite, sessions, caches, or credentials.
- Threads and rollouts must not select or persist provider routing.
- Provider-specific implementation and credentials remain behind the host adapter boundary.
- Work directly on `main`; do not create a Git worktree.
- Rust verification must use the repository `just` recipes under the memory-limited systemd scope.

## Review Focus

- Existing or mismatched provider homes: reject before state DB and credentials are opened.
- Exec fork/worktree startup: use the same runtime and marker boundary as ordinary Exec startup.
- TUI embedded startup: do not silently fall back to configuration-based provider construction.
- `debug models` and `debug prompt-input`: use host-created provider state and retain model-catalog behavior.
- Doctor/plugin diagnostics: validate the home before reading provider credentials, while preserving read-only behavior.

### Task 1: Record the audited decisions

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

- [ ] Add explicit `migrate-in-place` decisions for the Exec/worktree startup ordering, CLI debug provider construction, and doctor/plugin credential reads, naming their producers and consumers.
- [ ] Mark the corresponding candidate findings as resolved only after the implementation tasks below are complete.

### Task 2: Centralize host startup preparation

**Files:**
- Modify: `crates/app-server/src/runtime.rs`
- Modify: `crates/app-server/src/in_process.rs`
- Test: `crates/app-server/src/runtime.rs` and existing in-process tests

**Interfaces:**
- Produce `mount_provider_home_for_config(&Config)` for all local host surfaces.
- Produce an async configured runtime helper that mounts first, loads credentials second, and constructs `AppServerRuntime` third.
- Preserve support for injected runtimes without forcing a second credential-manager load.

- [ ] Add failing unit coverage for descriptor-based config mounting and injected-runtime startup behavior.
- [ ] Implement the shared mount/runtime helpers and make `start_uninitialized` reuse an injected runtime’s credential manager when available.
- [ ] Run the affected compile-only test target through `just check --tests`.

### Task 3: Route Exec, worktree, and TUI through the injected runtime

**Files:**
- Modify: `crates/exec/src/lib.rs`
- Modify: `crates/exec/src/worktree.rs`
- Modify: `crates/tui/src/lib.rs`
- Test: existing Exec/TUI startup tests plus new marker-order regression coverage where the harness permits

- [ ] Add failing coverage for an incompatible provider home being rejected before local state initialization.
- [ ] Prepare the configured runtime before `init_state_db` in Exec and fork/worktree paths, and pass it through `InProcessClientStartArgs`.
- [ ] Prepare and pass the runtime in embedded TUI startup while preserving the existing embedded state initialization behavior.
- [ ] Run `just check --tests` for the affected crates.

### Task 4: Remove core provider reconstruction from debug/catalog paths

**Files:**
- Modify: `crates/core/src/prompt_debug.rs`
- Modify: `crates/core/src/thread_manager.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/src/test_support.rs`
- Modify: `crates/core/tests/common/test_genex.rs`
- Modify: `crates/core/tests/suite/prompt_debug_tests.rs`
- Modify: `crates/cli/src/main.rs`

- [ ] Add failing compile/test coverage requiring prompt-debug to receive a provider rather than constructing one.
- [ ] Change prompt-debug to consume the injected provider and its credential manager.
- [ ] Remove production `build_model_provider`/`build_models_manager` constructors from core; retain only `build_models_manager_with_provider`.
- [ ] Construct the host runtime in `debug models` and `debug prompt-input` after provider-home validation, then pass its provider into core.
- [ ] Run `just check --tests` for core and CLI targets.

### Task 5: Protect doctor and plugin credential reads

**Files:**
- Modify: `crates/cli/src/doctor.rs`
- Modify: `crates/cli/src/plugin_cmd.rs`
- Test: existing doctor/plugin tests and marker failure coverage

- [ ] Add failing coverage for missing/incompatible markers before credential loading in these command paths.
- [ ] Mount the provider home before `CredentialManager::shared_from_config` and preserve doctor’s read-only semantics.
- [ ] Run `just check --tests` for CLI targets.

### Task 6: Final audit and verification

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

- [ ] Re-scan production Rust for direct provider construction, `runtime: None` in local startup, and credential/state initialization before marker mounting.
- [ ] Record any intentionally retained adapter/test-only construction as `retain` with evidence.
- [ ] Run `just check`, `just check --tests`, `git diff --check`, and `git status --short --branch`.
- [ ] Run `just generate` only if the final diff affects checked-in generated artifacts, then inspect the generated diff.
