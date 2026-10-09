# Remove Core Authentication Bridges Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove Core's direct provider-auth object dependencies while preserving provider-owned authentication, telemetry, plugin discovery, and managed-auth behavior.

**Architecture:** Core consumes `ModelProvider::auth_state()` for non-secret account/status telemetry and `ModelProvider::request_snapshot()` for request-scoped managed-auth state. Provider-specific `CredentialManager` and `GenexAuth` remain inside the adapter; dead auth arguments are removed from plugin discovery and installation paths.

**Tech Stack:** Rust workspace, `ModelProvider`, `ProviderAuthState`, `ProviderRequestSnapshot`, Tokio, repository `just` verification recipes.

**Spec:** `docs/superpowers/specs/2026-10-09-complete-single-provider-boundary-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Core production auth decisions and telemetry must not read `ModelProviderInfo`, `GenexAuth`, or provider-internal auth objects; `CredentialManager` may remain as an injected host/adapter service where it supplies environment metadata or MCP/plugin plumbing, but Core must not read its provider-auth payload for these decisions.
- Provider credentials remain private to the injected Provider adapter; only non-secret auth state and request snapshots cross the boundary.
- Preserve the existing managed-auth fallback behavior and plugin discovery behavior.
- Rust verification must use the memory-limited systemd scope with `just check` and `just check --tests`; standalone tests are not the default.
- Update the compatibility inventory before changing an existing producer/consumer contract.

## Review Focus

- Unauthenticated, invalid, and authenticated Provider states produce the correct Core telemetry without reading raw credentials.
- Managed-auth request snapshots preserve the previous-model fallback gate after removing direct CredentialManager access.
- Plugin discovery and installation remain behaviorally unchanged after removing unused auth parameters.
- Provider account IDs remain non-secret telemetry metadata and are not persisted into thread/rollout routing fields.
- Test-only Provider construction remains isolated from production runtime construction.

### Task 1: Record the Core auth-boundary residuals

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: this plan

- [x] Record the direct Core telemetry, managed-auth fallback, and dead plugin-auth parameter findings with producer/consumer evidence and `remove` or `migrate-in-place` decisions.
- [x] Record `Config.model_provider*` and test-only Provider construction as explicit `retain` decisions.

### Task 2: Make session telemetry consume Provider auth state

**Files:**
- Modify: `crates/core/src/session/session.rs`
- Test: existing Core telemetry compile-only targets and provider contract fake coverage

- [x] Add compile-only coverage for `ProviderAuthState` authentication status/account metadata in session telemetry.
- [x] Replace direct `CredentialManager::auth()` / `GenexAuth` telemetry mapping with `provider.auth_state()` and neutral status mapping.
- [x] Keep environment metadata sourced from `ModelProvider::auth_environment`.
- [x] Run the affected compile-only target.

### Task 3: Remove direct auth from turn fallback and plugin discovery

**Files:**
- Modify: `crates/core/src/session/turn.rs`
- Modify: `crates/core/src/plugins/discoverable.rs`
- Modify: `crates/core-plugins/src/discoverable.rs`
- Modify: `crates/core/src/tools/handlers/request_plugin_install.rs`
- Test: Core/plugin compile-only targets and existing discovery/install coverage

- [x] Add compile-only coverage for Provider request-snapshot managed-auth gating and auth-free plugin discovery.
- [x] Replace `CredentialManager::current_auth_uses_managed_auth` with `provider.request_snapshot().auth.managed_auth`.
- [x] Remove the unused `GenexAuth` parameter from tool-suggest discovery and plugin-install verification paths.
- [x] Run the affected compile-only targets.

### Task 4: Final audit and verification

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md` if final evidence requires clarification

- [x] Verify Core production has no direct `GenexAuth` reads for provider auth, no direct `CredentialManager` auth reads, and no production Provider construction outside documented host/config boundaries.
- [x] Verify retained `Config.model_provider*` and test-only construction are documented.
- [x] Run `just fmt-check`, `just check`, `just check --tests`, and `git diff --check`.
- [x] Inspect the complete diff and leave generated artifacts unchanged unless a schema/protocol change is found.
