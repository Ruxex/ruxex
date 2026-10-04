# Genex Residual Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Genex 化後に残るリポジトリ所有の内部名、製品向け表示、テスト名、MCP 適合性ハーネス名を整理し、妥当な外部契約だけを Codex 名のまま保持する。

**Architecture:** OpenAI API の認証・wire・モデル識別子、第三者プロトコル、OS/security 契約は保持する。リポジトリ内だけで完結する表示、fixture、テスト支援名、ハーネスのローカル変数・環境変数を Genex に移行する。Apps 残骸は producer/consumer を確認し、内部テストなら一般化、保存・外部契約なら保持する。

**Tech Stack:** Rust workspace, Python MCP conformance harness, Markdown, checked-in generated artifacts.

**Spec:** `AGENTS.md`, `docs/provider-decoupling-plan.md`, `docs/provider-decoupling-candidates.md`, `docs/pre-release-compatibility-inventory.md`.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Preserve OpenAI endpoint URLs, request/response fields, authentication variables/headers, and model identifiers.
- Preserve external protocol/package/upstream/OS/security identifiers unless producer/consumer audit proves they are repository-owned.
- Record every residual decision as `remove`, `retain`, or `migrate-in-place` in the compatibility inventory before changing code.
- Run `just check`; run `just check --tests` for Rust changes; run `just generate` when generated artifacts are affected.
- Do not run standalone tests for this cleanup.

## Review Focus

- Apps names in unregistered test sources must not be mistaken for active production capability.
- `codex_app` namespaces in persisted history or dynamic-tool compatibility paths must not be renamed without a reader/consumer audit.
- MCP conformance harness names must distinguish official external scenario names from local Genex implementation names.
- CLI documentation and binary names must agree with the actual Genex build target.
- Generated/Hawk artifacts must either be refreshed or explicitly labeled historical and excluded from migration counts.

### Task 1: Record the residual-name decisions

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

- [ ] Add an explicit row for repository-owned product-facing names, conformance-harness local names, and stale internal test names as `migrate-in-place`.
- [ ] Add an explicit retain row for API/auth/model/protocol/security/upstream names, including `CODEX_API_KEY`, `CODEX_ACCESS_TOKEN`, `codex.code_mode.v1`, `x-codex-*`, and `gpt-*-codex` identifiers.
- [ ] Reconcile the path-count methodology so `.codex/` agent infrastructure and external fixtures are not silently mixed with product paths.

### Task 2: Migrate product-facing and local harness names

**Files:**
- Modify: `README.md`
- Modify: `scripts/mcp_conformance/*.py`
- Modify: `scripts/mcp_conformance/README.md`

- [ ] Rename repository-local executable, home-directory, parameter, and environment names to Genex equivalents.
- [ ] Preserve official MCP scenario terminology and external auth variables where they are part of the conformance contract.
- [ ] Update user-facing commands and paths to match the current `genex-cli` binary.

### Task 3: Resolve stale Apps test names and suite coverage

**Files:**
- Modify: `crates/core/tests/all.rs`
- Modify: `crates/app-server/tests/all.rs`
- Modify: affected files under `crates/core/tests/suite/`, `crates/app-server/tests/suite/`, and `crates/tui/src/chatwidget/tests/`

- [ ] Audit each remaining Apps reference by producer/consumer.
- [ ] Rename internal-only Apps fixture, namespace, and helper names to provider-neutral Genex/MCP names.
- [ ] Preserve any serialized or external namespace that is still read for compatibility.
- [ ] Register only suites that are intended to remain active; remove no source file unless its producer/consumer audit records `remove` first.

### Task 4: Refresh generated artifacts and verify

**Files:**
- Modify: generated files only if regeneration changes them.
- Modify: compatibility inventory with final evidence.

- [ ] Run `just generate` and inspect the complete diff.
- [ ] Run `just check` and `just check --tests`.
- [ ] Run `git diff --check` and verify no unclassified repository-owned `codex` names remain.

