# Finish Single-Provider Residuals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the remaining single-provider boundary gaps identified by the repository audit, then verify, commit, and push the result.

**Architecture:** Validate the provider home before any TUI-owned local state database access. Keep test-only provider construction behind the existing test-support surface. Integrate normalized chat options into the provider-neutral chat request while retaining the OpenAI Responses wire representation inside the provider adapter.

**Tech Stack:** Rust workspace, Tokio tests, SQLite state runtime, provider contract and adapter crates.

**Spec:** `docs/superpowers/specs/2026-10-09-complete-single-provider-boundary-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers.
- Run Rust checks through the repository `just` recipes and the memory-limited user systemd scope.
- Run `just check` and `just check --tests`; do not run standalone Rust tests unless explicitly requested.

## Review Focus

- Existing and mismatched provider markers must be checked before TUI local SQLite opens.
- Cross-crate integration tests must retain access to test helpers without exposing runtime provider construction as ordinary production behavior.
- Chat Completions and Responses requests must retain their current wire behavior after contract normalization.
- Responses WebSocket continuation comparison and telemetry must remain unchanged.
- Provider metadata removed from plugin configuration must have no remaining producer or consumer.

### Task 1: TUI provider-home validation ordering

**Files:**
- Modify: `crates/tui/src/lib.rs`
- Test: `crates/tui/src/app/tests.rs` or an existing TUI startup test module

- [x] Add a regression test or a testable helper assertion for the non-embedded startup path.
- [x] Mount/validate the provider home before calling `state_db::get_state_db` for the local daemon path.
- [x] Preserve the remote path's non-owning optional local-state behavior and document the decision.
- [x] Compile the affected TUI test target with the required `just check --tests` command.

### Task 2: Test-only provider construction and dead provider metadata

**Files:**
- Modify: `crates/core/src/thread_manager.rs`, `crates/core/src/test_support.rs`
- Modify: `crates/core-plugins/src/manager.rs`, `crates/core-plugins/src/test_support.rs`, `crates/core/src/config/mod.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`

- [x] Keep provider construction used by integration tests behind the documented test-support API and record the explicit retain decision for the implementation-only constructors.
- [x] Remove `PluginsConfigInput.model_provider_id` and update all constructors and fixtures after confirming there are no consumers.
- [x] Gate or remove only the unused `ModelsEndpoint::new` convenience constructor while retaining the active adapter path.
- [x] Compile affected test targets with the required checks.

### Task 3: Provider-neutral chat request boundary

**Files:**
- Modify: `crates/model-provider/src/contract.rs`, `crates/model-provider/src/lib.rs`, `crates/model-provider/src/provider.rs`
- Modify: `crates/core/src/client.rs`, `crates/core/src/client_common_tests.rs`

- [x] Integrate the Responses-named normalized payload into `ChatRequest` and keep endpoint/wire serialization in the provider adapter.
- [x] Keep Core request comparison, telemetry, HTTP, and WebSocket continuation behavior unchanged while consuming the neutral name.
- [x] Preserve the existing Responses HTTP/WebSocket wire payload and Chat Completions behavior.
- [x] Reuse the existing request/continuation contract tests and compile them with the required checks.
- [x] Compile affected test targets with the required checks.

### Task 4: Documentation, full verification, commit, and push

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md` if audit status needs correction

- [x] Record explicit `remove`, `retain`, or `migrate-in-place` decisions for every changed residual.
- [x] Run `just check` and `just check --tests` through the required memory-limited scope.
- [x] Run `git diff --check` and inspect the complete diff.
- [x] Commit the completed changes on `main`.
- [x] Push `main` to its configured upstream.
