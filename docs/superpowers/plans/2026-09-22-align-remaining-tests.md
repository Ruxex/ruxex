# Align Remaining Tests With Current Provider-Neutral APIs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove or update test-only code that still exercises deleted hosted, Apps/connector, ChatGPT-auth, Agent Identity, and remote exec-server contracts so the test targets match the current production APIs.

**Architecture:** Preserve tests for current local/stdio exec-server behavior, provider-neutral plugin loading, current MCP elicitation, and security behavior. Delete only tests and fixtures whose sole purpose is a removed contract; update shared test helpers only when they still serve retained tests.

**Tech Stack:** Rust workspace, Cargo test-target compilation, `just check`, `just check --tests`, `rg`, and `git diff --check`.

**Spec:** `docs/pre-release-compatibility-inventory.md` and `docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md`.

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Do not change production behavior to satisfy obsolete tests.
- Preserve current provider-neutral plugin, MCP, filesystem, security, and stdio exec-server behavior.
- Remove test-only remote/hosted compatibility code instead of restoring deleted dependencies or APIs.
- Use `just check` for production type-checking and `just check --tests` after every Rust test-target cleanup group.
- Do not run standalone test commands for this cleanup unless explicitly requested.

## Review Focus

- Current stdio exec-server and local-environment tests remain after remote transport cleanup.
- Current plugin manifest/MCP/hooks/skills tests remain after removing Apps and hosted marketplace integrations.
- Current MCP elicitation tests retain native/provider-neutral flows without provider-specific forms.
- Removed ChatGPT and Agent Identity auth modes are absent from test fixtures and assertions.
- Test helper signatures match the current production constructors and loader APIs.

### Task 1: Remove remote exec-server test contracts

**Files:**
- Modify: `crates/exec-server/src/client.rs`, `crates/exec-server/src/connection.rs`, `crates/exec-server/src/environment.rs`, `crates/exec-server/src/environment_toml.rs`, `crates/exec-server/src/remote_file_system_path_uri_tests.rs`, `crates/exec-server/src/server/process_otel_tests.rs`.
- Modify or remove stale sections in: `crates/exec-server/tests/common/exec_server.rs`, `crates/exec-server/tests/environment.rs`, `crates/exec-server/tests/http_client.rs`, `crates/exec-server/tests/selected_capability_roots.rs`, `crates/exec-server/tests/capability_discovery.rs`.
- Modify: `crates/rmcp-client/tests/streamable_http_test_support.rs` only where it depends on the removed remote exec-server WebSocket harness.

- [ ] Delete tests and helpers that require `tokio_tungstenite`, `RemoteExecServerConnectArgs`, `connect_websocket`, `REMOTE_ENVIRONMENT_ID`, remote timeout constants, remote environment upsert, or remote-only telemetry registration.
- [ ] Retain HTTP response-stream, local environment, stdio transport, capability discovery, filesystem, and network-policy tests that compile against current APIs.
- [ ] Remove now-unused imports and test-support dependencies without adding the deleted remote transport dependency back.
- [ ] Run `just check --tests`; expected result is no errors from `exec-server` or `rmcp-client`.

### Task 2: Align core-plugins tests with the current plugin surface

**Files:**
- Modify: `crates/core-plugins/src/manager_tests.rs`, `crates/core-plugins/src/loader_tests.rs`, `crates/core-plugins/src/test_support.rs`.
- Modify: `crates/core-plugins/src/executor_hooks_tests.rs`, `crates/core-plugins/src/marketplace_policy_tests.rs`, `crates/core-plugins/src/script_attribution_tests.rs`, `crates/core-plugins/src/store_tests.rs`.
- Review current production interfaces in: `crates/core-plugins/src/lib.rs`, `manager.rs`, `loader.rs`, `store.rs`, and `crates/core-plugins/src/manifest.rs`.

- [ ] Delete tests and fixtures importing `crate::remote` or `remote_plugin_id_resolver`, including remote installed-plugin cache/reconciliation/recommended-plugin/telemetry tests whose production APIs were removed.
- [ ] Delete ChatGPT auth, Agent Identity, Apps, connector, and `effective_apps` assertions; remove corresponding `AppDeclaration`, `AppConnectorId`, `apps`, `app_connector_ids`, and `AuthMode` fixtures.
- [ ] Update retained helper calls to the current `PluginsConfigInput::new` and `load_plugins_from_layer_stack` signatures.
- [ ] Retain current marketplace source-policy, local/bundled plugin loading, MCP server, hooks, skills, script attribution, and store lifecycle tests.
- [ ] Run `just check --tests`; expected result is no errors from `core-plugins`.

### Task 3: Remove remaining provider-specific MCP test cases

**Files:**
- Review: `crates/rmcp-client/tests/mcp_2026_mrtr/native_verification_tests.rs` and `crates/rmcp-client/src/` elicitation types.
- Review any remaining `crates/rmcp-client` tests importing `provider_ELICITATION_EXTENSION_ID` or matching `providerElicitationForm`.

- [ ] Remove assertions and fixtures for deleted provider elicitation extension IDs/forms while retaining native MRTR validation, malformed-request rejection, continuation expiry, and cancellation behavior.
- [ ] Confirm no remaining test imports removed protocol constants or enum variants.
- [ ] Run `just check --tests` and inspect the complete error summary.

### Task 4: Final cleanup and verification

**Files:**
- Modify only test files and test-only dependencies identified by Tasks 1–3.
- Review: `Cargo.toml` files and `Cargo.lock` only if stale test-only dependencies are removed.

- [ ] Run `rg` for removed symbols: `RemoteExecServerConnectArgs`, `connect_websocket`, `REMOTE_ENVIRONMENT_ID`, `crate::remote`, `AppConnectorId`, `providerElicitationForm`, `AuthMode::Chatgpt`, and `create_dummy_chatgpt_auth_for_testing`.
- [ ] Run `just check` and `just check --tests`.
- [ ] Run `git diff --check` and inspect the diff for accidental production changes or deletion of retained provider-neutral tests.
- [ ] Report any remaining failures by root cause rather than masking them with compatibility shims.
