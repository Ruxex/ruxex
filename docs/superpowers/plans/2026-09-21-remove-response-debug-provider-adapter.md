# Remove provider-specific Response Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove provider-specific response-error headers and identity diagnostics while retaining provider-neutral request-ID, edge-diagnostic, error-redaction, and telemetry behavior.

**Architecture:** Keep `codex-response-debug-context` as a small provider-neutral diagnostic helper used by `codex-core` and `codex-model-provider`. Remove its provider-specific header parsing and the corresponding identity-error fields from the API/protocol error path; preserve ordinary HTTP status, request ID, `cf-ray`, authorization recovery, and bounded telemetry messages.

**Tech Stack:** Rust workspace, `codex-api`, `codex-protocol`, `codex-core`, `codex-model-provider`, `codex-otel`, `just check`, dependency-graph generation.

**Spec:** `docs/provider-decoupling-plan.md` and `docs/pre-release-compatibility-inventory.md`

## Global Constraints

- Work directly on the repository's `main` branch; do not create a worktree.
- Treat the change as a capability boundary, not a crate-name deletion.
- Preserve other providers behavior, Bedrock/workload identity, ordinary authorization recovery, security redaction, and current `cf-ray`/request-ID diagnostics.
- Remove only `x-oai-request-id` fallback, `x-provider-authorization-error`, `x-error-json`, and the identity-error fields that exist solely for those headers.
- Do not run tests for this cleanup; use focused reference audits, `cargo check`, `just crate-dependency-graph`, and `just check` as verification.

## Review Focus

- HTTP failures with only `x-request-id` and/or `cf-ray` still retain the diagnostic values.
- HTTP failures from other providerss no longer require provider-named headers or fields.
- Authorization recovery still uses the provider's existing recoverability policy and is not weakened by removing diagnostics.
- Error messages and telemetry continue to omit upstream response bodies and sensitive header values.
- Removing protocol error fields does not remove unrelated Bedrock, MCP, sandbox, or generic credential behavior.

### Task 1: Fix the inventory and freeze the boundary

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-plan.md`
- Modify: `crates/response-debug-context/src/lib.rs`
- Inspect: `crates/codex-api/src/api_bridge.rs`, `crates/protocol/src/error.rs`, `crates/core/src/client.rs`, `crates/model-provider/src/models_endpoint.rs`, and `crates/otel/src/events/session_telemetry.rs`

**Interfaces:**
- Consumes: current `TransportError::Http` headers and `ApiError` mapping.
- Produces: an explicit producer/consumer audit for the provider-specific response-diagnostics slice before implementation.

- [x] Run the production searches below and inspect every match before editing:

```bash
rg -n 'x-provider-authorization-error|x-error-json|x-oai-request-id|identity_authorization_error|identity_error_code|auth\.error_code|auth\.error' crates --glob '*.rs'
rg -n 'extract_response_debug_context|ResponseDebugContext|telemetry_api_error_message|telemetry_transport_error_message' crates --glob '*.rs'
```

- [x] Record the current producers and consumers in the inventory: `codex-api` produces the header-derived error fields; `codex-response-debug-context`, `codex-core`, `codex-model-provider`, `codex-otel`, `codex-protocol`, and fixtures consume them. Record the canonical replacement as request-ID/`cf-ray` diagnostics plus provider-owned recovery policy.
- [x] Keep `codex-response-debug-context` classified as `migrate-in-place`; classify the provider header constants and identity-error fields as `remove`.

### Task 2: Make response diagnostics provider-neutral

**Files:**
- Modify: `crates/response-debug-context/src/lib.rs`
- Modify: `crates/response-debug-context/Cargo.toml`
- Modify: `crates/core/src/client.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`

**Interfaces:**
- Consumes: `TransportError::Http` with standard request-ID and `cf-ray` headers.
- Produces: `ResponseDebugContext { request_id, cf_ray }` and the existing bounded telemetry error-message helpers.

- [x] Remove `OAI_REQUEST_ID_HEADER`, `AUTH_ERROR_HEADER`, `X_ERROR_JSON_HEADER`, `auth_error`, and `auth_error_code` from `ResponseDebugContext` and its extractor.
- [x] Retain `x-request-id`, `cf-ray`, the `ApiError`/`TransportError` error-message redaction, and the existing public extractor names unless a compile-driven rename is required.
- [x] Remove `base64` and `serde_json` from the response-debug-context manifest if no remaining source uses them.
- [x] Remove the corresponding `auth.error` and `auth.error_code` arguments at the model-provider/core telemetry call sites while leaving request-ID and `cf-ray` attributes intact.

### Task 3: Remove the obsolete protocol/API identity diagnostics

**Files:**
- Modify: `crates/codex-api/src/api_bridge.rs`
- Modify: `crates/codex-api/src/api_bridge_tests.rs`
- Modify: `crates/protocol/src/error.rs`
- Modify: `crates/protocol/src/error_tests.rs`
- Modify: `crates/tui/src/history_cell/tests.rs`
- Modify: `crates/otel/src/events/session_telemetry.rs`
- Modify: affected generated artifacts only if the repository generator reports a change

**Interfaces:**
- Consumes: provider-neutral `TransportError::Http` status, URL, request ID, and `cf-ray` values.
- Produces: protocol/API errors without the provider identity-error fields; ordinary error display and redaction remain unchanged.

- [x] Remove `identity_authorization_error`, `identity_error_code`, `X_provider_AUTHORIZATION_ERROR_HEADER`, `X_ERROR_JSON_HEADER`, and the base64 JSON-code extraction from the API bridge.
- [x] Remove the corresponding protocol display branches and fixture fields while retaining status, URL, request ID, and `cf-ray` rendering.
- [x] Remove telemetry function parameters and attributes that have no producer after the header path is removed; preserve unrelated auth attributes and current bounded error output.
- [x] Run the supported generation command only if protocol/config schemas are affected, then inspect the generated diff for unrelated changes.

### Task 4: Regenerate dependency metadata and verify the boundary

**Files:**
- Modify: generated dependency graph outputs when produced by the repository command
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-plan.md`

**Interfaces:**
- Consumes: the reduced response-diagnostics API and all updated consumers.
- Produces: a completed inventory row, updated Phase 3 selection history, and a verified workspace.

- [x] Run `rg -n 'x-provider-authorization-error|x-error-json|x-oai-request-id|identity_authorization_error|identity_error_code' crates --glob '*.rs'` and classify any remaining match as test/documentation-only or an active non-provider consumer.
- [x] Run `just crate-dependency-graph` because the response-debug-context dependency edges change.
- [x] Check the repository-local `target/` size before the workspace check; if it exceeds the documented threshold, remove only that repository-local directory.
- [x] Run `just check` and record the result; do not run tests.
- [x] Mark this plan and the inventory slice complete only after the reference audit, dependency graph, and `just check` all pass.
