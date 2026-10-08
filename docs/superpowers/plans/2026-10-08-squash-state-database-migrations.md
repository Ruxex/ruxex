# Squash State Database Migrations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 61 historical SQLite migration files with five version-1 baselines while proving that each baseline produces the same final schema, indexes, triggers, and stable initial data as the current migration chain.

**Architecture:** Keep the five existing `sqlx::Migrator` boundaries and directory paths, but replace each directory's history with one `0001_initial.sql`. Remove only compatibility code and tests that exist solely for the deleted migration history; retain current runtime migrator tolerance and all current database APIs.

**Tech Stack:** Rust, `sqlx::migrate!`, SQLite DDL, Tokio/sqlx migration tests, Python `sqlite3` for the pre-deletion equivalence probe.

**Spec:** [`docs/superpowers/specs/2026-10-08-squash-state-database-migrations-design.md`](../specs/2026-10-08-squash-state-database-migrations-design.md)

## Global Constraints

- This is an unreleased Genex fork; old development databases are disposable and are not migrated automatically.
- Preserve the five existing migrator boundaries and current state/logs/goals/queue/thread-history runtime APIs.
- Compare the old SQL chain from the starting HEAD and each new baseline before committing the history deletion; any schema or stable-data difference blocks the migration commit.
- Dynamic timestamp values such as `backfill_state.updated_at` are compared by generation expression and execution-time range, not byte-for-byte.
- Work directly on `main`; do not create a worktree.
- Rust build/check commands must use the repository `just` recipes and a user systemd scope with `CARGO_BUILD_JOBS=2` for workspace checks.
- Always run `just check`; because Rust tests are changed, also run `just check --tests`.
- Do not run standalone test commands unless explicitly requested.

## Review Focus

- State baseline omits a table or column still read by production code; test the complete current table/column set.
- Final index definitions lose sort order or partial-index predicates; compare normalized `sqlite_master` definitions and assert key indexes.
- Timestamp and recency triggers are missing or attached to the wrong column; insert/update rows and assert derived values.
- Initial pinned-section or backfill rows are absent; assert stable seed values and the dynamic timestamp invariant.
- Auxiliary logs, goals, queue, or thread-history baselines diverge from their current final schemas; apply all five migrators and inspect every object family.

### Task 1: Add final-schema baseline migrations and prove equivalence against HEAD history

**Files:**
- Create: `crates/state/migrations/0001_initial.sql`
- Create: `crates/state/logs_migrations/0001_initial.sql`
- Create: `crates/state/goals_migrations/0001_initial.sql`
- Create: `crates/state/queue_migrations/0001_initial.sql`
- Create: `crates/state/thread_history_migrations/0001_initial.sql`
- Delete: the historical SQL files from the five migration directories, leaving only the new baselines

**Interfaces:**
- Consumes: The existing SQL files in the five migration directories.
- Produces: One direct final-schema migration per existing `sqlx::Migrator` directory, all at version 1.

- [ ] **Step 1: Write the five baseline SQL files** with direct final table definitions, constraints, indexes, triggers, and required initial rows. Do not include obsolete dropped tables or historical data backfills that are no-ops on a fresh database.
- [ ] **Step 2: Delete the historical SQL files** from the five directories, leaving exactly one `0001_initial.sql` in each.
- [ ] **Step 3: Run the equivalence probe before committing** using `git show HEAD:<old-path>` to load the old sorted SQL chain into one in-memory SQLite connection and the new `0001_initial.sql` into another; normalize and compare `sqlite_master` tables/indexes/triggers and initial rows, normalizing only dynamic timestamps. Expected: zero differences for all five series.
- [ ] **Step 4: Inspect the probe output and file counts.** Expected: exactly five SQL migration files total, one per directory. If SQLite normalization reveals an intentional equivalent representation, record the ruling in the execution ledger and update the spec before proceeding.
- [ ] **Step 5: Commit the baseline SQL, history deletion, and successful equivalence evidence.**

### Task 2: Remove historical-only runtime repair and replace migration tests

**Files:**
- Modify: `crates/state/src/migrations.rs`
- Modify: `crates/state/src/sqlite.rs`
- Modify: `crates/state/src/migrations_tests.rs`
- Modify: the five baseline files only if Task 1's schema contract test reveals a missed current object

**Interfaces:**
- Consumes: Task 1's five version-1 baselines and the existing runtime migrator functions.
- Produces: Runtime initialization that runs the same five migrators without the obsolete version-39 repair path; tests that assert baseline shape and behavior rather than deleted history versions.

- [ ] **Step 1: Remove `repair_legacy_recency_migration_version` and its `SqlitePool`-specific helper dependency** from `migrations.rs`, and remove its state-only call from `sqlite.rs`. Keep `runtime_migrator` and its `ignore_missing` behavior unchanged.
- [ ] **Step 2: Replace old-version migration tests** in `migrations_tests.rs` with a baseline contract test that asserts all five embedded migrators have exactly one migration at version 1 and that a fresh state DB contains the complete current object set, pinned seed, backfill seed, indexes, and timestamp/recency trigger behavior. Remove tests for version 44/50/51/37/38 transitions, agent-job table upgrade, and recency-history repair; retain tests for current section ordering, attachment schema behavior, thread-history update defaults, and runtime newer-version tolerance where they still exercise current behavior against a fresh baseline.
- [ ] **Step 3: Run the focused test-target compilation check.** Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests -p genex-state`. Expected: exit 0. Per repository policy, do not run standalone Rust tests.
- [ ] **Step 4: Commit the runtime cleanup and test replacement.**

### Task 3: Record the compatibility decision and run repository verification

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Verify: `docs/provider-decoupling-candidates.md` for any newly discovered provider-specific behavior
- Verify: all five migration directories and production version references

**Interfaces:**
- Consumes: Task 1's equivalence evidence and Task 2's final source/test state.
- Produces: An auditable `remove`/`migrate-in-place` decision and a clean verified branch.

- [ ] **Step 1: Add the producer/consumer audit row** marking historical migration files and old-DB automatic conversion as `remove`, and the five current baseline schemas as `migrate-in-place`, with the unreleased/disposable-DB rationale and exact verification commands.
- [ ] **Step 2: Run repository audits** for old migration version assumptions, old migration filenames, and references to the removed recency repair helper. Expected: no production references remain; only documentation/spec/plan historical references may mention the old history.
- [ ] **Step 3: Run `git diff --check` and inspect the complete diff.** Expected: only the five baselines, migration runtime/tests, inventory, and plan/spec documentation are changed.
- [ ] **Step 4: Run the required workspace type-check:** `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check`. Expected: exit 0.
- [ ] **Step 5: Run the required test-target compilation check:** `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests`. Expected: exit 0.
- [ ] **Step 6: Run `just generate` only if the diff or generator dependency audit shows checked-in generated artifacts are affected; otherwise record that no generated artifact consumes SQLite migrations.**
- [ ] **Step 7: Commit the inventory and final verification documentation.**
