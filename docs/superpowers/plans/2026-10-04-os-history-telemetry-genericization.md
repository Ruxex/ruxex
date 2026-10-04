# OS, History, and Telemetry Genericization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove repository-owned Codex identity from platform security diagnostics, persisted history metadata, and telemetry while retaining external repository, endpoint, protocol, and Product::Codex contracts.

**Architecture:** Migrate the producer and consumer sides of each owned identifier together. Platform security keeps the trust and enforcement predicates but uses Genex package, bundle, executable, and user-facing identifiers. History and telemetry use Genex-owned keys/names; external GitHub URLs, OpenAI wire names, and Product::Codex remain unchanged.

**Tech Stack:** Rust workspace, serde/JSONL history, OpenTelemetry metrics/events, platform doctor checks, `rg`, `just generate`, `just check --tests`, and `just check`.

**Spec:** `docs/provider-decoupling-candidates.md`, `docs/pre-release-compatibility-inventory.md`, and the repository direction in `AGENTS.md`.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Change only repository-owned identifiers; retain external GitHub/upstream, endpoint wire, third-party protocol, and Product::Codex names.
- Preserve security predicates and trusted-process boundaries while changing product identifiers.
- This is a pre-release repository with no external users; no compatibility reader is required for the migrated owned history or telemetry names.
- Always run `just check`; run `just check --tests` for Rust changes; run `just generate` when generated artifacts may change.
- Do not run standalone tests unless explicitly requested.

## Review Focus

- A legacy Codex executable or package must not become trusted after the security rename; platform tests retain explicit untrusted cases.
- Genex package and bundle identifiers must be recognized consistently by platform parsing, event matching, output, and snapshots.
- History metadata producers and consumers must use the same Genex key; external GitHub attachment identities must remain unchanged.
- Telemetry metric scopes, event attributes, and auth-environment fields must be renamed consistently across producers and tests.
- Existing generic security/telemetry behavior must remain unchanged apart from naming.

### Task 1: Genericize platform security identity and diagnostics

**Files:**
- Modify: `crates/cli/src/doctor/desktop/macos_security.rs`
- Modify: `crates/cli/src/doctor/desktop/windows_security.rs`
- Modify: `crates/cli/src/doctor/desktop/platform.rs`
- Modify: `crates/cli/src/doctor/security.rs`, `crates/cli/src/doctor/output.rs`
- Modify: adjacent desktop/security tests and snapshots

**Interfaces:**
- Consumes: OS security events, package/bundle metadata, and executable paths.
- Produces: Genex-owned platform identity and diagnostics with the same trust and enforcement semantics.

- [x] Add/adjust tests proving Genex identifiers are recognized and legacy/decoy identifiers are not trusted.
- [x] Migrate package family, bundle ID, app/helper names, event predicates, summaries, remediation text, and snapshots.
- [x] Run the platform/security check-target compilation and inspect residual matches.

### Task 2: Migrate repository-owned persisted history metadata

**Files:**
- Modify: `crates/app-server/tests/suite/v2/thread_archive.rs`
- Modify: `crates/core/src/context/world_state/mod.rs` and its tests if the persisted hash namespace is covered
- Modify: other history/state producers or fixtures discovered by the audit
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: rollout/thread archive metadata and world-state persistence.
- Produces: Genex-owned persisted metadata names; external GitHub repository/attachment data remains byte-for-byte unchanged.

- [x] Update the owned archive metadata key and world-state namespace together with their fixtures.
- [x] Verify no production reader still expects the old owned key; retain Product::Codex and external Git fixture values.
- [x] Run affected Rust test-target compilation and generated-artifact checks if applicable.

### Task 3: Genericize telemetry scopes, metrics, and auth-environment attributes

**Files:**
- Modify: `crates/otel/src/metrics/client.rs`, `crates/otel/src/metrics/tags.rs`
- Modify: `crates/otel/src/events/session_telemetry.rs`
- Modify: `crates/login/src/auth_env_telemetry.rs`, `crates/model-provider/src/models_endpoint.rs`
- Modify: OTEL tests, README examples, and internal metric fixtures
- Modify: `docs/provider-decoupling-candidates.md`, `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: OTEL metric/event producers and authentication-environment telemetry.
- Produces: `genex` telemetry scope/names and Genex-owned auth attributes with unchanged values and routing behavior.

- [x] Rename the meter scope, repository-owned metric/tag names, and auth-environment fields consistently.
- [x] Update all in-repository producers/consumers and assertions; do not change external API headers or Product::Codex values.
- [x] Run OTEL and workspace check-target verification.

### Task 4: Final audit and verification

**Files:**
- Modify: this plan and the compatibility ledger
- Modify: generated files only if `just generate` produces changes

- [x] Record `migrate-in-place` decisions with producer/consumer evidence.
- [x] Run `just generate`, `just check --tests`, `just check`, the repository-name audit, and `git diff --check`.
- [x] Classify residual `codex` matches as external contract, Product::Codex, third-party protocol, historical fixture, or a separately tracked follow-up.
