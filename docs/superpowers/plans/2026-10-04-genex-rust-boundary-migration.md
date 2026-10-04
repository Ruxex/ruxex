# Genex Rust Boundary Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove repository-owned `Codex`/`codex` identity from Rust source, tests, schemas, persisted projections, and runtime metadata while keeping unavoidable external endpoint and third-party contracts isolated at explicit adapters.

**Architecture:** Internal Rust names, repository-owned protocol/schema keys, child-process environment variables, persisted state projections, telemetry/originator values, and test fixtures use `Genex` or provider-neutral names. OpenAI Responses-compatible input/output names remain only in adapter structs/constants with explicit `serde(rename = ...)` or boundary comments. Third-party MCP/gRPC/package/security identifiers are retained only where their producer/consumer audit proves they are external contracts.

**Tech Stack:** Rust workspace, serde/schema generation, app-server protocol, rollout/state persistence, OTEL metadata, and Rust integration tests.

**Spec:** `AGENTS.md`, `docs/provider-decoupling-plan.md`, `docs/provider-decoupling-candidates.md`, and `docs/pre-release-compatibility-inventory.md`.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- The pre-release fork has no external users, so repository-owned API/protocol/schema/persisted/CLI/config/UI names may migrate together after producer/consumer audits.
- Preserve OpenAI endpoint wire names only at the external adapter boundary: endpoint URLs, request/response fields, authentication variables/headers, and model identifiers that the endpoint requires.
- Preserve third-party protocol/package names and OS/security identifiers only when changing them would change an external contract or strand state; record those exact names in the compatibility inventory.
- Record each decision as `remove`, `retain`, or `migrate-in-place` before editing the corresponding production code.
- Run `just generate` when schemas or checked-in generated artifacts are affected, `just check --tests` for every Rust change, and `just check` for the final production workspace check. Do not run standalone tests.

## Review Focus

- External Responses-compatible fields must remain available at the adapter boundary while internal `TokenUsage` and related projections become provider-neutral.
- Child-process environment variables must be renamed consistently across allowlists, injection, scrubbing, shell snapshots, and app-server/exec-server consumers.
- Persisted rollout/state and app-server schema producers and readers must migrate together; no old internal key may survive only in a fixture.
- OTEL and originator values must be migrated when repository-owned, while external client identity values remain explicitly allowlisted.
- Remaining Rust `codex` matches must be either an explicit external boundary or an audited historical/security/test contract; no unclassified internal match may remain.

---

### Task 1: Establish the Rust residual audit and decisions

**Files:**
- Create: `scripts/audit_genex_rust_names.py`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

- [x] Define the audit's repository-owned migration set and explicit external allowlist. The audit must print path, line, and matched token for every failure and exit nonzero when an unclassified `codex` token remains.
- [x] Run the audit before implementation and record the expected failing result as the RED test.
- [x] Record the new boundary rule: internal persisted/protocol/telemetry names migrate; OpenAI/third-party/security names remain only at adapters or explicit compatibility boundaries.

### Task 2: Migrate runtime environment and process-boundary names

**Files:**
- Modify: `crates/protocol/src/shell_environment.rs`
- Modify: `crates/core/src/exec_env.rs`, `crates/core/src/spawn.rs`, and `crates/core/src/unified_exec/**`
- Modify: `crates/exec-server/**`, `crates/app-server/**`, `crates/cli/**`, and all Rust consumers of the renamed constants
- Test: existing Rust environment, sandbox, exec-server, app-server, and CLI test targets

- [x] Rename repository-owned `CODEX_*` runtime variables and Rust constants to `GENEX_*`, keeping any external auth or security state explicitly isolated.
- [x] Update environment inheritance, scrubbing, injection, command construction, test fixtures, and snapshots together.
- [x] Run the audit and `just check --tests`; record any retained OS/security variable with its consumer evidence.

### Task 3: Migrate repository-owned protocol, persisted, metadata, and MCP names

**Files:**
- Modify: `crates/protocol/**`, `crates/app-server-protocol/**`, `crates/core/**`, `crates/state/**`, `crates/rollout/**`, `crates/thread-store/**`, and `crates/genex-api/**`
- Modify: `crates/tools/**`, `crates/rmcp-client/**`, `crates/tui/**`, and affected integration tests
- Test: protocol, rollout/state, app-server, MCP, and TUI test targets

- [x] Rename repository-owned metadata keys, approval markers, app namespaces, rollout budget fields, and internal protocol aliases to `genex_*`/provider-neutral forms.
- [x] Keep OpenAI Responses input/output spelling in dedicated adapter structs using explicit serde mappings; map into the new internal types immediately after decoding.
- [x] Migrate persisted readers and writers together and regenerate app-server/schema artifacts where applicable.

### Task 4: Migrate telemetry, originators, CLI/runtime labels, and test fixtures

**Files:**
- Modify: `crates/otel/**`, `crates/exec/**`, `crates/app-server-daemon/**`, `crates/cli/**`, `crates/tui/**`, `crates/core-plugins/**`, and remaining Rust consumers
- Test: OTEL, exec, daemon, CLI, TUI, and plugin test targets

- [x] Rename repository-owned telemetry metric/event prefixes, originator values, diagnostics, and helper identifiers to Genex/provider-neutral names.
- [x] Retain only external client-originator values, model identifiers, upstream URLs, and OS/security names that are proven contracts.
- [x] Update Rust snapshots and fixture data after producer changes; do not hand-edit generated schemas.

### Task 5: Verify, review, and close the residual ledger

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: generated files only through `just generate`

- [x] Run the Rust audit and confirm every remaining match is explicitly allowlisted with a reason.
- [x] Run `just generate`, inspect the complete diff, then run `just check --tests`, `just check`, and `git diff --check`.
- [x] Perform a final source/diff review for accidental changes to OpenAI endpoint compatibility and security behavior.

### Task 6: Expand the repository-owned boundary migration

The follow-up audit confirmed that the plugin overlay, bundled runtime namespaces,
Responses-side generic headers/usage field, and selected authentication names are
repository-owned in this unreleased fork. This task supersedes the earlier provisional
retention of those exact values; external plugin ecosystem paths, provider residency and
inference trace headers, endpoint URLs, model identifiers, and security namespaces remain
outside this task.

- [x] Migrate `.codex-plugin` to `.genex-plugin` across plugin loaders, protocol discovery,
  app-server/CLI consumers, and fixtures.
- [x] Migrate `codex-bundled*`, `codex-primary-runtime`, and `codex-runtimes` namespaces
  and cache/runtime paths to their `genex-*` equivalents.
- [x] Migrate Responses-owned `codex_rollout_budget_units`, `x-codex-*` request/response
  names, and Responses-specific internal metadata/header names to `genex` values.
- [x] Migrate the repository-owned auth environment/keyring names to `GENEX_*` while
  preserving token semantics, redaction, and security behavior.
- [x] Run generation and workspace checks, inspect the residual scan, and review the diff.
