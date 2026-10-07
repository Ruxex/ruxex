# MCP Credential Isolation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make MCP credential resolution and lifecycle independent from model-provider credentials and the Genex login manager.

**Architecture:** MCP will consume only server configuration, MCP environment credentials, and the existing MCP OAuth stores. The MCP runtime will no longer carry `GenexAuth`, `AuthManager`, or model-provider auth-change state. The existing MCP-specific credential refresh and connection identity logic remains the source of reconnect behavior.

**Tech Stack:** Rust, Tokio, `genex-mcp`, `genex-rmcp-client`, `genex-core`, `genex-cli`, app-server MCP processors, checked-in protocol/config generation.

**Spec:** `docs/superpowers/specs/2026-10-08-provider-credentials-and-selection-design.md` (MCP sections)

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Do not remove active MCP OAuth, environment-header, bearer-token, transport, or security behavior.
- Before each production change, audit producers and consumers and record an explicit `remove`, `retain`, or `migrate-in-place` decision in `docs/pre-release-compatibility-inventory.md`.
- Do not run standalone Rust test commands; use `just check --tests` to compile test targets and `just check` for the production workspace check.
- Run `just generate` when config or protocol generated artifacts are affected and inspect the resulting diff.
- Run Rust build/check commands through the repository-required user systemd scope with `CARGO_BUILD_JOBS=2` for workspace checks.

## Review Focus

- MCP OAuth credentials remain usable when no model-provider credential exists; cover with the existing OAuth status/login fixtures in `crates/rmcp-client` and `crates/cli`.
- Static headers, environment headers, and bearer-token environment variables remain the only non-OAuth MCP request credentials; cover with `genex-mcp/src/server_tests.rs` and `rmcp-client` HTTP-client tests.
- A model-provider credential change no longer rebuilds or invalidates MCP connections; add a runtime identity regression test in `genex-mcp/src/server_tests.rs` or `runtime.rs`.
- The removed `genex/auth-change` host-auth notification path has no remaining producer or consumer; update `genex-mcp/src/auth_changes_tests.rs` and compile all app-server MCP tests.
- `McpServerAuth::EmaAuth` is handled by an explicit inventory decision and never silently falls back to model-provider credentials; cover config validation and startup behavior.

---

### Task 1: Record MCP auth-boundary decisions and freeze the public cleanup surface

**Files:**
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `crates/genex-mcp/src/mcp/mod.rs`
- Modify: `crates/genex-mcp/src/mcp/auth.rs`
- Modify: `crates/app-server/src/request_processors/mcp_processor.rs`
- Modify: `crates/cli/src/mcp_cmd.rs`

**Interfaces:**
- Remove the unused `auth: Option<&GenexAuth>` parameter from `effective_mcp_servers`, `effective_mcp_servers_from_configured`, and the MCP resource/status helpers.
- Keep `McpServerConfig`-owned auth inputs and `McpOAuthCredentialsStoreMode` unchanged.

- [ ] **Step 1: Audit every caller of the MCP discovery/status/resource functions**

  Confirm that each caller uses only MCP configuration, runtime context, and MCP OAuth storage. Record `remove` for the unused model-auth parameters and `retain` for MCP-owned credential inputs in the two inventory documents.

- [ ] **Step 2: Remove the unused model-auth parameters and imports**

  Update `genex-mcp/src/mcp/mod.rs`, `genex-mcp/src/mcp/auth.rs`, the app-server MCP processor, and `cli/src/mcp_cmd.rs`. Preserve `McpServerAuth::EmaAuth` behavior until Task 4 resolves its explicit inventory decision.

- [ ] **Step 3: Add unit coverage for provider-auth independence**

  Add a test around the effective-server/status projection proving that its result depends on MCP configuration and stored MCP OAuth state, not a `GenexAuth` value. Keep the test local to `genex-mcp` and compile it through `just check --tests`.

- [ ] **Step 4: Commit the API cleanup**

  ```bash
  git add docs/provider-decoupling-candidates.md docs/pre-release-compatibility-inventory.md crates/genex-mcp/src/mcp crates/app-server/src/request_processors/mcp_processor.rs crates/cli/src/mcp_cmd.rs
  git commit -m "refactor: remove model auth from MCP discovery"
  ```

### Task 2: Remove model-auth state from MCP runtime inputs and connection identity

**Files:**
- Modify: `crates/core/src/session/mcp.rs`
- Modify: `crates/core/src/session/mcp_runtime.rs`
- Modify: `crates/genex-mcp/src/runtime.rs`
- Modify: `crates/genex-mcp/src/connection_manager.rs`
- Modify: `crates/genex-mcp/src/server.rs`
- Modify: `crates/genex-mcp/src/rmcp_client.rs`
- Modify: `crates/rmcp-client/src/rmcp_client.rs`
- Modify: `crates/rmcp-client/src/http_client_adapter.rs`

**Interfaces:**
- `McpDesiredState` and `McpRuntimeInput` no longer contain `Option<GenexAuth>`.
- `McpRuntime::current_auth_matches` is removed; MCP refresh decisions use MCP configuration, environments, and OAuth credential snapshots.
- `McpServerConnectionIdentity::new` and the `rmcp-client` constructors no longer accept `SharedAuthProvider` or a model `GenexAuth`.

- [ ] **Step 1: Remove `GenexAuth` from session MCP desired state**

  Update `Session::latest_mcp_desired_state`, initial runtime installation, refresh paths, and `build_mcp_runtime_input`. Keep session configuration and MCP projection behavior unchanged.

- [ ] **Step 2: Remove runtime-auth fields and comparisons**

  Delete `auth`, `auth_token`, and `runtime_auth` fields that exist only to compare or forward model credentials. Remove `current_auth_matches` and its callers. Retain connection identity fields for MCP configuration, OAuth store, environment, headers, protocol mode, and client capabilities.

- [ ] **Step 3: Remove the unused `SharedAuthProvider` plumbing**

  Remove the `runtime_auth_provider` parameters from `genex-mcp` and `rmcp-client` constructors and HTTP adapters. Preserve the MCP server's configured bearer-token and OAuth resolution paths; only remove the optional model-provider auth path.

- [ ] **Step 4: Add a connection-identity regression test**

  Assert that changing the former model-provider auth value cannot change the MCP connection identity, while changing an MCP OAuth credential snapshot still does. If the first assertion becomes unrepresentable after the API cleanup, retain a test that verifies the identity includes only MCP-owned inputs.

- [ ] **Step 5: Commit the runtime boundary change**

  ```bash
  git add crates/core/src/session/mcp.rs crates/core/src/session/mcp_runtime.rs crates/genex-mcp/src crates/rmcp-client/src
  git commit -m "refactor: isolate MCP runtime credentials"
  ```

### Task 3: Replace host-auth change notifications with MCP-owned lifecycle behavior

**Files:**
- Modify: `crates/genex-mcp/src/auth_changes.rs`
- Modify: `crates/genex-mcp/src/auth_changes_tests.rs`
- Modify: `crates/genex-mcp/src/rmcp_client.rs`
- Modify: `crates/genex-mcp/src/connection_manager.rs`
- Modify: `crates/core/src/session/mcp.rs`

**Interfaces:**
- Remove the `genex/auth-change` experimental capability and `AuthChangeState` watcher from MCP startup unless the producer/consumer audit finds a current MCP-owned use.
- Continue using `oauth_credentials_changed` and the existing MCP runtime refresh/reconnect paths for MCP credential changes.

- [ ] **Step 1: Trace the `genex/auth-change` capability**

  Verify its only source is `genex-login::AuthChangeState` and its only consumer is the MCP notification task. Record the capability as `remove` if no MCP-owned consumer remains; otherwise define a new MCP credential revision type in `genex-mcp` and keep the capability under that new source.

- [ ] **Step 2: Remove or relocate the watcher**

  Implement the decision from Step 1. Do not pass `AuthManager::auth_change_state_receiver()` into MCP. If a new MCP revision is required, make it originate from MCP OAuth/config changes rather than model-provider login changes.

- [ ] **Step 3: Update tests and capability declarations**

  Remove tests that assert host-auth notifications when the capability is removed. If retained under MCP ownership, rewrite them to publish an MCP credential revision and assert that no secret values are included in the notification.

- [ ] **Step 4: Commit the lifecycle cleanup**

  ```bash
  git add crates/genex-mcp/src crates/core/src/session/mcp.rs docs/provider-decoupling-candidates.md docs/pre-release-compatibility-inventory.md
  git commit -m "refactor: make MCP auth changes server-scoped"
  ```

### Task 4: Resolve the remaining account-bound MCP path

**Files:**
- Audit and modify: `crates/config/src/types.rs`
- Audit and modify: `crates/genex-mcp/src/rmcp_client.rs`
- Audit and modify: `crates/genex-mcp/src/mcp/auth.rs`
- Audit and modify: `crates/app-server/src/request_processors/mcp_processor.rs`
- Audit and modify: `crates/cli/src/mcp_cmd.rs`
- Generated outputs: config/protocol artifacts only if the chosen decision changes their schemas

**Interfaces:**
- `McpServerAuth::EmaAuth` must either be removed as an obsolete host-account mode or explicitly retained as a separately configured MCP authentication mode. It must not reuse the active model-provider credential implicitly.

- [ ] **Step 1: Audit producers and consumers of `McpServerAuth::EmaAuth`**

  Record the exact config, CLI, app-server, and runtime behavior in the inventory before editing. Treat the current error path in `make_rmcp_client` and the CLI guidance as separate user-facing consumers.

- [ ] **Step 2: Apply the approved `remove`, `retain`, or `migrate-in-place` decision**

  If removed, delete the enum variant, validation branches, CLI guidance, and dead protocol/config schema entries together. If retained, require an explicit MCP credential source and update the error/status presentation to name that source.

- [ ] **Step 3: Generate and inspect artifacts when applicable**

  Run `just generate`, inspect the diff for only the intentional MCP schema changes, and update snapshots or fixtures that encode the removed/retained mode.

- [ ] **Step 4: Commit the account-boundary decision**

  ```bash
  git add crates/config crates/genex-mcp crates/app-server/src/request_processors/mcp_processor.rs crates/cli/src/mcp_cmd.rs docs/provider-decoupling-candidates.md docs/pre-release-compatibility-inventory.md
  git commit -m "refactor: make MCP account auth explicit"
  ```

### Task 5: Run repository verification and final MCP audit

**Files:**
- Verify: all files changed by Tasks 1-4
- Verify: `docs/provider-decoupling-candidates.md`
- Verify: `docs/pre-release-compatibility-inventory.md`

- [ ] **Step 1: Run test-target compilation**

  ```bash
  systemd-run --user --scope --collect --quiet \
    -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
    --setenv=CARGO_BUILD_JOBS=2 just check --tests
  ```

  Expected: success with all affected Rust test targets compiled.

- [ ] **Step 2: Run the production workspace check**

  ```bash
  systemd-run --user --scope --collect --quiet \
    -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
    --setenv=CARGO_BUILD_JOBS=2 just check
  ```

  Expected: success with no MCP/model-auth type or dependency errors.

- [ ] **Step 3: Inspect generated and source diffs**

  Run `git diff --check`, inspect generated changes, and search for remaining `runtime_auth_provider`, `current_auth_matches`, `effective_mcp_servers(..., auth)`, and `AuthManager` references in `genex-mcp` and MCP session code. Any remaining match must have an explicit retention reason.

- [ ] **Step 4: Commit the verified result**

  ```bash
  git add docs crates
  git commit -m "refactor: complete MCP credential isolation"
  ```
