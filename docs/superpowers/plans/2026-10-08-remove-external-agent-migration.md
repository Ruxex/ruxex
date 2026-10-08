# 外部エージェント移行機能削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Genex未リリースを前提に、外部AIエージェントからの設定・MCP・Plugins・履歴移行機能を、コード・protocol・state・生成物から完全に削除する。

**Architecture:** `genex-external-agent-migration` crateとその唯一のapp-server consumerを削除し、移行専用のTUI入口、RPC、通知、state APIを同じproducer/consumer waveで除去する。移行機能と共有されているPlugins、MCP、Skills、履歴の通常機能は残す。

**Tech Stack:** Rust workspace, app-server JSON-RPC protocol, checked-in JSON/TypeScript schemas, SQLite/sqlx migrations, TUI slash commands, `just generate`。

**Spec:** `docs/superpowers/specs/2026-10-08-remove-external-agent-migration-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- This is an unreleased Genex fork; do not add compatibility shims for external users or previously created migration tables.
- Preserve model-provider authentication, ordinary MCP management/execution, Plugins, Skills, Hooks, Genex settings, and normal session history.
- Rust commands that compile or link must run through a user systemd scope with `CARGO_BUILD_JOBS=2` for the full workspace.
- Always run `just check` for the production workspace type-check.
- Run `just check --tests` after Rust code is removed or refactored.
- Run `just generate` after checked-in generated artifacts are affected and inspect the diff.
- Do not run standalone test commands for this crate-removal work.

## Review Focus

- Removing `/import` must not remove unrelated import/config commands or generic MCP/Plugin operations; audit exact enum, parser, dispatch, and test consumers.
- Removing protocol variants must remove every Rust, JSON, and TypeScript producer/consumer without leaving stale generated schema entries.
- Removing state migrations must not disturb unrelated state migration numbering or runtime initialization; retain only the historical migrations that are unrelated to this feature.
- Removing the migration crate must not remove shared Skills, Plugins, Hooks, MCP, or session-history implementations that are consumed outside the migration path.
- Existing `CF_README.md` is an untracked user file and must remain untouched.

---

## Task 1: Remove external-agent state persistence

**Files:** `crates/state/src/lib.rs`, `crates/state/src/runtime.rs`, `crates/state/src/runtime/external_agent_config_imports.rs`, its test file, `crates/state/migrations/0038_external_agent_config_imports.sql`, and `crates/state/migrations/0044_external_agent_config_imports_provider_id.sql`.

- [ ] Confirm exact state API consumers with `rg`; distinguish migration-only records from normal thread/history state.
- [ ] Remove state module declarations, re-exports, record/query/write implementation, and its tests.
- [ ] Delete the two migration SQL files; do not add a replacement `DROP TABLE` migration because Genex has no released database consumers.
- [ ] Keep shared MCP/config/plugin/session code intact; only delete code whose producer/consumer audit is migration-specific.

**Expected:** No production source or state crate declares the state import-record API; the migration crate and its consumers remain until Task 2 removes the complete feature boundary.

## Task 2: Remove app-server, protocol, and TUI migration surfaces

**Files:** `crates/app-server/src/external_agent_migration/**`, `crates/app-server/src/lib.rs`, `crates/app-server/src/message_processor.rs`, `crates/app-server/src/in_process.rs`, app-server tests; `crates/app-server-protocol/src/protocol/common.rs`, `crates/app-server-protocol/src/protocol/v2/config.rs`, protocol tests; `crates/tui/src/external_agent_config_migration/**`, `crates/tui/src/app_server_session.rs`, TUI app/event/slash-command files and tests.

- [ ] Remove app-server migration processor construction, stored field, request dispatch arms, notification handling, and migration-specific test fixtures.
- [ ] Remove the four `externalAgentConfig/*` request methods and two progress/completed notifications from protocol Rust definitions and protocol tests.
- [ ] Remove TUI `/import` enum/help/completion/dispatch/test consumers, migration screen/flow/model/render/source modules, app event, app-server client wrappers, completion state, event routing, and notification output.
- [ ] Delete `crates/external-agent-migration/**`, remove its workspace member and app-server dependency declarations, and update `Cargo.lock` after all consumers are gone.
- [ ] Verify that slash-command parser and unrelated `/config`, `/mcp`, `/skills`, `/plugin`, and `/hooks` paths remain registered.
- [ ] Remove protocol Rust types only after all workspace consumers are deleted; do not alter unrelated account, model, MCP, or config protocol types.

**Expected:** `rg` finds no production references to `ExternalAgentConfig`, `externalAgentConfig`, `ExternalAgentConfigMigration`, `OpenExternalAgentConfigMigration`, or the migration crate; Rust compilation is ready for schema regeneration.

## Task 3: Regenerate and prune checked-in protocol artifacts

**Files:** `crates/app-server-protocol/schema/json/**`, `crates/app-server-protocol/schema/typescript/**`, plus any generated files reported by `just generate`.

- [ ] Run `just generate` through the repository-required systemd scope.
- [ ] Inspect the generated diff and confirm only external-agent migration protocol definitions/imports/notifications and their transitive unused schema entries are removed.
- [ ] If generation exposes a producer/consumer mismatch, fix the Rust source of truth rather than hand-editing generated output, then rerun `just generate`.
- [ ] Run `git diff --check` and verify no unrelated generated schema changes are present.

**Expected:** Checked-in JSON and TypeScript schemas no longer expose the external-agent migration RPCs, notifications, or types, and generation is reproducible.

## Task 4: Update compatibility inventory and perform final verification

**Files:** `docs/provider-decoupling-candidates.md`, `docs/pre-release-compatibility-inventory.md`, and only directly related documentation/tests surfaced by the audit.

- [ ] Replace the previous `retain` decision for external-agent migration with an explicit `remove` decision including producer/consumer evidence and the unreleased/no-existing-table rationale.
- [ ] Remove stale documentation that describes the migration workflow as supported; do not remove documentation for ordinary MCP, Plugins, Skills, Hooks, or model-provider authentication.
- [ ] Run targeted `rg` audits for crate name, RPC names, state table/API names, TUI command/event names, and migration metrics.
- [ ] Run `just check --tests` through the required systemd scope with `CARGO_BUILD_JOBS=2`.
- [ ] Run `just check` through the required systemd scope with `CARGO_BUILD_JOBS=2`.
- [ ] Review `git diff --stat`, `git diff --check`, and `git status --short`; confirm `CF_README.md` remains untracked and unchanged.

**Expected:** Both required checks exit successfully, no migration production surface remains, generated artifacts are synchronized, and only intended files are modified.
