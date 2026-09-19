# Superseded: Pre-release Image-generation Compatibility Removal Implementation Plan

> This image-generation-only plan is retained as historical context. Its scope is now covered by `docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the image-generation compatibility layer from this pre-release fork before continuing OpenAI-specific integration removal, while preserving generic image input and viewing.

**Architecture:** Delete the old Responses image-generation item, extension item, legacy events, rollout migration, app-server protocol variants, and UI/observability branches as one clean-break surface. Keep generic image media types and tools intact; future image generation, if needed, will be a new provider-neutral capability.

**Tech Stack:** Rust workspace, Cargo manifests and generated app-server schemas, rollout/thread-store persistence, TUI, analytics, and trace classification.

**Spec:** `docs/superpowers/specs/2026-09-19-remove-pre-release-image-generation-compatibility-design.md`

**Current status:** Task 1 is complete; Tasks 2–5 are implementation work.

## Global Constraints

- Work directly on `main`; do not create a worktree.
- This is a pre-release clean break: old image-generation rollout and protocol compatibility is not required.
- Do not delete local rollout files automatically; only remove the code that reads or migrates image-generation records.
- Preserve user-provided images, local media, image preparation/resizing, and `view_image`.
- For crate-removal work, use `cargo check` and `just check` as the primary verification; test-first development is not required.
- Update generated schemas and dependency-graph artifacts when source changes require them.

## Review Focus

- Removing output-side image-generation types must not remove input-side image handling or `view_image`.
- Removing legacy migration must not break migration of unrelated event types.
- Removing `image_generation_call` must not alter generic Responses history normalization for other item variants.
- Removing TUI replay branches must not suppress ordinary image-view replay.
- Removing image-generation analytics and trace code must not remove shared turn/tool accounting.

## File Map

- **Policy and project guidance:** `AGENTS.md`, `docs/openai-decoupling-plan.md`, `docs/openai-specific-crates.md`, and the completed adapter-removal plan.
- **Canonical data and lifecycle types:** `crates/ext/items/src/image_generation.rs`, `crates/ext/items/src/lib.rs`, `crates/ext/items/src/tests.rs`, `crates/protocol/src/{items.rs,models.rs,protocol.rs,legacy_events.rs}`.
- **Core mapping and history:** `crates/core/src/agent/control/spawn.rs`, `client_common.rs`, `config/mod.rs`, `context_manager/history.rs`, `context_manager/history_tests.rs`, `context_manager/normalize.rs`, `event_mapping.rs`, `image_preparation.rs`, `realtime_history/presentation.rs`, `realtime_history_tests.rs`, `session/turn.rs`, `stream_events_utils.rs`, `tools/handlers/extension_tools.rs`, `turn_timing.rs`, `tests/common/responses.rs`, and `tests/suite/model_switching.rs`.
- **Persistence and app-server:** `crates/thread-store/src/local/{rollout_migration/legacy_event.rs,rollout_migration_tests.rs,thread_history.rs,thread_history/search.rs}`, `crates/rollout/src/{policy.rs,persistence_metrics.rs}`, `crates/app-server-protocol/src/protocol/{v2/item.rs,thread_history.rs}`, generated app-server schemas, and the app-server notification/request processors and resume tests.
- **UI and observability:** `crates/tui/src/{thread_transcript.rs,dynamic_tools.rs,app/agent_status_feed.rs,chatwidget/{protocol.rs,replay.rs,tool_lifecycle.rs}}`, analytics, rollout-trace, otel, guardian-context, audio, goal, and code-mode references identified by the inventory search below.

### Task 1: Record the clean-break policy

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/openai-decoupling-plan.md`
- Modify: `docs/openai-specific-crates.md`
- Modify: `docs/superpowers/plans/2026-09-19-remove-image-generation-adapter.md`

- [x] Update `AGENTS.md` to require removal of pre-release compatibility layers before resuming the remaining OpenAI-specific integrations.
- [x] Change the decoupling plan to state that image-generation history/protocol/migration/display compatibility is intentionally not preserved in this zero-user fork.
- [x] Update the OpenAI-specific crate inventory to distinguish the already-removed adapter from the compatibility cleanup covered by this plan.
- [x] Add a superseding note to the completed adapter-removal plan so it no longer describes retained compatibility as the final project policy.
- [x] Run `git diff --check` and inspect the documentation diff for contradictory compatibility requirements.

### Task 2: Remove canonical image-generation data and lifecycle types

**Files:**
- Delete: `crates/ext/items/src/image_generation.rs`
- Modify: `crates/ext/items/src/lib.rs`, `crates/ext/items/src/tests.rs`
- Modify: `crates/protocol/src/items.rs`, `models.rs`, `protocol.rs`, `legacy_events.rs`
- Modify: `crates/core/src/agent/control/spawn.rs`, `client_common.rs`, `config/mod.rs`, `context_manager/history.rs`, `context_manager/history_tests.rs`, `context_manager/normalize.rs`, `event_mapping.rs`, `image_preparation.rs`, `realtime_history/presentation.rs`, `realtime_history_tests.rs`, `session/turn.rs`, `stream_events_utils.rs`, `tools/handlers/extension_tools.rs`, `turn_timing.rs`, `tests/common/responses.rs`, `tests/suite/model_switching.rs`.

- [ ] Run `rg -n -i 'image.?generation|image_generation_call|image_gen' crates` and classify every match as generation-specific or generic image handling before editing.
- [ ] Remove `ImageGenerationItem`, `ImageGenerationFailure`, and the `ExtensionItem::ImageGeneration` variant, including their schemas and tests.
- [ ] Remove `ResponseItem::ImageGenerationCall`, `ImageGenerationBeginEvent`, `ImageGenerationEndEvent`, and conversion/helpers that exist only for those types.
- [ ] Remove core event mapping, history retention, normalization, timing, and turn-processing branches for the deleted variants.
- [ ] Remove only image-generation fixtures and assertions; retain input-image, local-media, and `view_image` coverage.
- [ ] Run `cargo check -p codex-protocol -p codex-core -p codex-extension-items` and fix only compile errors caused by the removed variants.

### Task 3: Remove persistence migration and public app-server protocol surface

**Files:**
- Modify: `crates/thread-store/src/local/rollout_migration/legacy_event.rs`, `rollout_migration_tests.rs`, `thread_history.rs`, `thread_history/search.rs`
- Modify: `crates/rollout/src/policy.rs`, `persistence_metrics.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/item.rs`, `thread_history.rs`
- Modify: generated files under `crates/app-server-protocol/schema/`
- Modify: `crates/app-server/src/notification_media.rs`, `request_processors/thread_processor.rs`, `request_processors/thread_resume_redaction.rs`, `request_processors/turn_processor.rs`, `crates/app-server/tests/suite/v2/thread_resume.rs`.

- [ ] Remove the `EventMsg::ImageGenerationEnd` migration arm and its dedicated migration test without changing unrelated legacy-event conversions.
- [ ] Remove image-generation persistence policy and item-search branches while preserving ordinary thread-history paging and search.
- [ ] Remove app-server `ThreadItem::ImageGeneration`, public re-exports, conversion branches, and generated JSON/TypeScript definitions.
- [ ] Remove image-generation-specific notification redaction and turn/resume handling; retain redaction for generic media and other response items.
- [ ] Regenerate or update app-server schemas using the repository’s supported generator path, then inspect that only image-generation definitions disappeared.
- [ ] Run `cargo check -p codex-thread-store -p codex-rollout -p codex-app-server-protocol -p codex-app-server`.

### Task 4: Remove UI, analytics, and trace compatibility branches

**Files:**
- Modify: `crates/tui/src/app/agent_status_feed.rs`, `dynamic_tools.rs`, `chatwidget/protocol.rs`, `chatwidget/replay.rs`, `chatwidget/tool_lifecycle.rs`, `thread_transcript.rs`, `history_cell/patches.rs`, `crates/tui/src/dynamic_tools_tests.rs`, `crates/tui/src/history_cell/tests.rs`, `crates/tui/src/chatwidget/tests/exec_flow.rs`, `crates/tui/src/chatwidget/tests/helpers.rs`.
- Modify: `crates/analytics/src/{events.rs,reducer.rs,analytics_client_tests.rs}`.
- Modify: `crates/rollout-trace/src/{model/runtime.rs,protocol_event.rs,tool_dispatch.rs,reducer/conversation/normalize.rs,reducer/tool/agents.rs,reducer/tool/terminal.rs}`.
- Modify: `crates/otel/src/events/session_telemetry.rs`, `crates/guardian-context/src/transcript.rs`, `crates/utils/audio/src/lib.rs`, `crates/ext/goal/src/accounting.rs`, `crates/ext/guardian-v2/src/async_scorer/transcript_tests.rs`.
- Modify or delete image-generation-only code in `crates/code-mode-protocol/src/description.rs` and `crates/code-mode-runtime/src/runtime/callbacks.rs` after confirming it has no non-image-generation producer.

- [ ] Remove TUI replay, lifecycle, transcript, dynamic-tool, status-feed, and history-cell branches for image generation.
- [ ] Remove image-generation counters, event payloads, telemetry names, and trace classification only where the value is specific to the deleted capability.
- [ ] Remove image-generation tool aliases from trace dispatch; preserve unrelated tool classification and generic image viewing.
- [ ] Remove code-mode `generatedImage` handling if it is solely the deleted image-generation output bridge; otherwise document and retain its provider-neutral contract.
- [ ] Run `rg -n -i 'image.?generation|image_generation_call|image_gen' crates` again and review every remaining match manually.
- [ ] Run `cargo check -p codex-tui -p codex-analytics -p codex-rollout-trace -p codex-otel -p codex-guardian-context -p codex-utils-audio -p codex-code-mode-protocol -p codex-code-mode-runtime` and confirm no generic image path was removed.

### Task 5: Final verification and dependency/documentation audit

**Files:**
- Modify: `Cargo.lock` only if dependency removal changes it.
- Modify: `target/crate-dependency-graph/` generated outputs if workspace dependencies changed.

- [ ] Run `cargo check` for the workspace.
- [ ] Check repository-local `target/` size before `just check`; if it exceeds the project’s documented threshold, remove only that repository-local `target/` directory before checking.
- [ ] Run `just check` as the repository-wide verification.
- [ ] Run `just crate-dependency-graph` when manifests or crate dependencies changed, and confirm no image-generation-only crate or dependency remains.
- [ ] Review `git diff --check`, generated schema diffs, and the final `rg` inventory.
- [ ] Confirm the final documentation says that old image-generation rollout files are unsupported but not automatically deleted.
