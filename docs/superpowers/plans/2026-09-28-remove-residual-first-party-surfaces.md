# Remove Residual First-Party Surfaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the remaining OpenAI-only plugin UI residue and migrate the provider-auth capability name to a provider-neutral first-party boundary without removing active Bedrock, custom-provider, workload-identity, plugin, MCP, or security behavior.

**Architecture:** Keep the existing local/Git/npm/bundled marketplace and trusted script-attribution implementation. Remove the hard-coded Apps help disclosure, rename the shared auth capability from an OpenAI-specific label to `requires_first_party_auth`, and retain the explicit OpenAI-compatible adapter as a documented opt-in capability until its active auth consumers can be split safely.

**Tech Stack:** Rust workspace, generated config/protocol artifacts, `just generate`, `just check --tests`, `just check`, `rg`, and `git diff --check`.

**Spec:** `docs/openai-decoupling-candidates.md` and `docs/pre-release-compatibility-inventory.md`.

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Audit producers and consumers before changing code; record `remove`, `retain`, or `migrate-in-place` decisions in the compatibility inventory.
- Preserve provider-neutral marketplace loading, plugin trust, hooks, MCP/OAuth/elicitation, workload identity, Bedrock, custom providers, and security fallbacks.
- Run `just generate` when checked-in generated artifacts are affected.
- Run `just check --tests` after Rust code or test-target changes, and always run `just check`.
- Do not run standalone test commands for this cleanup unless explicitly requested.

## Review Focus

- An uninstalled plugin detail popup no longer links to an OpenAI Apps article or promises an unavailable hosted policy page.
- Configured non-OpenAI providers still use the generic base-URL warning and do not lose model selection behavior.
- Existing `requires_first_party_auth` consumers continue to distinguish first-party account/API-key auth from unauthenticated custom providers and AWS auth.
- Generated config/protocol artifacts use the migrated field consistently.
- Bundled marketplace loading, trusted runtime attribution, ordinary MCP/plugin behavior, and security checks remain unchanged.

### Task 1: Remove OpenAI-only plugin UI and test-only marketplace names

**Files:**
- Modify: `crates/tui/src/chatwidget/plugin_catalog.rs`, `crates/tui/src/chatwidget/model_popups.rs`.
- Modify: `crates/core-plugins/src/lib.rs` and its test-only consumers that import the curated constants.
- Modify: related TUI snapshots if the removed disclosure is rendered there.

**Interfaces:**
- Consumes: existing `ModelProviderInfo::base_url` and plugin detail rendering.
- Produces: provider-neutral model URL warning; no OpenAI Apps disclosure line; curated test fixtures named as test marketplaces.

- [ ] **Step 1: Remove the Apps-specific disclosure and make the model URL warning provider-neutral.** Delete the hard-coded Apps article link and disclosure renderer. Rename the model popup helper and message to refer to the configured provider rather than OpenAI, and warn for any explicitly configured provider base URL.
- [ ] **Step 2: Replace only test-only curated marketplace constants and values with neutral test names.** Keep production `openai-bundled` and `openai-primary-runtime` paths because active loader, policy, and trusted-attribution consumers depend on them.
- [ ] **Step 3: Run generated/snapshot checks required by the changed UI and inspect the diff.**

### Task 2: Migrate the auth capability name in place

**Files:**
- Modify all current Rust/config/protocol consumers of `requires_openai_auth` to use `requires_first_party_auth`.
- Modify: `crates/config/src/thread_config/proto/codex.thread_config.v1.proto` and generated Rust artifacts.
- Modify: `crates/cli/src/doctor.rs`, status/account copy, and provider metadata docs where the old label is user-visible.
- Modify: `docs/openai-decoupling-candidates.md` and `docs/pre-release-compatibility-inventory.md`.

**Interfaces:**
- Consumes: existing `ModelProviderInfo`, app-server protocol, thread-config serialization, and auth/provider routing.
- Produces: the same boolean behavior under the provider-neutral field `requires_first_party_auth`; explicit OpenAI-compatible auth remains opt-in.

- [ ] **Step 1: Rename the field and all current producers/consumers mechanically, preserving field numbers and behavior.** Keep `OPENAI_API_KEY`, ChatGPT workload-identity claims, account headers, and explicit `create_openai_provider` because their active consumers are first-party/security paths.
- [ ] **Step 2: Regenerate checked-in schemas/protobuf-derived artifacts with `just generate`.** Confirm the migrated field appears consistently and no current production source references the old identifier.
- [ ] **Step 3: Record the producer/consumer decision in the inventory.** Mark the naming migration complete while retaining active first-party auth behavior and the explicit adapter as `retain`/`migrate-in-place`, and explain why crate-wide deletion is unsafe at this stage.

### Task 3: Verify, commit, and push

- [ ] **Step 1:** Run `just check --tests`, `just check`, and `git diff --check`; inspect all output and the final diff.
- [ ] **Step 2:** Commit the completed cleanup on `main`.
- [ ] **Step 3:** Push `main` to its configured upstream.
