# Audit `just check` Warnings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce only the Linux `just check` warnings whose producer/consumer audit proves they are target-independent dead code, while preserving code used by macOS, Windows, tests, security, or future provider-neutral integrations.

**Architecture:** Treat each warning as a target-specific observation, not a deletion decision. Record the producer/consumer and `cfg` audit in the compatibility inventory, then remove only unconditionally unused imports/arguments and stale wrappers; use narrowly scoped target/test allowances for intentionally retained code.

**Tech Stack:** Rust workspace, Cargo target configuration, `rg`, `just check --tests`, and `just check`.

**Spec:** `AGENTS.md` and `docs/pre-release-compatibility-inventory.md`.

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Do not remove current platform fallbacks, active clients, security behavior, or provider-neutral capabilities merely because Linux reports `dead_code`.
- Record `remove`, `retain`, or `migrate-in-place` after auditing producers and consumers before changing production code.
- Do not run standalone tests; use `just check --tests` for Rust changes and `just check` as the final workspace gate.

## Review Focus

- macOS-only config loading must remain compiled and functional; Linux-only warnings must not remove it.
- Windows sandbox, network approval, and filesystem policy code must remain available even when Linux does not exercise every field or branch.
- Test-only helpers must be checked with `just check --tests` before being considered dead production code.
- Retained provider-neutral gaps documented in `docs/provider-neutral-gaps.md` must not be deleted as warning cleanup.
- Vendor C warnings and dependency future-incompatibility warnings must remain separate from Rust dead-code cleanup.

### Task 1: Record the warning audit

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`

- [x] Record the macOS-only strict-config helper as `retain` with its macOS producer.
- [x] Record target-independent dead imports/arguments/wrappers as `remove` candidates with exact call-site audits.
- [x] Record retained provider-neutral helpers, platform/security paths, test-only paths, vendor C warnings, and dependency warnings as `retain` or out of scope.

### Task 2: Remove target-independent warning sources

**Files:**
- Modify: `crates/core/src/tools/registry.rs`
- Modify: `crates/core/src/turn_metadata.rs`
- Modify: `crates/core/src/client.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/core/src/session/mod.rs`
- Modify: `crates/core/src/cyber_access_program.rs`
- Modify: `crates/tui/src/status/mod.rs`

- [x] Remove only unused imports and unnecessary mutability.
- [x] Remove the unused result binding in `record_initial_history` without changing its side effects.
- [x] Remove the unused `CodexAuth` argument from the provider-neutral no-op access-program adapter and update its two callers.
- [x] Remove the unused TUI re-export while preserving the helper's actual consumer in `status/rate_limits.rs`.

### Task 3: Preserve target-specific and intentional code explicitly

**Files:**
- Modify: `crates/config/src/strict_config.rs`

- [x] Add a narrowly scoped non-macOS dead-code allowance to the macOS-consumed strict-config helper, documenting why it is retained.
- [x] Do not alter approval, sandbox, daemon, app-server transport, or provider-neutral future-integration code in this pass.

### Task 4: Verify the warning reduction

- [x] Run `just check --tests`.
- [x] Run `just check`.
- [x] Re-audit the remaining warnings and inspect the diff for accidental platform/security changes.

## Additional formatting verification

- [x] Run `cargo fmt --all` across the workspace at the user's request.
- [x] Confirm with `cargo fmt --all -- --check` and `git diff --check`.
