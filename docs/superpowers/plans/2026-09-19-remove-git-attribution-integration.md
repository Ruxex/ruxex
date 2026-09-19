# Git Attribution Integration Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the OpenAI backend-controlled git attribution integration and its dedicated crate.

**Architecture:** Remove the extension registration at both application entry points, then remove the `codex-git-attribution` workspace member and its dependency declarations. The resulting provider-neutral runtime contributes no backend-fetched commit/PR attribution instruction.

**Tech Stack:** Rust workspace, Cargo, just.

**Spec:** `docs/openai-decoupling-plan.md` (Level 1, Phase 3)

## Global Constraints

- Keep shared architecture provider-neutral.
- Delete one hosted integration boundary; do not refactor unrelated authentication or transport code.
- Use `cargo check` and repository-wide `just check` as primary verification for crate removal.

## Review Focus

- App-server startup must no longer construct or register a git-attribution extension.
- CLI startup must no longer register the extension.
- No workspace member or dependency declaration may reference `codex-git-attribution`.
- Existing non-attribution extensions must retain their registration path.
- The workspace dependency graph must regenerate without the removed crate.

---

### Task 1: Remove application registration and its dedicated crate

**Files:**
- Modify: `crates/app-server/src/extensions.rs`
- Modify: `crates/app-server/Cargo.toml`
- Modify: `crates/cli/Cargo.toml`
- Modify: `Cargo.toml`
- Delete: `crates/ext/git-attribution/Cargo.toml`
- Delete: `crates/ext/git-attribution/src/lib.rs`
- Delete: `crates/ext/git-attribution/src/policy.rs`
- Delete: `crates/ext/git-attribution/src/world_state.rs`
- Delete: `crates/ext/git-attribution/src/git_attribution_tests.rs`

**Interfaces:**
- Consumes: app-server extension construction parameters.
- Produces: no `codex-git-attribution` workspace member or runtime registration.

- [x] **Step 1: Remove the app-server extension registration and configuration inputs used only by it.**

Delete `git_attribution_base_url`, `http_client_factory`, and `auth_manager` from the extension install input only where they are solely passed to `codex_git_attribution::install`, then remove that call.

- [x] **Step 2: Remove CLI and app-server dependencies.**

Delete `codex-git-attribution = { workspace = true }` from both manifests and remove its root workspace dependency/member declarations.

- [x] **Step 3: Delete the integration crate.**

Delete the dedicated crate source and manifest once no registrations or manifests reference it.

- [x] **Step 4: Regenerate the dependency graph.**

Run: `just crate-dependency-graph`

Expected: the generated graph reports one fewer crate and contains no `codex-git-attribution` node.

### Task 2: Verify removal

**Files:**
- Modify: `docs/openai-decoupling-plan.md` only if this completes the whole Phase 3 (it does not).

**Interfaces:**
- Consumes: the edited workspace.
- Produces: evidence that affected packages and the workspace build without the integration.

- [x] **Step 1: Format and check affected packages.**

Run: `cargo fmt --check && cargo check -p codex-app-server -p codex-cli`

Expected: PASS.

- [x] **Step 2: Run repository-wide validation.**

Run: `just check`

Expected: PASS.

- [x] **Step 3: Inspect the final diff and references.**

Run: `git diff --check && rg -n 'codex-git-attribution|codex_git_attribution' --glob '!target/**'`

Expected: no references outside historical documentation, and no whitespace errors.
