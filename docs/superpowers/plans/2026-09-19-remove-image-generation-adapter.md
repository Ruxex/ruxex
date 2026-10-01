# Image Generation Adapter Removal Implementation Plan

> Follow-up policy: this plan recorded the initial adapter removal while retaining history compatibility. Because this fork is pre-release with zero external users, the retained compatibility layer is superseded by the repository-wide `docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md`.

> Status: completed and superseded. Do not resume this plan; its retained-history
> assumption is replaced by the repository-wide inventory and ordered cleanup
> slices.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the provider Images API image-generation adapter and its live tool registration while preserving provider-neutral image history, protocol, and display compatibility.

**Architecture:** Delete the dedicated `codex-image-generation-extension` and the unused Images endpoint/types from `genex-api`. Remove the feature and provider/app-server capability fields that only advertised this deleted live integration. Retain `ImageGenerationItem`, response-history variants, rollout migration, and UI rendering so existing stored image-generation records remain readable.

**Tech Stack:** Rust workspace, Cargo manifests/lockfile, generated app-server and config schemas, dependency-graph script.

**Spec:** `docs/provider-decoupling-plan.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Treat this as crate-removal work; do not run tests.
- Use `just check` as the verification command.
- Before `just check`, if `target/` exceeds 120G, remove that repository-local directory and let the check recreate it.
- Preserve provider-neutral image history, rollout migration, protocol, and UI display types.
- Update the dependency graph after the removal.

## Review Focus

- No live extension registration or `image_gen` tool remains, while stored image-generation history still deserializes and renders.
- No `codex-image-generation-extension` or Images API endpoint/type dependency remains in workspace manifests or Rust sources.
- Model-provider capability responses no longer advertise a capability with no installed executor.
- Generated schemas and `Cargo.lock` agree with the source and workspace membership.

### Task 1: Remove the provider image adapter and live registration

**Files:**
- Delete: `crates/ext/image-generation/`
- Modify: `Cargo.toml`, `Cargo.lock`
- Modify: `crates/app-server/Cargo.toml`, `crates/app-server/src/extensions.rs`
- Modify: `crates/core/Cargo.toml`, `crates/core-api/Cargo.toml`, `crates/core-api/src/lib.rs`
- Modify: `crates/thread-manager-sample/src/main.rs`
- Delete: `crates/app-server/tests/suite/v2/imagegen_extension.rs`
- Modify: `crates/app-server/tests/suite/v2/mod.rs`
- Delete: `crates/core/tests/suite/extension_sandbox.rs`
- Modify: `crates/core/tests/suite/mod.rs`
- Modify: `crates/core/tests/suite/responses_lite.rs`

- [x] Remove the workspace member/dependency and every production registration/import of `codex-image-generation-extension`.
- [x] Remove tests that exist only to exercise the deleted image extension, and remove only image-generation-specific setup/assertions from the mixed Responses Lite suite; retain the web-search coverage.
- [x] Leave `genex-extension-items` image records and app-server/TUI history handling untouched.

### Task 2: Remove stale Images API and capability/feature advertisement

**Files:**
- Delete: `crates/genex-api/src/endpoint/images.rs`, `crates/genex-api/src/images.rs`
- Modify: `crates/genex-api/src/endpoint/mod.rs`, `crates/genex-api/src/lib.rs`
- Modify: `crates/model-provider/src/provider.rs`, `crates/model-provider/src/amazon_bedrock/mod.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/model.rs`
- Modify: `crates/app-server/src/request_processors/config_processor.rs`
- Modify: `crates/app-server/tests/suite/v2/model_provider_capabilities_read.rs`
- Modify: `crates/core/src/tools/spec_plan.rs`, `crates/core/src/tools/spec_plan_tests.rs`
- Modify: `crates/features/src/lib.rs`, `crates/features/src/legacy.rs`, `crates/features/src/tests.rs`
- Modify: `crates/core/config.schema.json`, `crates/tui/src/temporary_structured_request.rs`

- [x] Remove the unused Images client/request/response API and its endpoint module exports.
- [x] Remove `ProviderCapabilities.image_generation` and the app-server `modelProvider/capabilities/read` response field, preserving namespace tools and web-search fields.
- [x] Remove the `Feature::ImageGeneration` toggle and legacy alias, plus the image-specific tool-plan gate and tests.
- [x] Keep all historical `ResponseItem::ImageGenerationCall`, extension item, rollout, migration, and display code required for compatibility.

### Task 3: Regenerate artifacts, inspect the diff, and verify

**Files:**
- Modify: `Cargo.lock`
- Modify: generated app-server protocol schema files under `crates/app-server-protocol/schema/`
- Modify: `crates/core/config.schema.json`
- Modify: `docs/provider-decoupling-plan.md`, `docs/provider-specific-crates.md`
- Modify: `target/crate-dependency-graph/` generated outputs

- [x] Synchronize the app-server and config schemas after the source changes. The repository app-server schema generator invokes `cargo test`, so the capability schema was updated directly while the requested verification remained `just check`.
- [x] Record the image adapter/Images endpoint removal and retained history compatibility in the decoupling documents; keep Phase 3 in progress.
- [x] Run `just crate-dependency-graph` and confirm the image-generation crate and Images endpoint are absent from the graph.
- [x] Check the repository-local `target/` size; it was 116.5GB, below the 120G cleanup threshold.
- [x] Run `just check` and do not run tests as the verification step.
