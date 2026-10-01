# Static Hawk Public API Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove statically unreferenced deprecated public aliases identified by the checked-in Hawk result, while excluding OS-specific, dynamic, generated, protocol, and test-support surfaces.

**Architecture:** Use the existing producer/consumer audit as the first gate, then remove only the three `codex-core` compatibility aliases whose definitions are their only repository references. Compile the workspace and test targets after the change; keep all provider-neutral runtime, platform, protocol, and generated APIs untouched.

**Tech Stack:** Rust workspace, Cargo, `just check`, checked-in Hawk JSON, Markdown compatibility inventory.

**Spec:** `docs/provider-decoupling-plan.md`, `docs/pre-release-compatibility-inventory.md`, and `docs/hawk-codex-cli-result-analysis.md`.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Treat the checked-in Hawk result as a candidate list, not as proof of global unreachability.
- Exclude OS-specific code, dynamic dispatch/registration/string lookup, generated sources and schemas, protocol/app-server APIs, MCP APIs, and test-support APIs from this cleanup unit.
- Preserve unrelated working-tree changes.
- Use `just check` as the production workspace type-check and `just check --tests` when Rust code changes; do not run standalone tests.
- Update the compatibility inventory with the exact producer/consumer audit and verification result.

## Review Focus

- Deprecated aliases must have no repository consumer beyond their own definitions; verify with exact-symbol searches.
- The canonical `ThreadManager`, `NewThread`, and `CodexThread` exports must remain unchanged.
- No protocol, generated schema, OS fallback, or test-support symbol may be changed as collateral.
- Existing user changes in `AGENTS.md`, `docs/provider-specific-crates.md`, and the untracked CLI architecture document must remain untouched.

### Task 1: Remove unused codex-core compatibility aliases

**Files:**
- Modify: `crates/core/src/lib.rs:122-127`
- Modify: `docs/pre-release-compatibility-inventory.md:101`

**Interfaces:**
- Consumes: the canonical `ThreadManager`, `NewThread`, and `CodexThread` exports already defined in `crates/core/src/lib.rs`.
- Produces: no compatibility aliases named `ConversationManager`, `NewConversation`, or `CodexConversation`.

- [x] **Step 1: Audit the exact symbols**

Run:

```bash
rg -n 'ConversationManager|NewConversation|CodexConversation' crates --glob '*.rs'
```

Expected: only the three deprecated definitions in `crates/core/src/lib.rs` are found; no test, binary, protocol, dynamic, generated, or platform consumer is present.

- [x] **Step 2: Remove the compatibility aliases**

Delete the three deprecated `pub type` definitions from `crates/core/src/lib.rs`, leaving the canonical exports unchanged.

- [x] **Step 3: Record the completed decision**

Change the matching inventory row from `remove (planned)` to `remove (completed)` and record the actual verification commands without changing unrelated rows.

- [x] **Step 4: Run the required verification**

Run:

```bash
just check --tests
just check
```

Expected: both commands exit successfully with no Rust compilation errors attributable to the alias removal.

- [x] **Step 5: Inspect the final diff**

Run:

```bash
rg -n 'ConversationManager|NewConversation|CodexConversation' crates --glob '*.rs'
git diff --check
git diff -- crates/core/src/lib.rs docs/pre-release-compatibility-inventory.md
```

Expected: the exact-symbol search returns no matches, whitespace validation succeeds, and the diff contains only the three alias removals plus the inventory status/verification update.
