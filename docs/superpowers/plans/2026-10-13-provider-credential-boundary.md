# Provider Credential Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove provider-auth credential dependencies from Core, add a scoped neutral credential boundary for Plugin/MCP services, and make the injected `ModelProvider` authoritative from startup through turn execution.

**Architecture:** Keep provider authentication and opaque request snapshots inside the injected `ModelProvider` adapter. Introduce a small provider-neutral credential contract for Plugin marketplace access, while MCP retains its own server-scoped OAuth runtime. AppServer constructs these dependencies at the Host boundary and Core receives only the provider plus narrowly scoped service interfaces.

**Tech Stack:** Rust 2024 workspace, async trait-object contracts using boxed futures, `Arc`, existing `genex-model-provider`, `genex-login`, `genex-core`, `genex-core-plugins`, `genex-mcp`, AppServer protocol tests, and repository `just` recipes.

**Spec:** `docs/superpowers/specs/2026-10-13-provider-credential-boundary-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/environment names, and model identifiers.
- Preserve MCP OAuth storage and protocol behavior; only make its dependency on model-provider auth explicit and absent.
- Do not persist credentials or provider routing in threads, rollouts, history, SQLite, or session logs.
- Rust build/check commands must run through the user systemd scope with `CARGO_BUILD_JOBS=2` for workspace-scale checks.
- Run `just check` for production verification and `just check --tests` after Rust changes; run `just generate` only when checked-in generated artifacts are affected.
- Before deleting a repository-owned compatibility field/API, record the producer/consumer decision in `docs/pre-release-compatibility-inventory.md`.

## Review Focus

- A custom provider with no repository `CredentialManager` can build and initialize; test it by injecting a descriptor different from config and reading provider status.
- Provider logout rejects new turns but does not replace the credential snapshot of an in-flight turn; test the admission snapshot directly with a fake provider.
- Plugin credential resolution is owner-scoped; test that Plugin A cannot request or receive Plugin B's credential.
- MCP OAuth behavior remains independent of model-provider login state; compile and run existing MCP auth tests without passing a provider credential manager.
- Core and model-catalog APIs do not reintroduce concrete `CredentialManager`, `GenexAuth`, `ModelProviderInfo`, HTTP client, or auth-header types; add exact `rg` audits to the final task.

---

### Task 1: Add the provider-neutral scoped credential contract

**Files:**
- Create: `crates/credential-broker/Cargo.toml`
- Create: `crates/credential-broker/src/lib.rs`
- Modify: `Cargo.toml` workspace members and workspace dependencies
- Test: `crates/credential-broker/src/lib.rs` unit tests

**Interfaces:**
- Produces `CredentialRequest`, `CredentialOwner`, `CredentialPurpose`, `CredentialSnapshot`, `CredentialError`, `CredentialFuture`, `CredentialResult`, `CredentialBroker`, and `SharedCredentialBroker`.
- The crate must not depend on `genex-login`, `genex-model-provider`, `genex-api`, or HTTP crates.

- [ ] **Step 1: Write contract tests** for unsupported resolution, owner/purpose equality, and an in-memory broker that only resolves an exact owner/purpose pair.
- [ ] **Step 2: Run the crate check** with `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=4 cargo check -p genex-credential-broker` only if the repository recipe supports the new package; otherwise use the required workspace `just check` command.
- [ ] **Step 3: Implement the opaque contract**. Do not implement `Debug`, `Serialize`, or public value accessors for `CredentialSnapshot`.
- [ ] **Step 4: Run the contract tests through `just check --tests`** and verify the crate compiles.
- [ ] **Step 5: Commit** `feat: add scoped credential broker contract`.

### Task 2: Make the injected provider authoritative in AppServer startup

**Files:**
- Modify: `crates/app-server/src/runtime.rs`
- Modify: `crates/app-server/src/in_process.rs`
- Modify: `crates/app-server/src/message_processor.rs`
- Modify: `crates/app-server/src/request_processors/initialize_processor.rs`
- Modify: `crates/app-server/src/lib.rs`
- Test: `crates/app-server/src/runtime.rs` and `crates/app-server/src/in_process.rs` tests

**Interfaces:**
- `AppServerRuntimeBuilder::new(provider).build()` must not load a `CredentialManager` or config provider as a fallback.
- `InitializeRequestProcessor::new` consumes the runtime provider descriptor or the runtime itself.
- All provider-home mounting and initialize identity responses use `AppServerRuntime::provider_descriptor()`.

- [ ] **Step 1: Add a failing in-process test** that injects a fake provider whose descriptor differs from `config.model_provider` and asserts startup plus `modelProvider/read` returns the injected descriptor.
- [ ] **Step 2: Run the focused AppServer compile-only test target** with the required memory scope and confirm the new test fails for the config-based initialize mount.
- [ ] **Step 3: Thread `AppServerRuntime` into `InitializeRequestProcessor`** and replace the config-based marker lookup with runtime-based lookup.
- [ ] **Step 4: Remove the `CredentialManager` fallback from the explicit injected-runtime path**. Keep configured Host startup as the only path that constructs the repository adapter and its private auth state.
- [ ] **Step 5: Run the focused AppServer test target and verify the injected-provider test passes.**
- [ ] **Step 6: Commit** `fix: keep injected provider authoritative during app-server startup`.

### Task 3: Remove concrete auth managers from model catalog code

**Files:**
- Modify: `crates/models-manager/src/manager.rs`
- Modify: `crates/models-manager/src/manager_tests.rs`
- Modify: `crates/models-manager/src/api_key_discovery_tests.rs`
- Modify: `crates/models-manager/src/cache_identity_tests.rs`
- Modify: `crates/model-provider/src/model_catalog_source.rs`
- Modify: `crates/core/src/thread_manager.rs`
- Test: existing models-manager tests plus a provider-neutral catalog auth test

**Interfaces:**
- Replace `Option<Arc<CredentialManager>>` and `fn credential_manager()` with a small neutral `ModelCatalogAuthContext` or provider-owned catalog result.
- `build_models_manager_with_provider` obtains catalog/auth behavior from `SharedModelProvider`; it does not forward a credential manager.

- [ ] **Step 1: Add a failing test** proving catalog discovery/filtering can be driven by provider-neutral authenticated/managed metadata rather than `CredentialManager`.
- [ ] **Step 2: Implement the neutral metadata input** and remove the `genex-login` dependency from production `models-manager` APIs.
- [ ] **Step 3: Update model-provider catalog adapters** so provider auth and model discovery remain inside the adapter.
- [ ] **Step 4: Update callers and tests**, preserving existing model visibility and cache identity behavior.
- [ ] **Step 5: Run model-manager compile-only tests through the required scope.**
- [ ] **Step 6: Commit** `refactor: make model catalog auth provider-neutral`.

### Task 4: Remove provider credential state from Core session execution

**Files:**
- Modify: `crates/core/src/thread_manager.rs`
- Modify: `crates/core/src/session/mod.rs`
- Modify: `crates/core/src/session/session.rs`
- Modify: `crates/core/src/session/turn_context.rs`
- Modify: `crates/core/src/session/review.rs`
- Modify: `crates/core/src/state/service.rs`
- Modify: `crates/core/src/genex_delegate.rs`
- Modify: `crates/core/src/prompt_debug.rs`
- Modify: `crates/core/src/tasks/review.rs`
- Modify: `crates/core/src/lib.rs`
- Test: Core provider snapshot/auth lifecycle tests and affected compile-only targets

**Interfaces:**
- `ThreadManager`, `SessionSpawnArgs`, `SessionServices`, `TurnContext`, review/delegate helpers, and prompt-debug APIs no longer accept `Arc<CredentialManager>`.
- Turn admission uses `SharedModelProvider::request_snapshot()` and passes only `ProviderRequestSnapshot`/opaque snapshot data to `ModelClient`.
- Any non-model service dependency must be represented by a narrow service interface, not the provider auth manager.

- [ ] **Step 1: Add or extend failing compile-only contract tests** for Core construction with a fake provider and no `CredentialManager`.
- [ ] **Step 2: Remove provider credential fields from Core-owned session/thread state**, retaining only provider-neutral auth state and opaque request snapshot data.
- [ ] **Step 3: Update thread start/resume/fork/delegate/review paths** so credential state is never copied as a provider selector or persisted field.
- [ ] **Step 4: Update prompt-debug and task-review paths** to use the provider handle or neutral service needed by the operation.
- [ ] **Step 5: Run `just check --tests` and inspect all Core errors/warnings related to removed credential parameters.**
- [ ] **Step 6: Commit** `refactor: remove provider credentials from core execution`.

### Task 5: Introduce and wire the Plugin credential broker

**Files:**
- Modify: `crates/core-plugins/Cargo.toml`
- Modify: `crates/core-plugins/src/manager.rs`
- Modify: `crates/core-plugins/src/test_support.rs`
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/core/src/thread_manager.rs`
- Modify: `crates/app-server/src/runtime.rs`
- Modify: `crates/app-server/src/message_processor.rs`
- Modify: `crates/cli/src/plugin_cmd.rs`
- Modify: `crates/cli/src/marketplace_cmd.rs`
- Test: Plugin owner-scoping and marketplace-auth tests

**Interfaces:**
- `PluginsManager::new_with_options` accepts `SharedCredentialBroker`, not `Arc<CredentialManager>`.
- The configured Host creates an adapter from its local credential/keyring implementation.
- Core-plugins requests credentials with explicit `CredentialOwner` and `CredentialPurpose`.

- [ ] **Step 1: Add failing owner-isolation tests** for Plugin and marketplace requests.
- [ ] **Step 2: Implement the Host adapter around existing local credential behavior**, preserving redaction, keyring choice, and auth storage semantics.
- [ ] **Step 3: Replace `PluginsManager::auth_mode()` and provider-auth checks** with broker capability/result handling.
- [ ] **Step 4: Wire the broker through AppServerRuntime and CLI marketplace/plugin startup.**
- [ ] **Step 5: Run core-plugins and app-server compile-only tests through the required scope.**
- [ ] **Step 6: Commit** `refactor: scope plugin credentials behind broker`.

### Task 6: Make MCP credential ownership explicit

**Files:**
- Modify: `crates/genex-mcp/src/rmcp_client.rs`
- Modify: `crates/genex-mcp/src/mcp/auth.rs`
- Modify: `crates/genex-mcp/src/connection_manager/startup.rs`
- Modify: `crates/core/src/mcp/mod.rs` or the existing MCP construction modules
- Modify: AppServer MCP construction call sites as required
- Test: existing MCP auth tests and a no-provider-auth integration/compile test

**Interfaces:**
- MCP constructors receive only MCP OAuth store/config dependencies; no model-provider `CredentialManager`.
- MCP bearer environment variables remain resolved only for the configured MCP server.

- [ ] **Step 1: Add a compile/test assertion** that MCP auth construction has no provider credential parameter.
- [ ] **Step 2: Remove accidental provider-auth parameters and preserve MCP OAuth/keyring/refresh behavior.**
- [ ] **Step 3: Run the existing MCP auth tests and compile-only targets.**
- [ ] **Step 4: Commit** `refactor: isolate mcp credential lifecycle`.

### Task 7: Migrate AppServer, user verification, CLI, and model-provider adapters

**Files:**
- Modify: `crates/app-server/src/user_verification.rs`
- Modify: `crates/app-server/src/outgoing_message.rs`
- Modify: `crates/app-server/src/otel_reloader.rs`
- Modify: `crates/app-server/src/request_processors/account_processor.rs`
- Modify: `crates/app-server/src/extensions.rs`
- Modify: `crates/cli/src/login.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/doctor.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`
- Test: runtime auth lifecycle tests, CLI compile-only targets, provider fake tests

**Interfaces:**
- AppServer account/login/logout/usage calls the injected provider directly.
- User verification and telemetry observe provider-neutral auth state/revision callbacks.
- `CredentialManager` remains private to the configured adapter and Host-only compatibility paths.

- [ ] **Step 1: Add failing runtime tests** for provider login/logout sharing and in-flight snapshot behavior.
- [ ] **Step 2: Move provider credential construction behind the configured adapter factory.**
- [ ] **Step 3: Replace AppServer and CLI provider-auth consumers with `ModelProvider` lifecycle calls.**
- [ ] **Step 4: Preserve existing OpenAI-compatible adapter behavior and external auth names.**
- [ ] **Step 5: Run targeted AppServer, CLI, and model-provider compile-only checks.**
- [ ] **Step 6: Commit** `refactor: route provider auth through model provider`.

### Task 8: Persistence, inventory, generated artifacts, and final verification

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md` if new follow-up findings are discovered
- Modify: relevant checked-in schema only if protocol types changed
- Test: source audits and repository checks

- [ ] **Step 1: Audit rollout/thread/history/session-log producers and consumers** for provider routing or credential fields; record explicit `remove`, `retain`, or `migrate-in-place` decisions before code changes.
- [ ] **Step 2: Run `rg` invariants** for `CredentialManager`, `GenexAuth`, `ModelProviderInfo`, provider IDs, and credentials in Core/persistence paths; investigate every production match.
- [ ] **Step 3: Run `just generate` if checked-in generated artifacts changed and inspect the complete diff.**
- [ ] **Step 4: Run `just check` through the required systemd scope.**
- [ ] **Step 5: Run `just check --tests` through the required systemd scope.**
- [ ] **Step 6: Verify `git diff --check`, `git status`, and all focused tests/compile targets.**
- [ ] **Step 7: Commit** `refactor: complete provider credential boundary`.

