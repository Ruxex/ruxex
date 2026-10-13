# Extract Cloudflare Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the built-in Cloudflare provider implementation into a workspace crate while keeping `genex-model-provider` as the provider-neutral contract and allowing hosts to inject arbitrary `ModelProvider` implementations.

**Architecture:** `genex-model-provider` owns the public neutral contract and provider-home boundary. A new `genex-providers` crate owns the built-in adapter/runtime implementation, including Cloudflare and the existing configured OpenAI-compatible implementation. `genex-app-server` and host binaries compose a concrete provider and pass it through `AppServerRuntimeBuilder::new`; no runtime registry or provider switch is introduced.

**Tech Stack:** Rust workspace, Cargo path dependencies, `genex-model-provider`, `genex-api`, `genex-http-client`, `genex-login`, provider contract tests, repository `just` checks.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Keep one concrete `ModelProvider` per runtime/AppServer; another provider requires another runtime/data root.
- Keep provider HTTP, wire payloads, credential details, and provider-specific errors inside the implementation crate.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers.
- Validate the provider-home marker before credentials, SQLite, sessions, or caches are opened.
- Before changing existing provider contracts or configuration producers/consumers, record the decision in `docs/pre-release-compatibility-inventory.md`.
- Rust verification must use the repository `just` recipes through the user systemd memory scope; run `just check`, and run `just check --tests` for Rust changes. Run `just generate` if checked-in generated artifacts are affected.
- Do not run standalone test commands.

## Review Focus

- An arbitrary crate can implement `ModelProvider` and be injected without depending on a provider registry.
- Cloudflare model discovery preserves pagination, filtering, normalization, authentication, and identity caching behavior.
- Configured Responses/Chat providers retain existing streaming, WebSocket, auth lifecycle, and model-catalog behavior after moving to `genex-providers`.
- Provider-home mounting still uses the injected provider descriptor and happens before credential loading.
- `genex-model-provider` no longer contains a Cloudflare-specific factory branch or imports a built-in provider implementation.

### Task 1: Establish the workspace implementation crate and public composition seam

**Files:**
- Create: `crates/providers/Cargo.toml`
- Create: `crates/providers/src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `crates/model-provider/src/lib.rs`
- Modify: `crates/model-provider/src/contract.rs`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Test: provider contract and runtime-builder injection tests

**Interfaces:**
- Produce `genex_providers` as the workspace crate for built-in provider implementations.
- Keep `genex_model_provider::ModelProvider`, `SharedModelProvider`, and all neutral request/response types usable by external workspace crates.
- Keep `AppServerRuntimeBuilder::new(SharedModelProvider)` as the host injection entry point.

- [x] Add a compile-level contract test proving a provider implemented outside `genex-model-provider` can be stored as `SharedModelProvider`.
- [x] Add the new workspace crate, its dependency direction, and the explicit `migrate-in-place` inventory decision.
- [x] Implement the minimal crate root and exports needed by later tasks; do not add a registry or runtime switch.
- [x] Run the affected checks and confirm the contract test passes.

### Task 2: Move shared provider adapter runtime into `genex-providers`

**Files:**
- Create/modify: `crates/providers/src/provider.rs`
- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`
- Modify: `crates/model-provider/src/models_identity.rs`
- Modify: `crates/model-provider/src/contract.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Modify: `crates/model-provider/Cargo.toml`
- Test: moved configured-provider and adapter contract tests

**Interfaces:**
- `genex-providers` owns the private adapter runtime, wire projection, transport, auth, and catalog helpers.
- `genex-model-provider` retains the neutral contract/home/catalog-source APIs and the already-public provider-support helpers required by core and hosts.
- Built-in constructors return `genex_model_provider::SharedModelProvider`.

- [x] Add/relocate tests that assert configured provider construction still exposes the same descriptor, request format, auth snapshot, and unsupported capability behavior.
- [x] Move the common adapter implementation into `genex-providers`, making only the contract-facing APIs public.
- [x] Update imports and dependency edges so `genex-model-provider` does not depend on `genex-providers`.
- [x] Run the affected checks and confirm configured-provider behavior is unchanged.

### Task 3: Extract Cloudflare and host-side construction

**Files:**
- Create: `crates/providers/src/cloudflare.rs`
- Modify: `crates/providers/src/lib.rs`
- Modify: `crates/providers/src/provider.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Modify: `crates/app-server/src/runtime.rs`
- Modify: host Cargo manifests/imports that construct the default provider
- Test: Cloudflare adapter tests and AppServer runtime descriptor/mount tests

**Interfaces:**
- Produce `genex_providers::cloudflare::CloudflareModelProvider` and a constructor suitable for host composition.
- The constructor must expose a provider descriptor without opening credential state, so the host can validate `genex-home.json` first.
- Hosts construct the provider implementation and inject it through `AppServerRuntimeBuilder::new`.

- [x] Add Cloudflare crate tests for model-search normalization and descriptor construction independent of the old private module.
- [x] Move the Cloudflare implementation and tests, preserving the Cloudflare endpoint, environment variables, pagination, model filtering, and normalized catalog identity.
- [x] Remove the Cloudflare module and implementation branch from `genex-model-provider`; retain the provider metadata default there because configuration resolution must not depend on the implementation crate.
- [x] Route default host startup through `genex-providers` and the injected runtime seam; preserve marker-before-credential ordering.
- [x] Run the affected checks and confirm the Cloudflare and runtime tests pass.

### Task 4: Final audit and repository verification

**Files:**
- Modify: `docs/provider-specific-crates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: generated artifacts only if `just generate` changes them
- Verify: all provider imports, dependency graph, and workspace status

- [x] Search production code for the old Cloudflare module and factory branch, and remove all stale consumers.
- [x] Document that `genex-providers` contains built-ins while arbitrary providers are injected by the host, not registered at runtime.
- [x] Run `just check` in the repository-required memory-limited systemd scope.
- [x] Run `just check --tests` in the same scope.
- [x] Run `just generate` and inspect the diff.
- [x] Run `git diff --check`, inspect the full diff, and verify the worktree status.
