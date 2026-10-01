# Remove Personality Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the obsolete personality concept, including its prompt sections, configuration, protocol fields, runtime state, and compatibility metadata.

**Architecture:** First remove the built-in and catalog-provided personality instructions so that `Personality::None` has no remaining behavioral purpose. Then remove the personality value from the configuration, app-server/TUI protocol, session/rollout state, and model metadata. Old persisted data must remain readable by ignoring the removed field where the existing deserializers permit it; no new runtime personality compatibility layer should be introduced.

**Tech Stack:** Rust workspace, Serde/TOML/JSON, app-server protocol schemas, model instruction templates, Just verification targets.

**Spec:** [`docs/pre-release-compatibility-inventory.md`](../../pre-release-compatibility-inventory.md), [`docs/provider-decoupling-candidates.md`](../../provider-decoupling-candidates.md)

## Global Constraints

- Work directly on `main`; do not create a worktree for this repository.
- Record the producer/consumer audit and the explicit `remove` decision in [`docs/pre-release-compatibility-inventory.md`](../../pre-release-compatibility-inventory.md) before changing production code.
- Remove the personality concept only; retain provider-neutral instruction configuration such as `instructions`, `developer_instructions`, `model_instructions_file`, and `compact_prompt`.
- Remove the built-in `# Personality` / `## Personality` sections and `<personality_spec>` inputs rather than silently preserving a different personality behavior.
- Preserve thread/session resume and history loading for records that contain an old personality field by dropping that field at the read boundary when possible.
- Run `just check` for production type-checking after each implementation slice.
- Run `just check --tests` whenever Rust code or test-only code is changed.
- Run `just generate` whenever config, app-server schema, or checked-in generated artifacts may change, and inspect the resulting diff.
- Do not run standalone test commands unless explicitly requested.

## Review Focus

- A model with an old catalog entry containing a personality section must no longer receive that section after the prompt assets and catalog fixtures are updated.
- Explicit `instructions` and `model_instructions_file` must remain functional; removing personality must not remove general developer/model instruction support.
- `thread/start`, `turn/start`, `thread/resume`, and settings updates must reject or ignore the removed personality field according to the existing protocol compatibility policy without emitting personality state.
- Existing rollout/thread history containing `personality` must still load, while newly written history must not contain the field.
- Removing `Personality::None` must not accidentally remove model switching, session restart, or role-based instruction invalidation that is unrelated to personality.

## Planned Order

### Task 1: Freeze the removal boundary in the compatibility inventory

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Inspect: `crates/protocol/src/config_types.rs`
- Inspect: `crates/models-manager/src/model_info.rs`
- Inspect: `crates/core/tests/suite/personality.rs`
- Inspect: `crates/app-server-protocol/src/protocol/v2/thread.rs`
- Inspect: `crates/app-server-protocol/src/protocol/v2/turn.rs`

**Interfaces:**
- Consumes: the current producer/consumer audit for configuration, prompt rendering, app-server, TUI, session state, and history.
- Produces: an explicit `remove (planned)` decision that later tasks can implement without reopening the scope.

- [x] **Step 1: Record the decision and boundary.**

  Change the personality inventory entry from `retain` to `remove (planned)`. State that `friendly` and `pragmatic` no longer select styles, `none` only strips the built-in personality section, and the section itself is being removed first. List the protocol, runtime, TUI, persistence, schema, and model-catalog consumers that must be removed.

- [x] **Step 2: Confirm the replacement boundary.**

  Explicitly retain `instructions`, `developer_instructions`, `model_instructions_file`, and `compact_prompt` as the supported instruction mechanisms. Do not add a `personality_v2` or replacement field.

- [x] **Step 3: Verify the audit before code changes.**

  Use `rg` to confirm that the inventory names all production references to `Personality`, `personality_spec`, `supports_personality`, and the three personality instruction variables. Record any intentional incidental matches in documentation or generic prompting text.

### Task 2: Remove personality instructions and rendering behavior

**Files:**
- Modify: `crates/models-manager/prompt.md`
- Modify: `crates/protocol/src/prompts/base_instructions/default.md`
- Modify: `crates/core/gpt_5_1_prompt.md`
- Modify: `crates/core/gpt_5_2_prompt.md`
- Modify/Delete: `crates/core/templates/personalities/`
- Modify: `crates/models-manager/test-data/models.json`
- Modify: `crates/models-manager/src/model_info.rs`
- Modify: `crates/core/src/event_mapping.rs`
- Test: `crates/core/tests/suite/personality.rs`
- Test: `crates/models-manager/src/model_info_tests.rs`
- Test: `crates/prompts/src/model_instructions_tests.rs`

**Interfaces:**
- Consumes: the existing model instruction resolution path.
- Produces: instruction resolution with no personality section, personality placeholder, or personality-specific stripping path.

- [x] **Step 1: Update prompt and catalog fixtures.**

  Remove the `# Personality` / `## Personality` sections, `<personality_spec>` placeholders, and personality-specific template files from built-in prompts and model catalog fixtures. Keep the surrounding general instructions unchanged.

- [x] **Step 2: Remove the conditional stripping path.**

  Delete `Personality::None` checks and `strip_personality_section` from `model_info.rs`. Preserve explicit `base_instructions` precedence and all non-personality model instruction resolution.

- [x] **Step 3: Remove persisted personality prompt mapping.**

  Remove `<personality_spec>` handling from event/instruction mapping once replayed events no longer need to reconstruct it. Verify that ordinary event mapping remains unchanged.

- [x] **Step 4: Update behavior tests.**

  Delete tests that assert personality selection or stripping. Replace them with assertions that default and catalog instructions contain no personality section, while explicit base/model instructions remain unchanged.

- [x] **Step 5: Verify the prompt slice.**

  Run `just check --tests` and `just check`. Confirm no active prompt/catalog source contains `# Personality`, `## Personality`, or `<personality_spec>` outside intentionally historical documentation.

### Task 3: Remove TOML, feature, and runtime configuration fields

**Files:**
- Modify: `crates/config/src/config_toml.rs`
- Modify: `crates/config/src/profile_toml.rs`
- Modify: `crates/protocol/src/config_types.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/core/src/config/managed_features.rs`
- Modify: `crates/features/src/lib.rs`
- Modify: `crates/features/src/tests.rs`
- Modify: `crates/core/src/config/schema_tests.rs`
- Regenerate: `crates/core/config.schema.json`, `codex-cli.json`, and generated config indexes

**Interfaces:**
- Consumes: the prompt layer with no personality behavior from Task 2.
- Produces: configuration resolution with no `personality` TOML field, `Config::personality`, `ConfigOverrides::personality`, or no-op personality feature.

- [x] **Step 1: Remove config and profile fields.**

  Delete the top-level and profile `personality` fields and their associated documentation/imports. Remove the merge path that projects them into runtime configuration.

- [x] **Step 2: Remove runtime config overrides.**

  Delete the `Config::personality` and `ConfigOverrides::personality` fields, including serialization/output and config batch-write handling for the `"personality"` key. Preserve all other session-default config keys.

- [x] **Step 3: Remove the no-op feature compatibility.**

  Delete `Feature::Personality`, its removed feature specification, `apply_map` no-op handling, managed-feature special case, and feature tests that only preserve the old name.

- [x] **Step 4: Regenerate and inspect config artifacts.**

  Run `just generate`. Confirm the personality fields disappear from generated schemas and source indexes without changing unrelated config fields.

- [x] **Step 5: Verify configuration cleanup.**

  Run `just check --tests` and `just check`. Confirm no active config producer or consumer references `personality`.

### Task 4: Remove app-server protocol and TUI personality state

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/v2/thread.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/turn.rs`
- Modify: `crates/app-server/src/request_processors/thread_processor.rs`
- Modify: `crates/app-server/src/request_processors/turn_processor.rs`
- Modify: `crates/app-server/src/request_processors/config_processor.rs`
- Modify: `crates/tui/src/app_server_session.rs`
- Modify: `crates/tui/src/app/thread_settings.rs`
- Modify: `crates/tui/src/app_command.rs`
- Modify: `crates/tui/src/session_state.rs`
- Modify: `crates/app-server/tests/suite/v2/turn_start.rs`
- Modify: `crates/app-server/tests/suite/v2/thread_resume.rs`
- Modify: `crates/app-server-client/src/lib.rs`
- Regenerate: app-server protocol JSON schemas

**Interfaces:**
- Consumes: runtime configuration without personality from Task 3.
- Produces: app-server and TUI settings APIs without personality request, response, notification, or cached-state fields.

- [x] **Step 1: Remove protocol fields and deprecation/mismatch handling.**

  Delete personality from thread start/resume/settings, turn start, and thread settings update request/response types. Remove resume mismatch diagnostics and request-builder parameters that exist only for personality.

- [x] **Step 2: Remove request processor plumbing.**

  Stop reading, forwarding, persisting, or comparing personality in thread and turn processors. Remove the batch-write session-default key and keep the remaining config update validation intact.

- [x] **Step 3: Remove TUI state and command fields.**

  Delete personality from app-server session state, thread settings, cached session state, user-turn commands, and override contexts. Preserve model selection, instruction overrides, and other turn settings.

- [x] **Step 4: Update clients and protocol tests.**

  Remove tests that send or observe personality-only changes. Keep tests that prove ordinary thread start/resume, settings update, and turn start behavior remains unchanged.

- [x] **Step 5: Regenerate and verify protocol artifacts.**

  Run `just generate`, inspect all app-server schema changes, then run `just check --tests` and `just check`.

### Task 5: Remove core session, thread, rollout, and history state

**Files:**
- Modify: `crates/core/src/session/step_settings.rs`
- Modify: `crates/core/src/session/mod.rs`
- Modify: `crates/core/src/session/session.rs`
- Modify: `crates/core/src/session/turn_context.rs`
- Modify: `crates/core/src/codex_thread.rs`
- Modify: `crates/protocol/src/protocol.rs`
- Modify: `crates/state/src/extract.rs`
- Modify: `crates/thread-store/src/local/mod.rs`
- Modify: `crates/thread-store/src/thread_metadata_sync.rs`
- Modify: rollout/thread-history fixtures and tests that only initialize personality

**Interfaces:**
- Consumes: protocol and config types without personality from Tasks 3 and 4.
- Produces: session/model reuse, thread snapshots, rollout metadata, and history writes with no personality field.

- [x] **Step 1: Remove step-setting and model-resolution fields.**

  Delete personality from `StepSettings`, sparse updates, model reuse comparisons, and `ModelsManagerConfig` construction. Preserve model switching and instruction override invalidation unrelated to personality.

- [x] **Step 2: Remove thread and turn snapshot fields.**

  Delete personality from `CodexThread`, session snapshots, turn context, per-turn rollout config, and protocol settings representations.

- [x] **Step 3: Audit persisted readers before changing serialization.**

  Check JSON/Serde readers for thread history, rollout metadata, and local thread-store records. Confirm whether unknown old `personality` keys are already ignored. If a reader rejects them, add a narrow read-only compatibility shim that consumes and drops only that key; do not reintroduce a typed runtime field.

- [x] **Step 4: Update history fixtures and extraction.**

  Remove personality from newly written metadata and fixtures. Add or retain a compatibility test proving that an old record containing `personality` loads and is rewritten without it.

- [x] **Step 5: Verify core and persistence cleanup.**

  Run `just check --tests` and `just check`. Inspect serialized sample/fixture diffs and confirm newly emitted records contain no personality key.

### Task 6: Remove model-catalog and compatibility metadata

**Files:**
- Modify: `crates/protocol/src/model_types.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/model.rs`
- Modify: `crates/app-server/src/models.rs`
- Modify: `crates/tui/src/app_server_session.rs`
- Modify: `crates/models-manager/src/config.rs`
- Modify: `crates/models-manager/test-data/models.json`
- Modify: model catalog fixtures and tests that initialize `supports_personality`
- Modify: `crates/features/src/tests.rs` if any personality compatibility fixture remains

**Interfaces:**
- Consumes: model instruction resolution and runtime types without personality.
- Produces: model metadata with no deprecated personality capability flag or personality-only instruction variables.

- [x] **Step 1: Remove `supports_personality`.**

  Delete the deprecated catalog/protocol field and all copying, serialization, and fixture initialization. Preserve the remaining model capability metadata.

- [x] **Step 2: Remove personality instruction variables.**

  Delete `personality_default`, `personality_friendly`, and `personality_pragmatic` from `instructions_variables` handling. Keep any unrelated legacy variables only if current catalogs still require them; otherwise record their separate removal in the inventory rather than conflating it with runtime personality.

- [x] **Step 3: Update model catalog tests.**

  Ensure catalogs without personality metadata continue to deserialize and model selection continues to work. Remove tests that only assert deprecated personality capability behavior.

- [x] **Step 4: Verify metadata cleanup.**

  Run `just check --tests` and `just check`; confirm no production model metadata type exposes personality capability.

### Task 7: Final generated-artifact, documentation, and compatibility audit

**Files:**
- Modify: `crates/app-server/README.md`
- Modify: personality-related documentation and snapshots
- Regenerate: all generated config/app-server schemas and checked-in source indexes affected by the removal
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: all implementation slices above.
- Produces: a repository with one consistent removal decision and no stale generated or user-facing contract.

- [x] **Step 1: Remove stale documentation.**

  Delete documentation describing `personality = "none"`, personality request fields, or personality-specific model behavior. Keep generic instruction configuration documentation.

- [x] **Step 2: Regenerate and inspect all artifacts.**

  Run `just generate`. Review the diff for config schema, app-server schema, model/catalog fixtures, snapshots, and source indexes. Do not accept unrelated generated changes.

- [x] **Step 3: Run repository-wide reference audit.**

  Run `rg -n 'Personality|personality_spec|supports_personality|personality_default|personality_friendly|personality_pragmatic' crates --glob '*.rs' --glob '*.md' --glob '*.json'`. Classify every remaining match as an intentional historical/compatibility test, generic prose, or stale active reference; remove stale active references.

- [x] **Step 4: Complete verification.**

  Run `just check --tests`, `just check`, and `git diff --check`. If generated artifacts are affected, `just generate` must have completed before the checks.

- [x] **Step 5: Close the inventory entry.**

  Change the personality entry to `remove (completed)` and record the final verification commands plus the persisted-data compatibility result.

## Completion Criteria

- Built-in and catalog personality sections, placeholders, and personality-specific templates are removed.
- TOML, profile, feature, runtime, app-server, TUI, session, rollout, and model metadata no longer expose personality.
- Existing old thread/rollout records containing a personality field remain readable or are handled by a narrow drop-only reader shim.
- General instruction configuration, model switching, thread resume, and provider-neutral model capabilities remain functional.
- Generated schemas, fixtures, docs, and the compatibility inventory agree with the removal.
- `just check --tests`, `just check`, and applicable `just generate` verification complete successfully.
