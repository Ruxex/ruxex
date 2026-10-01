# Backend Account and Usage Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the remaining provider backend account, usage, workspace, Analytics, and turn-cost capabilities, then delete their dedicated crates.

**Architecture:** Delete the app-server protocol methods and the UI paths that invoke them instead of leaving disabled stubs. Preserve provider-neutral live rate-limit information carried in core events, but remove account-scoped backend reads, reset-credit actions, usage dashboards, and workspace headlines. With all direct consumers gone, remove `codex-backend-client` and `codex-backend-openapi-models` from the workspace.

**Tech Stack:** Rust workspace, Cargo manifests, app-server protocol bindings.

**Spec:** `docs/provider-decoupling-plan.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Treat this as crate-removal work; do not add deletion-specific tests.
- Do not run tests; use `just check` for verification.
- Retain provider-neutral core rate-limit events and display paths.

## Review Focus

- Live provider rate-limit snapshots must continue to reach the TUI without the account read RPC.
- No TUI command, event, or background request may refer to removed account usage, workspace headline, reset-credit, or Analytics APIs.
- No workspace manifest or Rust source may retain a `codex-backend-client` or generated OpenAPI-model dependency.
- Generated protocol exports must not mention deleted RPC methods or payload types.

### Task 1: Remove account/usage/workspace protocol and app-server handlers

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/common.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/account.rs`
- Modify: `crates/app-server/src/message_processor.rs`
- Modify: `crates/app-server/src/request_processors.rs`
- Modify: `crates/app-server/src/request_processors/account_processor.rs`
- Delete: `crates/app-server/src/request_processors/account_processor/rate_limit_resets.rs`
- Delete: account/usage and reset-credit app-server tests that only exercise deleted RPCs.

- [ ] Remove the four account backend request variants (`account/rateLimits/read`, reset-credit consume, `account/usage/read`, and add-credits nudge), the workspace-message read variant, their protocol DTOs, protocol export assertions, app-server dispatch arms, and processor methods.
- [ ] Retain `AccountRateLimitsUpdatedNotification` because it relays provider-neutral core events.
- [ ] Remove imports, module declarations, and test-only helpers that become unreachable.

### Task 2: Remove TUI hosted account surfaces

**Files:**
- Delete: `crates/tui/src/analytics.rs` and `crates/tui/src/analytics/`
- Modify: `crates/tui/src/app.rs`, `crates/tui/src/app_event.rs`, `crates/tui/src/app/event_dispatch.rs`, and `crates/tui/src/app/background_requests.rs`
- Modify: `crates/tui/src/chatwidget.rs` and affected `crates/tui/src/chatwidget/` modules
- Delete: `crates/tui/src/workspace_messages.rs` and its tests
- Modify: `crates/tui/src/lib.rs`

- [ ] Remove Analytics opening, rendering, event dispatch, background loading, and its dedicated test/snapshot tree.
- [ ] Remove `/usage` account refresh, reset-credit selection/consumption, credit-email actions, and workspace-headline polling while retaining live rate-limit status updates produced from core events.
- [ ] Remove all now-unreachable imports and protocol references.

### Task 3: Remove the backend-client boundary and document the result

**Files:**
- Delete: `crates/backend-client/`
- Delete: `crates/codex-backend-openapi-models/`
- Modify: `Cargo.toml`, `crates/app-server/Cargo.toml`, `crates/tui/Cargo.toml`, and `Cargo.lock`
- Modify: `docs/provider-decoupling-plan.md` and `docs/provider-specific-crates.md`

- [ ] Delete the provider turn-cost worker and its tests, which are the remaining app-server consumer of the backend client.
- [ ] Remove both workspace members and every dependency declaration after source references are gone.
- [ ] Record that account/usage/workspace boundaries and both backend crates are removed; retain Phase 3 as in progress because its later integrations remain.
- [ ] Run `just crate-dependency-graph` and `just check`; do not run tests.
