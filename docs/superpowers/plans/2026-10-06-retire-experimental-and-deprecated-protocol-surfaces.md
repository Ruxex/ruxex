# Experimental and Deprecated Protocol Surface Retirement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove the unused `gitDiffToRemote` and test-only `mock/experimentalMethod` protocol surfaces, retire obsolete deprecated request APIs after their in-repository consumers are migrated, and keep the two legacy notifications that still have replay/raw-event compatibility implications.

**Architecture:** Treat `crates/app-server-protocol/src/protocol/common.rs` as the wire-contract source of truth. Audit and update app-server processors, TUI/exec compatibility match arms, integration-test helpers, and generated schemas together; do not replace `gitDiffToRemote` with `turn/diff/updated` because their diff scopes differ. Preserve the internal `genex_git_utils::git_diff_to_remote` helper used by core Git-state tests unless the final consumer audit proves it is independently dead.

**Tech Stack:** Rust workspace, JSON-RPC app-server protocol, checked-in JSON/TypeScript schemas, `just generate`, `just check --tests`, and `just check`.

**Spec:** `docs/pre-release-compatibility-inventory.md`, `docs/provider-decoupling-candidates.md`, and the user-approved protocol audit in this thread.

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Record an explicit `remove`, `retain`, or `migrate-in-place` decision in `docs/pre-release-compatibility-inventory.md` before changing production protocol code.
- Preserve current provider-neutral behavior, sandbox/security behavior, persisted readers, and the OpenAI endpoint boundary.
- Do not run standalone tests; use `just check`, `just check --tests`, and `just generate` as required by `AGENTS.md`.

## Review Focus

- `gitDiffToRemote` must not be confused with `turn/diff/updated` or TUI `/diff`; verify that no current TUI/CLI caller relies on the removed wire method.
- Removing `mock/experimentalMethod` must retain real experimental-capability gating coverage through existing production experimental methods.
- `getAuthStatus` test migration must not reintroduce token material into the newer `account/read` response.
- Legacy approval request variants must have no remaining producer before their protocol definitions and consumer match arms are removed.
- `item/fileChange/outputDelta` and `thread/compacted` remain retained until raw-event/rollout replay compatibility is explicitly re-audited.

### Task 1: Record the protocol decisions before code changes

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

**Interfaces:**
- Records the audited producer/consumer evidence and decisions used by Tasks 2–5.

- [x] Add `remove` for the repository-owned `gitDiffToRemote` wire API: no TUI/CLI producer or consumer; only app-server dispatch/processor and protocol/schema definitions remain; `turn/diff/updated` and TUI `/diff` are not exact replacements.
- [x] Add `remove` for `mock/experimentalMethod` and its test-only payload/processor path; retain experimental gating coverage through real experimental methods.
- [x] Add `migrate-in-place` then `remove` for `getAuthStatus`, recording its app-server auth-test consumers and migration to `account/read` without exposing legacy tokens.
- [x] Add `remove` for `getConversationSummary` after recording that only its deprecated integration tests and implementation remain and no TUI/CLI consumer exists.
- [x] Add `remove` for legacy `applyPatchApproval` and `execCommandApproval` server requests after confirming their producers are absent; record the current canonical v2 approval requests.
- [x] Add `retain` for `item/fileChange/outputDelta` and `thread/compacted` pending raw-event/rollout replay audit.

### Task 2: Remove `gitDiffToRemote` and the test-only mock method

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/common.rs`
- Modify: `crates/app-server-protocol/src/protocol/v1.rs`
- Modify: `crates/app-server-protocol/src/lib.rs`
- Modify: `crates/app-server-protocol/src/export.rs`
- Modify: `crates/app-server/src/message_processor.rs`
- Modify: `crates/app-server/src/request_processors.rs`
- Modify: `crates/app-server/src/request_processors/git_processor.rs`
- Modify: `crates/app-server/src/request_processors/catalog_processor.rs`
- Modify: `crates/app-server/tests/common/test_app_server.rs`
- Modify: `crates/app-server/tests/suite/v2/experimental_api.rs`
- Modify: protocol export tests and any generated schema files affected by `just generate`

**Interfaces:**
- Removes the `ClientRequest` variants/wire methods and their protocol payload exports.
- Leaves `genex_git_utils::git_diff_to_remote` and its core Git behavior intact unless a final consumer scan proves otherwise.

- [x] Remove `GitDiffToRemote` from the app-server protocol and dispatch path, including the v1 payload exports and `V1_CLIENT_REQUEST_METHODS` entry.
- [x] Remove `MockExperimentalMethod`, its payload/response, catalog processor method, test-client helper, and dedicated mock gating test.
- [x] Keep or strengthen gating tests for `thread/settings/update` and `turn/settings/update` so experimental capability enforcement remains covered.
- [x] Run `just generate` and inspect the schema diff for only the intended removals.

### Task 3: Retire `getAuthStatus` and `getConversationSummary`

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/common.rs`
- Modify: `crates/app-server-protocol/src/protocol/v1.rs`
- Modify: `crates/app-server-protocol/src/lib.rs`
- Modify: `crates/app-server/src/message_processor.rs`
- Modify: `crates/app-server/src/request_processors.rs`
- Modify: `crates/app-server/src/request_processors/account_processor.rs`
- Modify: `crates/app-server/src/request_processors/thread_processor.rs`
- Modify: `crates/app-server/tests/common/test_app_server.rs`
- Modify: `crates/app-server/tests/suite/auth.rs`
- Modify: `crates/app-server/tests/suite/v2/account.rs`
- Modify: `crates/app-server/tests/suite/conversation_summary.rs`
- Modify: generated protocol schemas affected by `just generate`

**Interfaces:**
- `account/read` remains the canonical account read API.
- No replacement wire method is introduced for `getConversationSummary`; remove the deprecated API and its deprecated-only integration coverage while preserving active thread read/list behavior.

- [x] Migrate or remove all `getAuthStatus` test helpers/assertions, using `account/read` for current account/auth state and not carrying over `include_token` behavior.
- [x] Remove `getConversationSummary` request definitions, processor methods, imports, helpers, and deprecated-only integration tests; preserve any useful summary algorithm coverage in an internal test if needed.
- [x] Regenerate schemas and verify no deprecated client method remains.

### Task 4: Remove legacy server approval request variants

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/common.rs`
- Modify: `crates/app-server-protocol/src/protocol/v1.rs`
- Modify: `crates/app-server-protocol/src/lib.rs`
- Modify: `crates/tui/src/chatwidget/protocol_requests.rs`
- Modify: `crates/tui/src/app/app_server_events.rs`
- Modify: `crates/tui/src/app/app_server_event_targets.rs`
- Modify: `crates/tui/src/app/app_server_requests.rs`
- Modify: `crates/tui/src/app/side.rs`
- Modify: `crates/exec/src/lib.rs`
- Modify: protocol serialization tests and generated schemas affected by `just generate`

**Interfaces:**
- Canonical approval requests remain `item/fileChange/requestApproval` and `item/commandExecution/requestApproval`.

- [x] Reconfirm with a repository-wide producer search that `ServerRequestPayload::ApplyPatchApproval` and `ServerRequestPayload::ExecCommandApproval` have no production producer.
- [x] Remove the obsolete variants, v1 payload types, serialization-only test, and unreachable TUI/exec match arms.
- [x] Preserve all current canonical approval handling and test coverage.

### Task 5: Verify the retained legacy notifications and the complete cleanup

**Files:**
- Inspect only unless verification exposes an issue: `crates/app-server/src/bespoke_event_handling.rs`, `crates/tui/src/chatwidget/protocol.rs`, `crates/app-server/src/notification_media.rs`, and raw-event/rollout readers.

- [x] Confirm `item/fileChange/outputDelta` and `thread/compacted` still have an explicit `retain` decision and active compatibility rationale; do not remove them in this change.
- [x] Run exact residual searches for removed method names, variants, payloads, and generated schema entries.
- [x] Run `just generate`, inspect `git diff`, then run `just check --tests` and `just check`.
- [x] Review the final diff for accidental removal of current sandbox, approval, persisted-data, or provider-neutral behavior.
