# Remove ChatGPT Contract Holds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the remaining ChatGPT-specific auth, moderation, and remote-client compatibility paths while preserving generic coding-agent, provider, security, and Marketplace behavior.

**Architecture:** Replace the ChatGPT account header with an explicit optional identity on externally managed auth headers, so account continuity remains provider-neutral and no wire header is invented. Remove the provider moderation metadata event path and the deleted Remote Control redaction branch, then update generated protocol artifacts and fixtures.

**Tech Stack:** Rust workspace, `rg`, `apply_patch`, checked-in app-server schemas, `just check --tests`, `just check`, and `just generate`.

**Spec:** `docs/superpowers/specs/2026-10-01-remove-chatgpt-rust-residuals-design.md`, amended by the user's explicit decision that the three previously held contracts are out of scope.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Do not add a replacement provider or ChatGPT wire field.
- Preserve generic external-auth header forwarding and optional account identity support.
- Preserve safety buffering, model verification, Marketplace, MCP, plugin, and other provider-neutral coding-agent behavior.
- Update the compatibility inventory before changing the affected production paths.
- Run `just check --tests` for Rust changes, `just check` always, and `just generate` for protocol/generated-artifact changes.
- Do not run standalone tests.
- Commit each coherent slice with its inventory and verification changes.

## Review Focus

- Generic external auth must continue forwarding arbitrary provider-owned headers without adding a ChatGPT-specific header.
- Account continuity and model-cache separation must remain safe when an external auth provider supplies an explicit generic identity.
- Removing the moderation event must not remove safety buffering or model verification behavior.
- Removing Remote Control compatibility must not alter ordinary thread resume behavior.
- Generated app-server schemas must not retain the removed moderation notification.

### Task 1: Reclassify the held contracts

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Create: `.superpowers/sdd/2026-10-01-remove-chatgpt-contract-holds/progress.md`

- [x] Record that the exact account header, moderation metadata key/event, and named remote clients are `remove`, not `hold`, because their producers are deleted or outside this fork's product scope.
- [x] Run the pre-change negative guard proving the old Rust matches still exist.
- [x] Commit the inventory decision.

### Task 2: Remove the ChatGPT account-header adapter

**Files:**
- Modify: `crates/login/src/auth/auth_headers.rs`
- Modify: `crates/login/src/auth/manager.rs`
- Modify: `crates/login/src/auth/change_state.rs`
- Modify: `crates/model-provider/src/bearer_auth_provider.rs`
- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/models_identity.rs`
- Modify: `crates/model-provider/src/amazon_bedrock/auth.rs`
- Modify: `crates/codex-mcp/src/server.rs`
- Modify: `crates/codex-mcp/src/runtime.rs`
- Modify: affected auth fixtures and tests under `crates/core`, `crates/cli`, `crates/app-server`, `crates/codex-api`, and `crates/tui`

- [x] Add the generic optional identity field/accessor to `AuthHeaders` and update external-auth tests to use it instead of a wire header.
- [x] Verify the old exact-header assertion fails after the fixture is made provider-neutral.
- [x] Remove `ChatGPT-Account-ID`, the FedRAMP-only bearer fields, and their tests; preserve arbitrary header forwarding and cache identity hashing.
- [x] Run `just check --tests`, `just check`, and `git diff --check`; commit the auth slice.

### Task 3: Remove the provider moderation metadata path

**Files:**
- Modify: `crates/codex-api/src/sse/responses.rs`
- Modify: `crates/codex-api/src/endpoint/responses_websocket.rs`
- Modify: `crates/codex-api/src/common.rs`
- Modify: `crates/protocol/src/protocol.rs`
- Modify: `crates/core/src/session/mod.rs`
- Modify: `crates/core/src/session/turn.rs`
- Modify: `crates/otel/src/events/session_telemetry.rs`
- Modify: `crates/rollout/src/policy.rs`
- Modify: `crates/rollout-trace/src/protocol_event.rs`
- Modify: `crates/app-server/src/bespoke_event_handling.rs`
- Modify: `crates/app-server/src/notification_media.rs`
- Modify: `crates/app-server-protocol/src/protocol/common.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/model.rs`
- Delete/update: moderation fixtures and test assertions
- Regenerate: checked-in app-server protocol schemas

- [x] Remove the parser, response event, core event, rollout handling, app-server notification, and test fixture for the ChatGPT-only moderation metadata.
- [x] Verify safety buffering and model verification paths remain present.
- [x] Run `just generate`, inspect the generated diff, then run `just check --tests`, `just check`, and commit the protocol slice.

### Task 4: Remove the deleted Remote Control client branch

**Files:**
- Modify: `crates/app-server/src/request_processors.rs`
- Delete: `crates/app-server/src/request_processors/thread_resume_redaction.rs`
- Modify: `crates/app-server/src/request_processors/thread_processor.rs`
- Modify: `crates/app-server/src/request_processors/thread_lifecycle.rs`
- Modify/delete: `crates/app-server/tests/suite/v2/thread_resume.rs`

- [x] Remove the client-name predicate and response-only redaction state from thread resume.
- [x] Verify ordinary thread resume still returns persisted MCP/image items unchanged.
- [x] Run the required checks and commit the Remote Control slice.

### Task 5: Remove stale hosted fixtures and close the audit

**Files:**
- Modify/delete: residual fixture files under `crates/app-server/tests/suite/v2`, `crates/cli/tests`, `crates/core/tests`, and `crates/tui/src/app/tests`
- Modify: `docs/provider-decoupling-candidates.md` if needed
- Modify: `docs/pre-release-compatibility-inventory.md`
- Regenerate: only artifacts changed by `just generate`

- [x] Remove exact header assertions from generic tests and delete tests whose sole producer/consumer was deleted hosted Apps or Remote Control behavior; retain Marketplace tests.
- [x] Run final case-insensitive Rust search and confirm zero matches.
- [x] Run `just generate`, `just check --tests`, `just check`, `git diff --check`, inspect status and all commits, then commit the final audit if documentation or generated output changed.
