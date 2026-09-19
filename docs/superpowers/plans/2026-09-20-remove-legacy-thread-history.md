# Remove Legacy Thread History Compatibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make paginated thread history the only current writer contract, then remove the obsolete legacy rollout reader, migration state machine, and CLI migration command without deleting old files automatically.

**Architecture:** The current rollout writer and SQLite projection already support paginated history, while legacy mode remains the serde fallback for old metadata and blocks paginated reads in some stores. First move every current creation/import boundary to `ThreadHistoryMode::Paginated` and remove the rejection guards, while retaining the enum fallback until old metadata readers are removed. Once no current producer depends on legacy persistence, remove legacy-only persistence branches and the migration-only rollout conversion stack. Historical files remain on disk but are unsupported after the cleanup.

**Tech Stack:** Rust workspace, Cargo, JSONL rollout files, SQLite thread-history projection, app-server protocol, CLI subcommands, and repository compatibility inventory.

**Spec:** `docs/superpowers/specs/2026-09-19-remove-pre-release-compatibility-layers-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- This fork has zero external users; old persisted rollout contracts are unsupported after cleanup.
- Do not delete local rollout/config files automatically; remove only obsolete readers, migrations, aliases, and replay paths.
- Preserve canonical paginated writing, SQLite projection, paging, search, fork lineage, metadata, deletion, and provider-neutral tools/media.
- Preserve current external-agent import as a supported workflow, but emit canonical paginated history from it.
- For crate-removal work, use `cargo check` and repository-wide `just check` as primary verification; deletion-specific TDD is not required.
- Update `docs/pre-release-compatibility-inventory.md` with producer/consumer evidence before each removal boundary.

## Review Focus

- New threads created through Local, InMemory, app-server, and external-agent import use paginated history.
- Paginated rollouts retain ordinals, SQLite projection, fork boundaries, and metadata when shutdown or flush occurs.
- The in-memory store no longer rejects the active paginated contract.
- Old rollout files are not deleted as a side effect of removing their readers or migration command.
- Removing the migration command does not remove current thread listing, resume, paging, archive, delete, or fork paths.

## Current-state audit

- `ThreadHistoryMode` defaults to `Legacy` in `crates/protocol/src/protocol.rs` so omitted fields in old metadata remain readable; current store creation now defaults to `Paginated`.
- Local and in-memory stores accept paginated create, resume, and history reads; their explicit old-file compatibility branches remain for the later cleanup slice.
- `crates/app-server/src/external_agent_migration/session_importer.rs` now creates canonical paginated history for the retained import workflow.
- The former `crates/thread-store/src/local/rollout_migration/` and `crates/cli/src/migrate_rollouts.rs` conversion surface was removed in Task 3; old rollout files remain untouched on disk.
- `ThreadHistoryMode::Legacy` has broad test coverage and historical readers; those call sites are split between current-default migration work and later removal work rather than deleted by name alone.

### Task 1: Make paginated history the current writer contract

**Files:**
- Modify: `crates/protocol/src/protocol.rs`
- Modify: `crates/thread-store/src/store.rs`
- Modify: `crates/thread-store/src/error.rs`, `crates/thread-store/src/in_memory.rs`, and `crates/thread-store/src/local/read_thread.rs`
- Modify: `crates/app-server/src/external_agent_migration/session_importer.rs`, `crates/app-server/src/request_processors/thread_summary.rs`, and state/test defaults that create new threads
- Test: `crates/thread-store` and affected core/app-server tests

**Interfaces:**
- Produces: all new `SessionMeta.history_mode` and `CreateThreadParams.history_mode` defaults are `ThreadHistoryMode::Paginated`.
- Preserves: explicit legacy metadata can still be read during this transition; this task does not delete the legacy enum or reader branches.

- [x] Change current store, import, and synthetic metadata defaults to `Paginated`, while retaining the enum serde default until Task 4 removes old-metadata readers; update the focused default/serialization tests.
- [x] Remove `reject_paginated_history_mode` and its callers so current stores accept paginated create, resume, and read requests.
- [x] Change current external-agent import and app-server synthetic metadata paths to create paginated threads.
- [x] Run `cargo test -p codex-thread-store` and the focused app-server/core tests; run `cargo check -p codex-thread-store -p codex-core -p codex-app-server`.

### Task 2: Remove legacy-only persistence branches after Task 1 is green

**Files:**
- Modify: `crates/rollout/src/policy.rs`, `crates/rollout/src/ordinal.rs`, and `crates/rollout/src/state_db.rs`
- Modify: `crates/thread-store/src/local/live_writer.rs`, `read_thread.rs`, `helpers.rs`, `thread_rollout_resolver.rs`, `update_thread_metadata.rs`, and `thread_history/read.rs`
- Modify: current tests that construct legacy rollouts only to exercise the removed writer path

**Interfaces:**
- Consumes: paginated-only current writer contract from Task 1.
- Produces: current writes always persist canonical `ItemCompleted`/response records, ordinals, and SQLite projection; no new branch consults legacy mode.

- [ ] Convert current persistence filtering and ordinal initialization to the paginated path while retaining only explicitly scoped old-file handling until Task 3.
- [ ] Remove rollout filename/title/metadata fallbacks whose only consumer is legacy current-mode writing.
- [ ] Verify fork lineage, shutdown/flush projection, thread paging, and metadata updates with focused tests.
- [ ] Run `cargo test -p codex-rollout -p codex-thread-store` and `cargo check -p codex-rollout -p codex-thread-store`.

### Task 3: Delete the obsolete rollout migration surface

**Files:**
- Delete: `crates/thread-store/src/local/rollout_migration.rs` and its migration-only modules under `crates/thread-store/src/local/rollout_migration/`
- Delete: `crates/cli/src/migrate_rollouts.rs`
- Modify: `crates/thread-store/src/local/mod.rs`, `crates/cli/src/main.rs`, CLI command/help/completion tests, and manifests only if dependency edges change
- Modify: `docs/pre-release-compatibility-inventory.md` and `docs/openai-decoupling-plan.md`

**Interfaces:**
- Consumes: canonical paginated rollouts and SQLite projection from Tasks 1–2.
- Produces: no migration-only public API, startup migration, rollback replay, or `migrate-rollouts` CLI surface.

- [x] Remove migration module exports and startup invocation without adding automatic file deletion.
- [x] Remove migration-only tests and preserve current rollout listing/resume/fork/delete tests.
- [x] Refresh the inventory rows for `RolloutMigration*`, legacy replay, and filename fallback to `remove` with exact producer/consumer evidence.
- [x] Run `cargo check -p codex-thread-store -p codex-rollout -p codex-cli` and focused non-destructive checks.

### Task 4: Remove the remaining legacy mode and finish the audit

**Files:**
- Modify: `crates/protocol/src/protocol.rs`, `crates/state/`, `crates/history/`, `crates/app-server-protocol/`, `crates/core/`, `crates/tui/`, and tests selected by the refreshed inventory
- Modify: `docs/pre-release-compatibility-inventory.md` and this plan

- [ ] Remove `ThreadHistoryMode::Legacy`, legacy-only replay/display branches, and stale default fixtures after all current producers are paginated.
- [ ] Keep provider-neutral image generation, audio/media handling, platform security fallbacks, and active external-agent import behavior.
- [ ] Run the broad compatibility search and classify every remaining production match as `remove`, `retain`, or `migrate-in-place`.
- [ ] Run workspace `cargo check`, repository `just check`, and `just crate-dependency-graph` when manifests change.

## Execution status

- [x] Audit current producers, consumers, and migration boundaries.
- [x] Task 1: canonical paginated writer contract.
- [ ] Task 2: legacy-only persistence branches.
- [x] Task 3: rollout migration surface.
- [ ] Task 4: final legacy mode and audit cleanup.
