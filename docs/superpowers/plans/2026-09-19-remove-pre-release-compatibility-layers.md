# Pre-release Compatibility-layer Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove obsolete upstream and historical compatibility layers across the repository before continuing OpenAI-specific integration removal.

**Architecture:** Inventory compatibility candidates first, then remove them by contract boundary: persisted data, protocol/app-server, CLI/config/import, and UI/observability. Preserve current runtime fallbacks and provider-neutral behavior only when a current producer or consumer is demonstrated.

**Tech Stack:** Rust workspace, Cargo, app-server protocol schemas, rollout/thread-store persistence, CLI/TUI, generated dependency graph.

**Spec:** `docs/superpowers/specs/2026-09-19-remove-pre-release-compatibility-layers-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- This fork is pre-release with zero external users; old external protocol and persisted-data compatibility is not required.
- Do not delete local rollout or config files automatically; remove only obsolete readers, migrations, aliases, and replay paths.
- Preserve current provider-neutral capabilities, generic media handling, active in-repository clients, platform support, and security behavior.
- For crate-removal work, use `cargo check` and `just check` as the primary verification; test-first development is not required.
- Every compatibility candidate must receive an explicit `remove`, `retain`, or `migrate-in-place` decision in the inventory.

## Review Focus

- The inventory must distinguish historical compatibility from current runtime fallback.
- Removing rollout migration must not break current canonical history writes or reads.
- Removing app-server aliases must not break an active in-repository client.
- Removing deprecated CLI/config surfaces must not remove the canonical replacement.
- Removing UI/telemetry compatibility branches must not remove current tool, media, or security behavior.

## File Map

- **Policy:** `AGENTS.md`, `docs/openai-decoupling-plan.md`, `docs/openai-specific-crates.md`, and the superseded image-generation compatibility plan.
- **Inventory:** new `docs/pre-release-compatibility-inventory.md`, produced from repository-wide `rg` searches and manual call-site checks.
- **Persistence:** `crates/thread-store/src/local/rollout_migration.rs`, `crates/thread-store/src/local/rollout_migration/`, `crates/thread-store/src/local/thread_history/`, `crates/rollout/src/`, and `crates/cli/src/migrate_rollouts.rs`.
- **Protocol/app-server:** `crates/protocol/src/legacy_events.rs`, `crates/protocol/src/models.rs`, `crates/protocol/src/config_types.rs`, `crates/protocol/src/openai_models.rs`, `crates/app-server-protocol/src/protocol/v1.rs`, `crates/app-server-protocol/src/protocol/common.rs`, `crates/app-server-protocol/src/protocol/v2/`, generated app-server schemas, and app-server request processors.
- **CLI/config/import:** `crates/cli/src/`, `crates/core/src/config/`, `crates/app-server/src/config*`, `crates/app-server/src/external_agent_migration/`, and related tests/docs selected by the inventory.
- **Runtime/UI/observability:** `crates/core/src/context/legacy_*`, `crates/core/src/context/world_state/`, `crates/sandboxing/src/manager.rs`, `crates/tui/src/`, `crates/analytics/src/`, `crates/otel/src/`, `crates/rollout-trace/src/`, and tests selected by the inventory.

### Task 1: Establish the repository-wide compatibility inventory

**Files:**
- Create: `docs/pre-release-compatibility-inventory.md`
- Modify: `AGENTS.md`
- Modify: `docs/openai-decoupling-plan.md`
- Modify: `docs/openai-specific-crates.md`
- Modify: `docs/superpowers/plans/2026-09-19-remove-image-generation-adapter.md`
- Modify: `docs/superpowers/plans/2026-09-19-remove-pre-release-image-generation-compatibility.md`

- [x] Run the searches below and record each production match in the inventory:

```bash
rg -n -i 'legacy|compatib|deprecated|deprecat|migration|fallback|historical|serde\(alias' crates docs --glob '*.rs' --glob '*.md' --glob '*.toml' --glob '*.json'
rg -n -i 'image_generation|image_generation_call|image_gen' crates --glob '*.rs' --glob '*.json' --glob '*.ts'
```

- [x] For every candidate, record the exact symbol, current producer/consumer, canonical replacement, decision (`remove`, `retain`, or `migrate-in-place`), and verification command.
- [x] Mark image-generation history cleanup as one inventory slice, not the whole compatibility project.
- [x] Add the inventory path and ordering rule to `AGENTS.md`.
- [x] Update the decoupling documents to require this cleanup before the next OpenAI-specific integration.
- [x] Mark the image-specific compatibility plan as superseded by this repository-wide plan.
- [x] Run `git diff --check` and review the inventory for candidates classified solely by name rather than call-site evidence.

### Task 2: Remove obsolete persisted-data compatibility

**Progress:** Complete. Rollout `ghost_snapshot` filtering/config/schema
compatibility, the deprecated conversation-path re-export, migration state,
legacy persistence policy/ordinal branches, old metadata fallbacks, and the
explicit legacy history readers were removed. Existing rollout/config files are
left untouched; current writers, SQLite projection, paging, lineage, and
external-agent import remain on the paginated contract.

**Files:**
- Modify or delete files selected in the inventory under `crates/thread-store/src/local/rollout_migration/`, `crates/thread-store/src/local/thread_history/`, `crates/rollout/src/`, and `crates/cli/src/migrate_rollouts.rs`.
- Modify: related thread-store, rollout, CLI, and app-server migration tests.

- [x] Remove migrations for obsolete event/item shapes that have no current writer, including the image-generation migration slice.
- [x] Remove old rollout readers, rollback replay branches, historical metadata aliases, and migration-only telemetry only when the inventory marks them `remove`.
- [x] Preserve canonical current rollout writing, paging, search, thread metadata, and deletion behavior.
- [x] Keep unsupported old files untouched on disk; do not add a cleanup command that silently deletes them.
- [x] Run `just check` as the repository verification gate.

### Task 3: Remove obsolete protocol and app-server compatibility

**Files:**
- Modify: `crates/protocol/src/legacy_events.rs`, `models.rs`, `config_types.rs`, `openai_models.rs`, and selected protocol modules.
- Modify: `crates/app-server-protocol/src/protocol/v1.rs`, `common.rs`, `v2/`, generated schema files, and selected app-server request processors.
- Modify: `docs/protocol_v1.md` and protocol tests for removed aliases.

- [x] Remove deprecated fields, serde aliases, legacy event tags, and compatibility-only response variants that have no active in-repository consumer.
- [x] Remove old app-server protocol surfaces only after checking all TUI, CLI, test, and daemon call sites; update the canonical replacement in the same change when one exists.
- [x] Remove generated schema and TypeScript definitions for deleted fields/variants, then inspect the generated diff for unrelated changes.
- [x] Preserve active protocol versions that current in-repository clients still use, and record them as retained with their consumer in the inventory.
- [x] Verify with `just check`.

### Task 4: Remove obsolete CLI, configuration, and import compatibility

**Files:**
- Modify: `crates/cli/src/`, `crates/core/src/config/`, `crates/app-server/src/config*`, and `crates/app-server/src/external_agent_migration/` entries selected by the inventory.
- Modify: corresponding CLI/config/app-server tests and user-facing documentation.

- [x] Remove deprecated CLI commands, config aliases, old field names, and compatibility-only import paths with no current producer.
- [x] Keep the canonical command/config path and update help, schema, completion, and error messages together.
- [x] Treat external-agent migration as compatibility-only only if no current supported import workflow consumes it; otherwise retain it and document the active consumer.
- [x] Remove obsolete alias fixtures while retaining canonical configuration and active import behavior.
- [x] Verify the affected production crates with `just check`.

### Task 5: Remove obsolete UI, observability, and runtime shims

**Files:**
- Modify: `crates/tui/src/`, `crates/analytics/src/`, `crates/otel/src/`, `crates/rollout-trace/src/`, and `crates/core/src/context/` entries selected by the inventory.
- Review: `crates/sandboxing/src/manager.rs` and platform-specific tests.

- [x] Remove replay/display/status/telemetry/trace branches that exist only for deleted historical shapes.
- [x] Remove `legacy_*` prompt or UI paths only when the current canonical prompt/config path fully replaces them.
- [x] Retain platform fallbacks such as legacy Landlock behavior when current supported environments still require them; record the evidence.
- [x] Preserve active media, security, and client behavior while removing compatibility-only history paths.
- [x] Re-run the broad search and classify remaining production matches through the inventory.

### Task 6: Final verification and next-target handoff

**Files:**
- Modify: `Cargo.lock` only if dependency removal changes it.
- Modify: `target/crate-dependency-graph/` only if workspace dependency edges change.
- Modify: `docs/openai-decoupling-plan.md` and the inventory with final decisions.

- [x] Run workspace `cargo check` through `just check`.
- [x] Check repository-local `target/` size before `just check`; it was 131G, above the 120G cleanup threshold, so only the repository-local `target/` directory was removed before checking.
- [x] Run `just check`.
- [x] No manifests or crate dependency edges changed, so dependency-graph regeneration is not required.
- [x] Confirm the compatibility inventory has no unclassified production entries.
- [x] Leave the next OpenAI-specific integration selection to the subsequent Phase 3 task.
