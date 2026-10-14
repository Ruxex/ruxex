# Close Provider Injection Residuals Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining provider-injection boundary defects so an injected provider is authoritative from startup through TUI presentation and Core telemetry.

**Architecture:** The in-process client selects the mounted-home descriptor from the injected `AppServerRuntime` before starting the server. TUI bootstrap state is carried into the session and status card as server-owned provider metadata. Core emits a telemetry-only neutral event shape owned by the telemetry crate instead of converting through the provider wire crate; configured built-in provider construction remains an explicit host adapter.

**Tech Stack:** Rust workspace, Tokio, AppServer protocol, TUI/rataui, `genex-model-provider`, `genex-otel`, repository `just` recipes.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers.
- A runtime/AppServer owns exactly one injected `ModelProvider`; another provider requires another runtime/data root.
- Do not persist provider routing or credentials in threads, rollouts, or history.
- Keep configured `genex-providers` construction only at the host adapter boundary already recorded as `retain`.
- Run Rust checks only through memory-scoped `systemd-run --user --scope --collect --quiet` invocations of `just`, with `CARGO_BUILD_JOBS=2` for workspace checks.
- Do not run standalone Rust tests; use `just check`, `just check --tests`, and `just generate` when applicable.

## Review Focus

- A configured provider descriptor different from an injected provider descriptor must not cause a false home-marker mismatch in the in-process client.
- Reconnect and startup must replace provider status/capabilities together with the server context; stale provider metadata must not remain in the TUI.
- Status output must show the connected AppServer's provider name rather than a compile-time product/provider string.
- Core production code must not require `genex-api` merely to adapt stream telemetry; provider wire types remain in the provider adapter.
- Unsupported provider capabilities remain explicit and the host convenience adapter does not reintroduce runtime provider selection.

---

### Task 1: Make injected runtime authoritative in the in-process client

**Files:**
- Modify: `crates/app-server-client/src/lib.rs`
- Test: `crates/app-server-client/src/lib.rs` unit tests
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Add a private startup mount helper that accepts `&Config` and `Option<&AppServerRuntime>` and uses `mount_provider_home_for_runtime` when a runtime is present, otherwise `mount_provider_home_for_config`.

- [x] Add a unit test with different configured and injected descriptors asserting the helper writes the injected provider marker.
- [x] Run the affected compile-only test target and confirm the test/helper contract is initially unavailable or fails for the old selection path.
- [x] Implement the helper and use it before `in_process::start`.
- [x] Re-run the compile-only target and update the inventory with the producer/consumer decision.

### Task 2: Carry provider status and capabilities into TUI presentation

**Files:**
- Modify: `crates/tui/src/app_server_session.rs`
- Modify: `crates/tui/src/app/startup.rs`
- Modify: `crates/tui/src/app/reconnect.rs`
- Modify: `crates/tui/src/chatwidget.rs`
- Modify: `crates/tui/src/chatwidget/status_controls.rs`
- Modify: `crates/tui/src/status/card.rs`
- Modify: affected TUI test helpers and status tests

**Interfaces:**
- `ChatWidgetInit` and `ChatWidget` carry the server-provided provider display name and capability snapshot.
- `new_status_output_with_rate_limits_handle` accepts the provider display name and renders it in the header.

- [x] Add compile-level/status rendering coverage for a non-default provider name and a capability-disabled provider.
- [x] Run the scoped TUI compile-only test target and inspect the expected missing-argument failure before implementation.
- [x] Thread bootstrap values through initial startup and reconnect, replacing the old hardcoded provider label.
- [x] Use capability data for an existing optional UI affordance, explicitly hiding it when unsupported, without adding provider selection.
- [x] Run the scoped TUI compile-only target and update the provider-derived TUI inventory row with actual consumers.

### Task 3: Remove Core's provider-wire telemetry dependency

**Files:**
- Create: `crates/otel/src/events/provider_event.rs`
- Modify: `crates/otel/src/events/session_telemetry.rs`
- Modify: `crates/otel/src/lib.rs`
- Modify: `crates/otel/Cargo.toml`
- Modify: `crates/core/src/client.rs`
- Modify: `crates/core/src/session/turn.rs`
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/model-provider/src/contract.rs`
- Modify: `crates/providers/src/provider.rs`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Add a telemetry-owned `ProviderResponseEvent`/`ProviderSafetyBuffering` shape containing only the fields used by `SessionTelemetry::record_responses`.
- `SessionTelemetry::record_responses` consumes the telemetry-owned shape; Core maps `ProviderChatEvent` to it without `genex-api`.
- Provider adapters perform any conversion from `genex_api::ResponseEvent` to `ProviderChatEvent` locally.

- [x] Add a compile-level contract test or type assertion proving telemetry accepts the neutral event and Core does not import `genex_api`.
- [x] Run the scoped Core/OTel test-target check and observe the old conversion/dependency boundary failure before implementation.
- [x] Implement the neutral telemetry event and move the adapter conversion into `genex-providers`.
- [x] Remove the Core `genex-api` dependency and retain external wire behavior in the provider adapter.
- [x] Run the affected and workspace compile-only checks; record the boundary decision.

### Task 4: Final audit and required verification

**Files:**
- Modify: `docs/provider-decoupling-candidates.md` if the candidate status needs correction.
- Modify: `docs/pre-release-compatibility-inventory.md` with final evidence.

- [x] Audit provider selection, `model_provider_id`, `wire_api`, `base_url`, `genex_api`, and `genex_providers` matches in production paths.
- [x] Run `just generate` and inspect all generated changes if protocol/schema output is affected.
- [x] Run `just check` and `just check --tests` through the required memory-scoped systemd commands.
- [x] Run `git diff --check`, inspect the complete diff, and confirm no credentials/provider routing entered persistence.
- [x] Perform a separate final self-review and record any deferred minor findings.
