# Close Single-Provider Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate the remaining provider transport leaks and duplicated startup/auth boundaries so the single-provider Genex runtime is complete and verifiable.

**Architecture:** The public `ModelProvider` contract carries only provider-neutral request context. The concrete HTTP client factory, authentication adapter, and OpenAI-compatible wire request construction live in the injected provider runtime. AppServer startup uses one mount-and-runtime helper, while generic credential storage remains available only to host-owned security/MCP consumers.

**Tech Stack:** Rust workspace, `genex-model-provider`, `genex-core`, AppServer runtime, provider contract tests, repository `just` recipes.

**Spec:** `docs/superpowers/specs/2026-10-09-complete-single-provider-boundary-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers.
- Record `remove`, `retain`, or `migrate-in-place` decisions before changing existing contracts.
- Run Rust checks through the memory-limited user systemd scope and use `just check` plus `just check --tests` after Rust changes.
- Run `just generate` and inspect the diff if checked-in generated artifacts are affected.

## Review Focus

- A configured outbound proxy must still reach the provider after the factory moves into the runtime; test host construction with an injected factory.
- Custom/fake providers must still receive unsupported optional operations explicitly rather than being forced to implement HTTP.
- Provider logout/login state changes must remain shared across AppServer clients while in-flight turn snapshots remain stable.
- Mismatched provider homes must fail before credentials or SQLite are opened on every startup path.
- Existing Responses and Chat request JSON must remain field-compatible at the adapter boundary.

### Task 1: Record the boundary decisions and make the public contract transport-neutral

**Files:** `docs/pre-release-compatibility-inventory.md`, `docs/provider-decoupling-candidates.md`, `crates/model-provider/src/contract.rs`, `crates/model-provider/src/lib.rs`, and request producers in Core, extensions, CLI, and model catalog source.

- [x] Record producer/consumer evidence and a `migrate-in-place` decision before contract edits.
- [x] Remove `HttpClientFactory` and `ProviderTransportContext` from public request shapes.
- [x] Add/adjust contract tests proving requests can be built without an HTTP implementation type and unsupported capabilities remain explicit.
- [x] Compile the affected targets with the required test check.

### Task 2: Give the injected provider ownership of outbound transport and unify startup construction

**Files:** `crates/model-provider/src/provider.rs`, `crates/model-provider/src/cloudflare_provider.rs`, `crates/app-server/src/runtime.rs`, `crates/app-server/src/lib.rs`, and `crates/cli/src/doctor.rs`.

- [x] Add the host-only factory injection path and keep a default constructor for test fixtures.
- [x] Route daemon startup through `configured_app_server_runtime_from_config`.
- [x] Add regression coverage that provider-home validation precedes credential loading.
- [x] Run required checks.

### Task 3: Remove the remaining Core wire-construction dependency

**Files:** `crates/core/src/client.rs`, provider contract/adapter files, and relevant Core contract tests.

- [x] Replace Core-owned Responses payload and WebSocket wire construction with a provider-neutral request/builder result owned by the adapter.
- [x] Preserve Core continuation, turn-state, telemetry, fallback, and auth-recovery policy.
- [x] Add a regression/compile test that Core no longer imports provider wire request types.
- [x] Run required checks and inspect any generated diff.

### Task 4: Finish the credential boundary and repository-wide residual audit

**Files:** Core session/thread/state/prompt-debug paths, AppServer account processor, compatibility inventory, candidates, and model-catalog plan.

- [x] Keep `CredentialManager` only for host-owned generic external-auth/security/MCP concerns; provider auth state/lifecycle stays on `SharedModelProvider`.
- [x] Remove provider-specific credential-manager requirements from Core production construction paths.
- [x] Re-run provider selection/persistence/HTTP/wire searches and record all retained external contracts.
- [x] Run `just check --tests`, `just check`, `git diff --check`, commit, and push `main`.
