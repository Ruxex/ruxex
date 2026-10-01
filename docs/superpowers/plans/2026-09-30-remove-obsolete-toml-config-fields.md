# Remove Obsolete TOML Configuration Fields Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove configuration fields that have no current producer/consumer or are obsolete compatibility aliases, while preserving active provider-neutral behavior and security boundaries.

**Architecture:** Treat `ConfigToml` and its nested `*Toml` structs as the public TOML/schema contract. Remove a field only after auditing its consumers, then remove the field from the TOML type, runtime projection, generated schema, and tests together. Keep active runtime/protocol fields when only their TOML entry is obsolete, and migrate provider-specific naming before removing `provider_base_url`.

**Tech Stack:** Rust workspace, Serde/TOML, Schemars-generated JSON schema, app-server protocol, Just verification targets.

**Spec:** [`docs/pre-release-compatibility-inventory.md`](../../pre-release-compatibility-inventory.md), [`docs/provider-decoupling-candidates.md`](../../provider-decoupling-candidates.md)

## Global Constraints

- Work directly on `main`; do not create a worktree for this repository.
- Follow the inspection order and producer/consumer decision rule in `AGENTS.md`.
- Record an explicit `remove`, `retain`, or `migrate-in-place` decision in [`docs/pre-release-compatibility-inventory.md`](../../pre-release-compatibility-inventory.md) before changing production code.
- Preserve active platform fallbacks, provider-neutral capabilities, in-repository clients, and security behavior.
- Run `just check` for production type-checking after each implementation slice.
- Run `just check --tests` whenever Rust code or test-only code is changed.
- Run `just generate` when config or protocol generated artifacts may change, and inspect the diff.
- Do not run standalone test commands unless explicitly requested.

## Review Focus

- A removed field must no longer appear in `ConfigToml`, profile TOML types, generated config schema, or generated source indexes.
- The canonical replacement must continue to work: `compact_prompt`, `tui.disable_paste_burst`, and `features.unified_exec`.
- Unknown removed fields must follow the repository's strict/non-strict configuration behavior without accidentally becoming active again through CLI session flags.
- `personality` must not be removed while active app-server, TUI, session replay, and prompt consumers remain.
- `provider_base_url` must not be deleted before the credential-broker trusted-host and redaction behavior has an equivalent provider-neutral source.

## Planned Order

### Task 1: Audit and freeze the removal boundaries

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Inspect: `crates/config/src/config_toml.rs`
- Inspect: `crates/config/src/profile_toml.rs`
- Inspect: `crates/core/src/config/mod.rs`
- Inspect: `crates/features/src/legacy.rs`
- Inspect: `crates/features/src/lib.rs`
- Inspect: `crates/app-server-protocol/src/protocol/v2/config.rs`

**Interfaces:**
- Consumes: current `ConfigToml`, profile, feature, runtime, and protocol consumers.
- Produces: explicit decisions and removal boundaries for the later tasks.

- [ ] **Step 1: Confirm the producer/consumer audit for each candidate.**

  Record these decisions before production edits:

  - `experimental_compact_prompt_file`: `remove`; retain inline `compact_prompt`.
  - Top-level `disable_paste_burst`: `remove`; retain `tui.disable_paste_burst`.
  - Top-level/profile `experimental_use_unified_exec_tool`: `remove` as a legacy alias; retain `features.unified_exec`.
  - `personality`: `retain` for now; active runtime and protocol consumers remain, and no canonical replacement field has been selected.
  - `provider_base_url`: `migrate-in-place`; retain credential-broker security behavior until a replacement source exists.

- [ ] **Step 2: Update the compatibility inventory with exact symbols and rationale.**

  Include the canonical replacement, the affected producer/consumer paths, and the verification commands for each decision. Do not mark `personality` or `provider_base_url` as removed in this plan's first implementation slice.

### Task 2: Remove `experimental_compact_prompt_file`

**Files:**
- Modify: `crates/config/src/config_toml.rs:444`
- Modify: `crates/config/src/profile_toml.rs:45`
- Modify: `crates/core/src/config/mod.rs:3671-3678`
- Regenerate: `crates/core/config.schema.json`, `codex-cli.json`, and any generated config artifacts produced by `just generate`
- Test: config schema/strict-config coverage for the removed field

**Interfaces:**
- Consumes: existing `compact_prompt` runtime path.
- Produces: configuration with no file-based experimental compact-prompt input.

- [ ] **Step 1: Add or update a config validation test for the removal.**

  Assert that `compact_prompt` remains accepted and that `experimental_compact_prompt_file` is no longer part of the generated schema. If strict-config behavior is covered through an existing generic test, extend that test rather than adding a duplicate harness.

- [ ] **Step 2: Remove the top-level and profile TOML fields.**

  Delete both `Option<AbsolutePathBuf>` fields. Keep the surrounding `ConfigToml` and `ConfigProfile` Serde/JsonSchema derives because both structs still contain active fields.

- [ ] **Step 3: Remove the file-loading fallback.**

  Delete the `cfg.experimental_compact_prompt_file` lookup and `try_read_non_empty_file` call used only for this setting. Keep the inline `compact_prompt` resolution and the helper if it is still used for `model_instructions_file`.

- [ ] **Step 4: Regenerate and inspect generated artifacts.**

  Run `just generate`. Confirm that only the removed schema/source-index entries and expected formatting changes are present.

- [ ] **Step 5: Verify the slice.**

  Run `just check --tests`, then `just check`. Confirm `rg -n 'experimental_compact_prompt_file' crates docs codex-cli.json` returns no active references.

### Task 3: Remove legacy top-level config aliases

**Files:**
- Modify: `crates/config/src/config_toml.rs`
- Modify: `crates/config/src/profile_toml.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/features/src/legacy.rs`
- Modify: `crates/features/src/lib.rs`
- Modify: `crates/core/src/config/managed_features.rs`
- Modify: `crates/core/src/config/auth_keyring.rs`
- Regenerate: config schema and generated source indexes
- Test: config, feature, merge, and strict-config tests covering canonical replacements

**Interfaces:**
- Consumes: canonical `tui.disable_paste_burst` and `features.unified_exec` settings.
- Produces: no top-level `disable_paste_burst` fallback and no `experimental_use_unified_exec_tool` compatibility input.

- [ ] **Step 1: Test the canonical paths before removing aliases.**

  Assert that `tui.disable_paste_burst` still controls the runtime setting and `[features].unified_exec` still controls unified exec. Add rejection/ignore coverage for the removed aliases according to strict-config behavior.

- [ ] **Step 2: Remove the top-level paste-burst fallback.**

  Delete `ConfigToml::disable_paste_burst` and change the runtime projection to read only `cfg.tui.disable_paste_burst`.

- [ ] **Step 3: Remove the unified-exec legacy alias.**

  Delete the top-level and profile `experimental_use_unified_exec_tool` fields, remove the corresponding `FeatureConfigSource` plumbing and legacy alias mapping, and preserve the canonical `features.unified_exec` path.

- [ ] **Step 4: Regenerate and verify.**

  Run `just generate`, inspect the schema and source-index diff, then run `just check --tests` and `just check`.

### Task 4: Decide the replacement boundary for `personality`

**Files:**
- Inspect/Modify only after the prerequisite audit: `crates/app-server-protocol/src/protocol/v2/*`, `crates/core/src/session/*`, `crates/tui/src/*`, `crates/models-manager/src/*`, `crates/protocol/src/*`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: current thread/turn personality fields, model instruction templates, session replay, and TUI settings.
- Produces: either an explicit `retain` decision, or a separate design that selects a replacement boundary. There is currently no `personality_v2` or other canonical replacement field.

- [ ] **Step 1: Do not remove the TOML field in the alias-removal slice.**

  The current inventory records `personality` as active. Preserve `ConfigToml::personality`, `ConfigProfile::personality`, and runtime behavior until protocol and session consumers are migrated.

- [ ] **Step 2: Separate the global TOML default from per-thread personality state.**

  `ConfigToml::personality` supplies the initial global default. The runtime `Config.personality` is then consumed by model-instruction rendering, while app-server thread/turn fields and session state can still carry per-thread values. Removing only the TOML fields would make the global default unavailable and effectively use `None`/model defaults; it would not remove the runtime or protocol capability.

- [ ] **Step 3: If the whole concept should be removed, write a separate design first.**

  The design must explicitly choose between existing instruction mechanisms (`instructions`, `developer_instructions`, `model_instructions_file`) and provider/model-owned instruction templates. It must cover `Personality` protocol fields, `none` opt-out behavior, prompt-template rendering, model switching, thread resume/replay, and current tests. Only then decide whether to remove just the TOML default or the runtime/protocol capability as well.

### Task 5: Migrate `provider_base_url` before removal

**Files:**
- Inspect/Modify: `crates/config/src/config_toml.rs`
- Inspect/Modify: `crates/core/src/config/mod.rs`
- Inspect/Modify: credential-broker/network-proxy configuration and consumers
- Modify: `docs/pre-release-compatibility-inventory.md`
- Test: credential-broker host binding and shell-credential redaction coverage

**Interfaces:**
- Consumes: current configured URL and credential-broker trusted-host behavior.
- Produces: an explicitly named provider-owned or credential-broker-owned replacement field.

- [x] **Step 1: Identify the replacement owner and field name.**

  Decide whether the URL belongs under `model_providers.<name>.base_url`, a credential-broker-specific setting, or both. Preserve host validation and prevent trusted local state from being forwarded to arbitrary providers.

- [x] **Step 2: Add migration and security tests.**

  Assert that the replacement preserves trusted-host binding and redaction behavior, while unrelated custom providers cannot inherit the trusted credential behavior accidentally.

- [x] **Step 3: Migrate consumers and fixtures.**

  Update core, credential-broker/network-proxy code, tests, and configuration schema to use the replacement. Keep `provider_base_url` only until all active consumers are migrated.

- [x] **Step 4: Remove the old field after the migration gate passes.**

  Delete `ConfigToml::provider_base_url`, regenerate artifacts, update the inventory to `remove`, then run `just check --tests` and `just check`.

## Completion Criteria

- `experimental_compact_prompt_file` and the selected legacy aliases have no active producers, consumers, schema entries, or generated source-index entries.
- `compact_prompt`, `tui.disable_paste_burst`, and `features.unified_exec` remain functional.
- `personality` is either explicitly retained with evidence or removed through a separate protocol-aware migration.
- `provider_base_url` was removed after its credential-broker security consumers gained the generic credential-provider configuration source.
- The compatibility inventory records the final decision and verification evidence for every changed field.
