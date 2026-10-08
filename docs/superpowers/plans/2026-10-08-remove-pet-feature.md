# Remove Terminal Pet Feature Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove every executable terminal Pet surface while preserving unrelated TUI, configuration, image, URL, UUID, hashing, and terminal-detection capabilities.

**Architecture:** Delete the Pet-owned module tree and unwind its producers and consumers from the configuration, core projection, TUI event loop, ChatWidget, rendering, session logging, and tests. Regenerate checked-in schemas and indexes after source removal, then use the repository's memory-limited checks to catch stale references and accidental dependency removal.

**Tech Stack:** Rust workspace, serde/schemars configuration types, ratatui/crossterm TUI, checked-in generated JSON/schema artifacts, `just` verification recipes.

**Spec:** `docs/superpowers/specs/2026-10-08-remove-pet-feature-design.md`

## Global Constraints

- Work directly on the existing `main` checkout; do not create a worktree.
- Preserve all pre-existing unrelated working-tree changes.
- This is an unreleased repository-owned feature; remove Pet configuration without compatibility aliases or migrations.
- Retain generic, historical, and external uses of the word `pet` unless they are proven runtime Pet producers or consumers.
- Run Cargo-backed commands only through a user systemd scope with `MemoryHigh=16G`, `MemoryMax=18G`, `MemorySwapMax=4G`; use `CARGO_BUILD_JOBS=2` for workspace checks.
- Use `just generate`, `just check --tests`, `just check`, and `git diff --check`; do not run standalone tests.

## Review Focus

- Stale `AppEvent` variants or session-log arms must disappear together so event conversion and logging remain exhaustive.
- Removed `[tui].pet` and `[tui].pet_anchor` fields must disappear from both typed config projection and generated schema/public re-exports.
- TUI shutdown, redraw, resize, transcript wrapping, and normal notification behavior must not retain Pet-only state or cleanup calls.
- Generated schema/index changes must contain only the intended Pet removal and current pre-existing changes.
- Shared `image`, `base64`, `sha2`, `uuid`, `url`, and terminal-detection dependencies must remain when other production consumers still use them.

---

### Task 1: Remove Pet configuration and public API

**Files:**
- Modify: `crates/config/src/types.rs`
- Modify: `crates/core-api/src/lib.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/core/src/config/edit.rs`
- Modify: `crates/core/config.schema.json` via `just generate`
- Modify: generated schema/index artifacts via `just generate`

**Interfaces:**
- Removes `TuiPetAnchor`, `TuiConfig.pet`, `TuiConfig.pet_anchor`, `Config.tui_pet`, `Config.tui_pet_anchor`, `tui_pet_edit`, and the `core-api` re-export.
- Leaves all other TUI configuration fields and config edit builders unchanged.

- [x] **Step 1: Remove the Pet-specific config type and fields**

Delete `TuiPetAnchor` and the `pet`/`pet_anchor` fields from the config types. Remove the corresponding `genex-core` imports, projected fields, defaults, and assignments in `config/mod.rs`.

- [x] **Step 2: Remove the public re-export and config edit builder**

Delete the `TuiPetAnchor` re-export from `core-api`, remove `tui_pet_edit`, and remove only its focused unit-test assertion from `crates/core/src/config/edit.rs`.

- [x] **Step 3: Regenerate configuration artifacts**

Run `just generate` under the required systemd scope, inspect the diff, and confirm the generated schema no longer defines `pet`, `pet_anchor`, or `TuiPetAnchor`.

- [x] **Step 4: Verify config-only compilation**

Run the workspace `just check --tests` command under the required systemd scope after the remaining tasks are complete; do not run an intermediate standalone Cargo command.

### Task 2: Remove ChatWidget commands, state, and selection flow

**Files:**
- Delete: `crates/tui/src/chatwidget/pets.rs`
- Delete: `crates/tui/src/chatwidget/pets_tests.rs`
- Delete: `crates/tui/src/app/pets.rs`
- Delete: `crates/tui/src/pets/**`
- Modify: `crates/tui/src/lib.rs`, `crates/tui/src/app.rs`, `crates/tui/src/chatwidget.rs`
- Modify: `crates/tui/src/chatwidget/constructor.rs`
- Modify: `crates/tui/src/chatwidget/slash_dispatch.rs`
- Modify: `crates/tui/src/slash_command.rs`
- Modify: `crates/tui/src/app_event.rs`, `crates/tui/src/app/event_dispatch.rs`
- Modify: `crates/tui/src/app/config_persistence.rs`, `crates/tui/src/local_settings.rs`

**Interfaces:**
- Removes the `/pets` and `/pet` command, Pet picker/selection methods, Pet app events, Pet-specific async loading, and Pet-related ChatWidget/App state.
- Preserves all other slash commands, selection views, app events, and configuration synchronization.

- [x] **Step 1: Remove the Pet modules and declarations**

Delete the Pet module tree and its test file, remove `mod pets` declarations, and remove the Pet-specific module import/references from the TUI root and App.

- [x] **Step 2: Unwind ChatWidget construction and state**

Remove the Pet HTTP client, ambient Pet fields, preview/loading request IDs, support overrides, constructor setup, and Pet helper module inclusion. Keep unrelated terminal detection and HTTP clients used by other ChatWidget paths.

- [x] **Step 3: Remove commands and event flow**

Remove the `SlashCommand::Pets` variant and description/aliases, its dispatch branches, all Pet `AppEvent` variants, corresponding event-dispatch arms, and the Pet app handler module. Remove the Pet config persistence sync helpers and Pet fields from `LocalSettings`.

- [x] **Step 4: Remove Pet-specific tests**

Delete the dedicated Pet tests and remove Pet-only cases/helpers from `crates/tui/src/chatwidget/tests/slash_commands.rs` and other TUI tests. Keep test coverage for neighboring slash commands and selection behavior.

### Task 3: Remove rendering, lifecycle, and notification integration

**Files:**
- Modify: `crates/tui/src/tui.rs`
- Modify: `crates/tui/src/app.rs`
- Modify: `crates/tui/src/chatwidget/rendering.rs`
- Modify: `crates/tui/src/chatwidget/constructor.rs`
- Modify: `crates/tui/src/chatwidget/turn_runtime.rs`
- Modify: `crates/tui/src/chatwidget/tool_requests.rs`
- Modify: `crates/tui/src/chatwidget.rs`
- Modify: `crates/tui/src/session_log.rs`
- Modify: `crates/tui/src/app/tests.rs`

**Interfaces:**
- Removes ambient image rendering, preview image clearing, Pet animation scheduling, Pet notification calls, Pet transcript-width reservation, and Pet session-log serialization.
- Leaves standard TUI drawing, terminal cleanup, transcript wrapping, tool-request notifications, and session logging intact.

- [x] **Step 1: Remove TUI image state and methods**

Delete Pet image render state fields, the TUI `Drop` cleanup call, Pet draw/clear methods, and Pet-specific error handling. Keep the normal terminal draw and shutdown flow.

- [x] **Step 2: Remove App redraw and shutdown branches**

Delete the ambient/preview image branches from the draw loop and Pet shutdown/error calls. Ensure the regular `render_chat_widget_frame` and shutdown feedback sequence remains ordered as before.

- [x] **Step 3: Remove ChatWidget layout and animation hooks**

Remove frame scheduling for the ambient Pet, notification setters and calls from tool/turn runtime producers, Pet reserved columns from transcript rendering, and the Pet-specific resize/reflow test.

- [x] **Step 4: Remove Pet session-log serialization**

Delete only the `PetPreviewLoaded` and `PetSelectionLoaded` serialization arms; retain the generic fallback variant logging and all unrelated event records.

### Task 4: Audit dependency and documentation surfaces

**Files:**
- Modify: `crates/tui/Cargo.toml` only if a dependency is proven Pet-only
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Retain: generic image-generation samples and unrelated historical/external catalog text

**Interfaces:**
- Records the producer/consumer audit and explicit `remove` decision required by the repository instructions.
- Does not remove shared dependencies solely because Pet used them.

- [x] **Step 1: Confirm dependency consumers**

Use repository-wide searches to verify each candidate dependency/import. Remove only Pet-exclusive imports or dependency declarations; retain `image`, `base64`, `sha2`, `uuid`, `url`, and terminal detection where other production consumers remain.

- [x] **Step 2: Update the candidate queue**

Add or revise the Pet/CDN candidate entry with exact production paths and producer/consumer evidence, marking the feature removal as `remove` and distinguishing retained generic/historical text.

- [x] **Step 3: Update the compatibility inventory**

Add the corresponding explicit `remove` decisions for persisted/config/schema/UI/runtime Pet surfaces in the inventory's required audit order. Do not alter unrelated entries.

### Task 5: Generate and verify the completed removal

**Files:**
- Modify: any checked-in generated artifacts emitted by `just generate`
- No new production files

- [x] **Step 1: Search for residual executable Pet references**

Run targeted `rg` searches for `TuiPetAnchor`, `tui_pet`, `pet_anchor`, `AppEvent::Pet`, `SlashCommand::Pets`, `crate::pets`, `ambient_pet`, `tui-pets`, the Pet CDN URL, and Pet-specific event-log variants. Review each remaining match and document why generic/historical/external matches are retained.

- [x] **Step 2: Regenerate artifacts and inspect the diff**

Run:

```sh
systemd-run --user --scope --collect --quiet \
  -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
  --setenv=CARGO_BUILD_JOBS=2 just generate
```

Inspect all generated changes and ensure no unrelated pre-existing modifications are overwritten.

- [x] **Step 3: Compile test targets without executing them**

Run:

```sh
systemd-run --user --scope --collect --quiet \
  -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
  --setenv=CARGO_BUILD_JOBS=2 just check --tests
```

Expected result: success with no unresolved Pet symbols or stale generated config references.

- [x] **Step 4: Run the production workspace check**

Run the same memory-limited scope with `just check`.

- [x] **Step 5: Check formatting and working-tree scope**

Run `git diff --check`, inspect `git status --short`, and verify the pre-existing unrelated changes remain present. No standalone tests are run.
