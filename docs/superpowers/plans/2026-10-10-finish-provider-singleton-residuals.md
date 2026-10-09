# Finish Provider Singleton Residuals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the remaining provider-singleton boundary gaps, validate every local sensitive-state entry point, remove obsolete authentication plumbing, and leave `main` verified and pushed.

**Architecture:** The host validates `genex-home.json` before constructing provider credentials or opening local state. Core sends a neutral streaming-session request to the injected `ModelProvider`; provider-specific wire selection, WebSocket connection reuse, payload construction, and transport fallback stay inside the adapter. Generic security invalidation remains, while unused connection-auth and processor credential fields are removed only after producer/consumer confirmation.

**Tech Stack:** Rust workspace, `genex-model-provider`, `genex-core`, AppServer/CLI, SQLite state, checked-in generated schemas, `just` recipes, user systemd memory scope.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`

## Global Constraints

- One `ModelProvider` instance is injected per runtime/AppServer; provider switching requires another runtime/data root.
- `genex-home.json` must be validated before SQLite, sessions, caches, or credentials are opened.
- Core consumes provider-neutral types; provider HTTP, wire payloads, credential formats, and provider-specific error details remain in the adapter.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers.
- Update `docs/pre-release-compatibility-inventory.md` with an explicit producer/consumer decision before changing each production contract.
- Use `just check`, `just check --tests`, and `just generate` through the repository-mandated user systemd scope; do not run standalone tests.
- Work directly on `main`; do not create a worktree.

## Review Focus

- Existing populated homes without a marker and homes with a mismatched marker fail before credentials or SQLite are touched; cover through the shared mount helper tests.
- MCP and Marketplace commands that create `CredentialManager` cannot bypass provider-home validation; cover the shared CLI host helper.
- A provider that does not support persistent streaming returns an explicit unsupported/fallback result without Core inspecting wire names; cover the fake provider contract.
- In-flight stream state preserves continuation and credential snapshots while logout/auth-owner changes affect only new admissions; retain existing lifecycle tests and compile coverage.
- Diagnostic log readers cannot silently treat an arbitrary home as the mounted provider root; cover explicit `--genex-home` validation while retaining explicit external `--db` behavior.

### Task 1: Record residual producer/consumer decisions and add shared CLI mount boundary

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `crates/cli/src/cloud_config.rs`
- Modify: `crates/cli/src/mcp_cmd.rs`
- Modify: `crates/cli/src/marketplace_cmd.rs`
- Test: existing `crates/model-provider/src/home_marker.rs` tests and CLI compile targets

**Interfaces:**
- Produce `cloud_config::mount_provider_home(&Config) -> anyhow::Result<()>` for CLI commands that may open provider credentials.
- MCP `list/login/logout` and Marketplace `list/upgrade` must call the helper before `load_cli_credential_manager`.

- [x] **Step 1: Record decisions before code changes.** Add explicit `migrate-in-place` entries for MCP/Marketplace credential startup and diagnostic log home validation, and `remove` entries for the unused auth bridge fields identified by the audit.
- [x] **Step 2: Add the failing compile/test assertion.** Route the commands through a single host helper; use existing marker tests plus a focused source-level invariant test if the CLI test surface supports it. The expected pre-fix failure is that the command path has no mount call before credential construction.
- [x] **Step 3: Implement the shared helper and call sites.** Make the helper derive the descriptor from `AppServerRuntimeBuilder`, validate the marker, and then permit credential construction. Keep config-only MCP add/remove and Marketplace add/publish/remove paths unchanged unless they open credentials or SQLite.
- [x] **Step 4: Run the mandated compile verification.** Run `just check --tests` in the memory-limited scope and confirm the CLI targets compile.

### Task 2: Move streaming transport ownership behind `ModelProvider`

**Files:**
- Modify: `crates/model-provider/src/contract.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/core/src/client.rs`
- Modify: related model-provider/core tests and imports
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Add neutral `ProviderStreamRequest`, `ProviderStreamContinuation`, `ProviderStreamOutcome`, and provider-owned `ProviderStreamSession` interfaces.
- `ModelProvider::stream_session(...)` returns a provider-owned session; Core passes normalized chat input, continuation items/id, warmup state, turn metadata, opaque credential snapshot, and neutral telemetry callbacks.
- Core receives only `ProviderChatStream`, `ProviderError`, and explicit `FallbackToChat`/`Unsupported` outcomes; it no longer imports `ProviderWireApi`, `ProviderWebsocket*`, or constructs WebSocket payloads.

- [x] **Step 1: Add contract tests first.** Extend the fake provider to prove unsupported streaming is explicit and that the request carries no provider-specific wire payload type.
- [x] **Step 2: Run `just check --tests` and confirm the new contract tests fail to compile until the trait implementation exists.**
- [x] **Step 3: Implement the adapter-owned session.** Move connection reuse, timeout, WebSocket handshake, payload projection, warmup, continuation transport, and transport telemetry from Core into the configured provider runtime. Preserve existing OpenAI Responses JSON and headers exactly.
- [x] **Step 4: Migrate Core.** Replace `ProviderWireApi` branching and direct WebSocket calls with the neutral session operation. Keep Core-only continuation bookkeeping, turn state, retry/auth-recovery policy, response-event mapping, and rollout telemetry.
- [x] **Step 5: Remove the Core-facing low-level transport exports.** Keep concrete WebSocket/wire structs private to the provider adapter or its neutral session implementation.
- [x] **Step 6: Run `just check --tests` and inspect exact remaining provider transport references.** No production Core reference may remain to `ProviderWireApi`, `ProviderWebsocketConnectRequest`, `ProviderWebsocketPayloadRequest`, `ProviderWebsocketRequest`, or `SharedProviderWebsocketConnection`.

### Task 3: Remove obsolete authentication bridge and unused processor wiring

**Files:**
- Modify/Delete: `crates/app-server-transport/src/connection_auth.rs`, `crates/app-server-transport/src/lib.rs`
- Modify: `crates/app-server/src/connection_rpc_gate.rs`, `crates/app-server/src/transport.rs`, `crates/app-server/src/lib.rs`
- Modify: `crates/app-server/src/request_processors/plugins.rs`
- Modify: `crates/app-server/src/request_processors/turn_processor.rs`
- Modify: constructors and tests affected by the producer/consumer audit
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Keep generic `AuthChangeState` and user-verification invalidation behavior.
- Remove `ConnectionAuth` only if no active transport producer or gate consumer remains.
- Remove processor `CredentialManager` fields/parameters only where the field has no runtime consumer; retain account, verification, extension, and provider lifecycle consumers.

- [x] **Step 1: Add/adjust compile assertions for the active auth-change consumers.** Confirm account lifecycle and user-verification paths still compile without the dead bridge.
- [x] **Step 2: Remove the unused bridge and fields with minimal constructor changes.** Do not remove active credential-manager ownership from `MessageProcessor`, account processing, extensions, or user verification.
- [x] **Step 3: Run `just check --tests` and review warning reduction.**

### Task 4: Enforce marker semantics in diagnostics and retire misleading documentation

**Files:**
- Modify: `crates/cli/src/bin/logs_client.rs`
- Modify: `docs/superpowers/plans/2026-10-08-provider-credentials-and-selection.md`
- Modify: `docs/superpowers/plans/2026-10-08-mcp-credential-isolation.md` if it references the retired provider-selection design without a historical note
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- `logs_client --genex-home` validates the mounted provider marker before opening the logs DB.
- Explicit `--db` remains a diagnostic external-database override and is documented as outside provider-home ownership.

- [x] **Step 1: Add the failing diagnostic-path test or compile assertion for an unmarked/mismatched `--genex-home`.**
- [x] **Step 2: Validate the marker before `StateRuntime::init` when the database is resolved from a provider home.**
- [x] **Step 3: Mark the old provider-selection plan as historical/retired with all unchecked implementation items explicitly obsolete or linked to the completed singleton plan.**
- [x] **Step 4: Run `just generate` if documentation or source changes affect generated artifacts, and inspect the diff.**

### Task 5: Full verification, commit, and push

**Files:**
- Verify all changed files and generated artifacts.

- [x] **Step 1: Run `just check` in the memory-limited user systemd scope.**
- [x] **Step 2: Run `just check --tests` in the memory-limited user systemd scope.**
- [x] **Step 3: Run `just generate` in the memory-limited user systemd scope and inspect the generated diff.**
- [x] **Step 4: Run `git diff --check`, exact residual searches, and `git status --short --branch`.**
- [x] **Step 5: Commit all implementation, inventory, plan, and generated changes on `main`.**
- [x] **Step 6: Push `main` to `origin` and verify the remote commit matches local HEAD.**
