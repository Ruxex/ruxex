# Genex Internal Surfaces Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate repository-owned backend paths, OS security identifiers, internal test/path vocabulary, and Genex-owned UI/update text while retaining external MCP/VS Code, GitHub, provider, and protocol contracts.

**Architecture:** Update each repository-owned producer and consumer together. Backend routes and internal fixtures move to `genex`; external endpoint paths, client originators, GitHub identities, and third-party protocol values remain unchanged.

**Tech Stack:** Rust workspace, app-server routes, platform security/config identifiers, TUI/UI strings, fixtures, snapshots, `rg`, `just generate`, `just check --tests`, and `just check`.

**Spec:** Approved in chat on 2026-10-04; repository direction in `AGENTS.md`, `docs/provider-decoupling-plan.md`, and the compatibility inventory.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Migrate repository-owned API/wire paths and UI names; preserve external OpenAI endpoint paths and third-party protocol names.
- Migrate repository-owned OS security and sandbox names; preserve only identifiers proven to be external contracts.
- Migrate internal test-only paths and fixture labels unless they intentionally model an external value.
- Update producers, consumers, tests, snapshots, and documentation together.
- Run `just generate`, `just check --tests`, and `just check`; do not run standalone tests by default.

## Review Focus

- A backend route must not be changed on only one side of a producer/consumer pair.
- OS security names must be changed without weakening trust, sandbox, or enforcement behavior.
- External GitHub, MCP, VS Code, and provider values must remain byte-for-byte stable.
- Test-only renames must not alter fixtures that intentionally exercise external legacy values.
- UI update instructions must point at the Genex-owned distribution surface, not stale Codex package names.

### Task 1: Migrate repository-owned backend paths

**Files:**
- Modify: app-server and API route producers/consumers found by `rg` for `/api/codex`, `/backend-api/codex`, and repository-owned analytics paths
- Modify: matching request tests and fixtures
- Modify: compatibility inventory and candidate ledger

- [x] Audit each route's producer and consumer, retaining OpenAI/provider wire paths.
- [x] Rename repository-owned routes and update all request assertions/fixtures together.
- [x] Compile-check affected Rust targets and record the decision evidence.

### Task 2: Migrate OS security and sandbox identifiers

**Files:**
- Modify: `crates/windows-sandbox-rs`, `crates/sandboxing`, `crates/config`, `crates/hooks`, `crates/user-verification`, and dependent tests discovered by the audit
- Modify: platform/security snapshots and diagnostics where applicable

- [x] Rename owned package, private desktop, WFP, provisioning, managed-preference, and system-config identifiers.
- [x] Preserve security predicates, access behavior, and external signing/provider identifiers.
- [x] Update platform fixtures and compile-check all affected targets.

### Task 3: Migrate internal test, path, and UI vocabulary

**Files:**
- Modify: internal `codex-rs`, `codex-test`, `codex-workspace`, temporary-path, and fixture labels that do not model an external value
- Modify: TUI/app-server user-facing text, update prompts, package instructions, and snapshots
- Retain: GitHub URLs, external package identifiers, MCP/VS Code identities, Product::Codex, and external protocol fixtures

- [x] Rename internal-only paths and test labels to Genex equivalents.
- [x] Rename Genex-owned UI/update text and retain external package/distribution contracts.
- [x] Review each retained UI/external value and document why it remains.

### Task 4: Final audit and verification

- [x] Run `just generate`, `just check --tests`, `just check`, the repository-owned-name audit, and `git diff --check`.
- [x] Recount all Rust `codex` matches and classify retained external values versus remaining repository-owned candidates.
- [x] Update the plan and compatibility ledgers with producer/consumer evidence.
