# Remove Unused Compatibility Surfaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the unused SQLite feature toggle and the deprecated `autoResolutionMs` compatibility field while retaining SQLite state storage and current request-blocking behavior.

**Architecture:** SQLite remains an always-on implementation detail of local state and is not replaced. The obsolete `Feature::Sqlite` registry entry and test-only enables are removed. `autoResolutionMs` is removed from current Rust and generated app-server protocol surfaces; deserializers continue accepting and ignoring the old JSON key so existing rollout/protocol input can still be read.

**Tech Stack:** Rust workspace, serde/schema generation, checked-in app-server JSON schemas, `just generate`, `just check --tests`, and `just check`.

**Spec:** `docs/pre-release-compatibility-inventory.md`, `docs/provider-decoupling-candidates.md`, and the repository `AGENTS.md` cleanup policy.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Keep the SQLite StateRuntime, SQLite databases, JSONL rollout source of truth, and SQLite-less fallback behavior.
- Record explicit `remove` or `retain` decisions before changing production contracts.
- Preserve old `autoResolutionMs` input compatibility while removing the deprecated field from new output and generated schemas.
- Run Rust checks only through the repository `just` recipes and the user systemd memory scope.
- Run `just check --tests` for Rust changes, `just check` always, and `just generate` when checked-in generated artifacts change.

## Review Focus

- Removing `Feature::Sqlite` must not disable SQLite initialization or alter state-store behavior.
- Removing `autoResolutionMs` must not make older JSONL/protocol input unreadable.
- The generated schema must no longer advertise `autoResolutionMs` or `sqlite` as user-configurable surfaces.
- `isBlocking` must remain the only current request-blocking control.
- No OpenAI endpoint, authentication, model, persisted rollout, or security contract may be changed unintentionally.

### Task 1: Record producer/consumer decisions

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

**Interfaces:**
- Records that `Feature::Sqlite` is an orphaned configuration/registry flag while the SQLite runtime is retained.
- Records that `autoResolutionMs` is a deprecated, behaviorless compatibility field; old input is tolerated, current output/schema removes it.

- [ ] Add the two decisions before production edits, with exact producer/consumer evidence and verification commands.

### Task 2: Remove the unused SQLite feature flag

**Files:**
- Modify: `crates/features/src/lib.rs`
- Modify: test callers using `.enable(Feature::Sqlite)` under `crates/core/tests`, `crates/app-server/tests`, and related fixtures.
- Modify: generated feature/config artifacts if `just generate` changes them.

**Interfaces:**
- SQLite remains constructed through `genex_state::StateRuntime`; only the feature enum/spec and redundant test toggles are removed.

- [ ] Remove `Feature::Sqlite` and its registry entry.
- [ ] Remove test-only `Feature::Sqlite` enables without changing direct SQLite setup.
- [ ] Confirm production SQLite initialization references remain present.

### Task 3: Remove deprecated `autoResolutionMs` from current output

**Files:**
- Modify: `crates/protocol/src/request_user_input.rs`
- Modify: `crates/core/src/session/mod.rs` and request-user-input construction sites.
- Modify: `crates/app-server/src/bespoke_event_handling.rs`.
- Modify: `crates/app-server-protocol/src/protocol/v2/item.rs` and related protocol tests.
- Modify: test clients, TUI fixtures, and checked-in generated schemas affected by the field removal.

**Interfaces:**
- `RequestUserInputArgs` and `RequestUserInputEvent` no longer serialize or expose `auto_resolution_ms`.
- Custom deserialization ignores the old `autoResolutionMs` input instead of storing it.
- `isBlocking` remains unchanged and controls current behavior.

- [ ] Add/update the compatibility test first so old input with `autoResolutionMs` deserializes without preserving the field and current serialization omits it.
- [ ] Remove the field from producers, relay code, app-server protocol types, clients, fixtures, and schemas.
- [ ] Regenerate and inspect only the intended schema changes.

### Task 4: Verify, review, commit, and push

**Files:**
- Inspect all changed files and generated diffs.

- [ ] Run exact residual searches for `Feature::Sqlite`, `auto_resolution_ms`, and `autoResolutionMs`, classifying only historical documentation or retained compatibility parser references.
- [ ] Run memory-scoped `just check --tests`.
- [ ] Run memory-scoped `just check`.
- [ ] Run `git diff --check` and a final diff review.
- [ ] Commit the changes on `main`.
- [ ] Push the commit to `origin/main`.
