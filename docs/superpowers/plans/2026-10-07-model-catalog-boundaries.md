# Model Catalog Boundaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move model discovery behind provider-neutral catalog types and keep caching/catalog lifecycle management outside `ModelProvider`.

**Architecture:** Rename the wire-facing Rust catalog wrapper to `ModelCatalog`, introduce `ModelCatalogSnapshot` for fetched catalogs plus identity/ETag metadata, and rename the manager-facing source trait to `ModelCatalogSource`. Remove manager-construction methods from `ModelProvider`; construct remote/static managers at the shared composition boundary using a `ProviderModelCatalogSource` adapter.

**Tech Stack:** Rust workspace, `genex-protocol`, `genex-models-manager`, `genex-model-provider`, Tokio, wiremock, `just check`, and `just check --tests`.

**Spec:** `docs/superpowers/specs/2026-10-07-model-catalog-boundaries.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Preserve the JSON `{ "models": [...] }` wire shape and persisted cache schema.
- Preserve Cloudflare pagination, auth, model normalization, and API-key discovery behavior.
- Run `just check`; run `just check --tests` for all Rust changes.
- Run Rust commands through the repository's user systemd memory scope with `CARGO_BUILD_JOBS=4` for crate-local work and `2` for workspace checks.
- Leave the pre-existing untracked `CF_README.md` untouched.

## Review Focus

- The `ModelsResponse` Rust rename must not alter serialized JSON or configuration behavior; cover protocol serialization tests.
- A changed provider/account/auth identity must not reuse a prior catalog cache; preserve identity tests.
- A stable ETag must preserve the existing TTL-refresh path; preserve manager ETag tests.
- Cloudflare and My Provider must still use provider-specific discovery while the manager remains generic; cover provider and source adapter tests.
- Configured static catalogs must still bypass remote discovery; cover shared manager construction tests.

### Task 1: Rename catalog values and fetched snapshot metadata

**Files:**
- Modify: `crates/protocol/src/model_types.rs`
- Modify: all Rust consumers found by `rg -l '\bModelsResponse\b' crates`
- Modify: `crates/models-manager/src/manager.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/cloudflare_provider.rs`
- Modify: `crates/model-provider/src/my_provider.rs`
- Test: existing protocol/model-manager/provider tests

**Interfaces:**
- Produces `ModelCatalog` with the existing serialized `{models: [...]}` shape.
- Produces `ModelCatalogSnapshot { catalog: ModelCatalog, identity: String, etag: Option<String> }` for fetched model catalogs.

- [x] Write/update serialization and manager assertions so the renamed `ModelCatalog` preserves the existing wire shape.
- [x] Run the focused model-provider/model-manager checks and observe failures from the missing new names before implementation.
- [x] Rename `ModelsResponse` to `ModelCatalog` everywhere, rename `ModelsEndpointResponse` to `ModelCatalogSnapshot`, and update conversions while keeping cache fields and JSON unchanged.
- [x] Run the focused checks and confirm protocol serialization and existing cache behavior pass.

### Task 2: Rename and clarify the manager-facing source abstraction

**Files:**
- Modify: `crates/models-manager/src/manager.rs`
- Modify: `crates/models-manager/src/manager_tests.rs`
- Modify: `crates/models-manager/src/cache.rs` only if documentation references endpoint-specific terminology
- Modify: all `ModelsEndpointClient` implementations and imports

**Interfaces:**
- Produces `pub trait ModelCatalogSource` with the existing identity, auth capability, remote support, and `list_models` methods.
- `RemoteModelsManager` consumes `Arc<dyn ModelCatalogSource>`.

- [x] Add/update a manager test using a `ModelCatalogSource` test double and assert catalog snapshots are applied unchanged.
- [x] Run the focused manager test target and confirm the old trait name is the expected compile failure.
- [x] Rename `ModelsEndpointClient` to `ModelCatalogSource`, update manager fields/constructors/docs, and preserve all cache/ETag behavior.
- [x] Run the focused manager checks and verify identity mismatch and stable-ETag tests pass.

### Task 3: Make `ModelProvider` own discovery only

**Files:**
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/my_provider.rs`
- Modify: `crates/model-provider/src/cloudflare_provider.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Test: `crates/model-provider/src/provider.rs`, `my_provider.rs`, and `cloudflare_provider.rs`

**Interfaces:**
- `ModelProvider::list_models(&self, request: ListModelsRequest) -> ModelProviderFuture<'_, ProviderResult<ProviderModelCatalog>>`; the injected provider owns its HTTP factory.
- `ModelProvider` no longer declares `models_manager`, `models_manager_without_cache`, or `models_manager_with_cache`.
- `ProviderModelCatalogSource` implements `ModelCatalogSource` by delegating to `SharedModelProvider`.

- [x] Add a test that calls the provider-facing list method and asserts the normalized `ModelCatalogSnapshot` for Cloudflare.
- [x] Add a source-adapter test that delegates through `ProviderModelCatalogSource` and preserves identity/capability metadata.
- [x] Remove provider-owned manager construction methods and the Cloudflare-specific endpoint client; route all provider catalog fetches through `ModelProvider::list_models`.
- [x] Implement `ProviderModelCatalogSource` and preserve default, My Provider, and Cloudflare discovery behavior.
- [x] Run the model-provider test target or required test-target compilation and verify all provider catalog tests pass.

### Task 4: Move manager construction to the shared catalog layer

**Files:**
- Modify: `crates/models-manager/src/manager.rs` or add a focused catalog factory module
- Modify: `crates/core/src/thread_manager.rs`
- Modify: `crates/core/src/test_support.rs`
- Modify: `crates/core/tests/suite/injected_models_cache.rs`
- Modify: `crates/model-provider/src/provider.rs` tests and `my_provider.rs` tests as needed
- Modify: any remaining direct provider manager call sites found by `rg`

**Interfaces:**
- Shared construction accepts a `ModelCatalogSource`, optional static `ModelCatalog`, cache mode, auth manager, and discovery policy.
- `RemoteModelsManager` and `StaticModelsManager` remain the concrete `ModelsManager` implementations.

- [x] Add a shared construction test covering static catalogs and remote catalogs through a source adapter.
- [x] Run it to verify current direct provider-manager calls fail after Task 3 removes those methods.
- [x] Implement the shared factory/composition path and update all call sites; preserve Cloudflare/My Provider default API-key discovery policy.
- [x] Run focused core/model-manager checks and verify configured static catalogs still bypass remote requests.

### Task 5: Final audit and verification

**Files:**
- Modify: any remaining catalog naming/documentation references found by `rg`

- [x] Confirm no Rust references remain to `ModelsResponse`, `ModelsEndpointResponse`, or `ModelsEndpointClient`.
- [x] Confirm the only remaining endpoint-specific names describe actual HTTP endpoints, not catalog abstractions.
- [x] Run `rustfmt --edition 2024 --check` on changed Rust files and `git diff --check`.
- [x] Run the required workspace `just check` and `just check --tests` commands under the repository memory guardrails.
