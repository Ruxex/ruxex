# Remove Configured Compatibility Surfaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove the explicitly rejected user-configurable compatibility surfaces while retaining all other configuration and current behavior.

**Architecture:** Treat configuration removal and runtime/backend removal separately. Delete obsolete config producers, schema entries, merge/CLI paths, and dead compatibility branches first; remove the zsh-fork execution backend and its packaged wrapper only after its producers and consumers are enumerated. Keep provider-neutral execution safety, direct shell/unified-exec behavior, Code Mode, V1 agent behavior, and all retained prompt/context/model/MCP settings.

**Tech Stack:** Rust workspace, TOML/JSON schema generation, shell-escalation wrapper, app-server/TUI integration tests.

**Spec:** `docs/config-audit.md` and the deletion decisions recorded in `docs/pre-release-compatibility-inventory.md`.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- This is an unreleased fork; remove repository-owned compatibility surfaces only after producer/consumer audit.
- Preserve execution safety, provider-neutral capabilities, external endpoint contracts, and retained configuration behavior.
- Run Rust build/check commands only through the repository `just` recipes and the user systemd memory scope.
- Run `just check --tests` after Rust changes, `just check` always, and `just generate` when checked-in generated artifacts may change.

## Review Focus

- Removing a config field must not remove the corresponding protocol or managed-instruction field when that field is still an active runtime contract.
- Removing project-root/fallback overrides must leave the fixed default project discovery behavior intact.
- Removing token-limit override must preserve the context manager's safe built-in limit.
- Removing V2 config controls must not accidentally remove V1 or internal V2 execution code that remains outside user configuration.
- Removing zsh fork must remove its wrapper and packaged artifacts without weakening direct sandbox/approval execution.

### Task 1: Persist the deletion decisions and establish invariants

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md` if a new candidate entry is required by the producer audit
- Modify: `docs/config-audit.md`

- [x] Record `remove` decisions for the four top-level config fields, the V2 config surface, obsolete compatibility Feature config keys, and the zsh-fork backend with exact producer/consumer paths.
- [x] Record retained current behavior for project discovery, context output limits, direct execution, Code Mode, and V1 agents.
- [x] Define repository-wide search invariants for old config keys and zsh-fork runtime names.

### Task 2: Remove top-level obsolete configuration overrides

**Files:**
- Modify: `crates/config/src/config_toml.rs`
- Modify: `crates/config/src/defaults.toml`
- Modify: `crates/config/src/loader/**`
- Modify: context-manager/config projection consumers found by exact search
- Delete or simplify: `crates/config/src/project_root_markers.rs` when no consumer remains
- Modify: related config/schema/merge/loader tests

- [x] Remove `developer_instructions` as a user config field while retaining protocol, managed requirements, role-file, and built-in developer instruction paths that remain active.
- [x] Remove `project_doc_fallback_filenames` and `project_root_markers` parsing/merging while hard-coding the retained project instruction/root discovery defaults.
- [x] Remove `tool_output_token_limit` parsing and override propagation while preserving the built-in context-manager safety limit.
- [x] Regenerate checked-in schemas and update tests/fixtures.

### Task 3: Remove V2-only user configuration

**Files:**
- Modify: `crates/features/src/lib.rs`, `crates/features/src/feature_configs.rs`, `crates/features/src/tests.rs`
- Modify: `crates/config/src/schema.rs`, `crates/config/src/merge.rs`, `crates/config/src/config_toml.rs`
- Modify: `crates/core/src/config/mod.rs` and V2 config consumers/tests

- [x] Remove the user-configurable `features.multi_agent_v2` TOML table and its nested override fields from schema/merge/config resolution.
- [x] Preserve internal V2 types and execution code only where needed by runtime/host-controlled paths; use immutable built-in defaults for fields no longer configurable.
- [x] Remove config-only compatibility tests and add rejection/absence coverage for the removed TOML paths.

### Task 4: Remove obsolete compatibility Feature overrides

**Files:**
- Modify: `crates/features/src/lib.rs` and feature registry/schema generation
- Modify: each producer/consumer found for `undo`, `terminal_resize_reflow`, `apply_patch_preserve_line_endings`, `use_linux_sandbox_bwrap`, `remote_models`, `tool_search`, `tool_search_always_defer_mcp_tools`, `resize_all_images`, `item_ids`, `steer`, `send_async_message`, `collaboration_modes`, and `remote_compaction_v2`
- Modify: related tests and generated schemas

- [x] Remove obsolete config keys and compatibility-only normalization/merge paths.
- [x] Preserve the current always-on behavior where the old flag is now a no-op gate.
- [x] Preserve external beta headers or wire identifiers where the audit identifies them as active contracts.

### Task 5: Remove the zsh-fork execution backend

**Files:**
- Modify: `crates/tools/src/tool_config.rs`, `crates/core/src/tools/**`, `crates/core/src/session/**`, `crates/core/src/config/**`, `crates/arg0/**`, `crates/install-context/**`, `crates/cli/**`, `crates/tui/**`, and related manifests
- Delete: zsh-fork runtime modules, patched-zsh wrapper support, zsh-fork-only fixtures/tests, and `crates/shell-escalation` only after confirming no retained consumer
- Modify: release/build manifests and generated app-server/config schemas

- [x] Remove `shell_zsh_fork` and `unified_exec_zsh_fork` Feature entries and all config gates.
- [x] Remove the `ZshFork` shell mode, escalation bridge, patched-zsh packaging, and zsh-fork-only integration tests.
- [x] Keep direct shell/unified-exec execution, sandbox policy, approvals, and credential/network safety behavior unchanged.

### Task 6: Generate and verify

- [x] Run `just generate` and inspect all generated diffs.
- [x] Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests`.
- [x] Run the equivalent memory-scoped `just check`.
- [x] Search for removed config keys and zsh-fork runtime symbols; classify only external/historical references that must remain.
- [x] Inspect `git diff --check` and update the audit/inventory records with verification results.
