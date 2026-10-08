# Single-Provider Genex Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate Genex from runtime provider selection to a host-injected, single-provider runtime with provider-scoped data roots and provider-neutral contracts.

**Architecture:** Reuse the existing `genex-model-provider` boundary as the first transition point, then move provider-neutral contracts above provider-specific transport and configuration. A Genex runtime/AppServer will own one provider instance; provider identity will later be validated by a marker in the configured data root rather than persisted in thread routing state.

**Tech Stack:** Rust workspace, Tokio futures, serde, SQLite-backed state, app-server protocol, existing `genex-model-provider` adapters.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`

## Global Constraints

- Keep one `ModelProvider` per Genex runtime/AppServer; provider switching and fallback remain outside the core runtime.
- Preserve provider-neutral agent, MCP, sandbox, security, and platform fallback behavior.
- Preserve OpenAI endpoint URLs, wire fields, authentication headers/variables, and model identifiers when they are external contracts.
- Audit producers and consumers before changing persisted, protocol, CLI/configuration, or schema fields.
- Run Rust checks through the repository's memory-limited `systemd-run --user --scope` recipes.

## Review Focus

- A configured provider without an explicit provider ID must still receive a deterministic descriptor: contract tests in Task 1.
- A provider ID containing user-visible names must round-trip through the descriptor without normalization: contract tests in Task 1.
- Optional capabilities must distinguish unsupported from unavailable rather than inventing empty values: capability tests in Task 2.
- A mismatched data-root marker must be rejected before state or credentials are opened: mount tests in Task 4.
- Existing threads and rollouts must remain resumable while their provider-routing fields are being migrated: persistence tests in Task 6.

## Implementation Progress

- The provider descriptor, common capability flags, and provider-home marker contract are implemented.
- AppServer no longer applies a persisted active-provider selection to reloaded configuration.
- AppServer now mounts and validates the configured provider home before credentials and SQLite state are initialized.
- AppServer constructs one shared provider runtime and reuses it for sessions, model catalogs, account state, and capability queries.
- AppServer model-provider/account status APIs expose only the configured provider; provider switching is no longer represented in runtime, host commands, or app-server protocol.
- Runtime resume and AgentControl no longer restore or select a provider from persisted thread metadata; the mounted provider is authoritative.
- App-server `thread/list` no longer exposes a provider filter; listing is always scoped to the mounted provider. Lower thread-store/rollout filter parameters remain under the final persistence audit.
- `thread/start`, `thread/resume`, and `thread/fork` no longer accept provider request overrides; TUI/CLI producers and AppServer override/mismatch consumers use the mounted provider implicitly. Response provider status remains available.
- Lower thread-store/state/rollout provider list filters and named-session provider allowlists are removed; historical rollout `SessionMeta.model_provider` is no longer a protocol field and old JSONL keys are ignored by serde.
- Thread-store live metadata, stored-thread projections, and metadata patches no longer carry provider routing fields; historical rollout `SessionMeta.model_provider` is deserialized but ignored, while API responses continue to report the mounted provider.
- Stored-thread responses now expose the mounted provider so clients cannot restore a historical provider into their local session state.
- New rollout session metadata no longer writes `model_provider`; optional deserialization remains for historical JSONL and state backfill.
- SQLite state migration `0002` removes `threads.model_provider` and `idx_threads_provider`; state metadata and runtime SQL no longer persist or filter threads by provider. Historical rollout values remain read-only migration input.
- Provider-switching host surfaces are removed: CLI `login switch`, TUI provider selection/switching, `Config::switch_model_provider`, and app-server `account/switch` plus generated schema artifacts. The configured provider is used directly; another provider requires another AppServer/data root.
- CredentialManager no longer persists or applies `active_provider`, rejects login/delete operations for a different configured provider, and treats logout as deletion of the configured provider's credentials; file, keyring, ephemeral, and legacy-read storage remain.
- Historical provider fields have now been removed from live thread/state projections and from the `SessionMeta` protocol type; old JSONL keeps loading because unknown fields are ignored. Remaining work is the final host-surface and provider-neutral contract audit.
- The mounted-provider list response no longer exposes `activeProviderId` or per-entry `active` selection state; it reports the single mounted provider and its auth fields.
- `StateRuntime` no longer stores or consults a provider identity; the retained initialization parameter is compatibility-only while state metadata construction is provider-neutral.
- Rollout configuration and local thread-store configuration no longer carry a provider ID. Metadata building, rollout backfill/reconciliation, and thread listing/resume no longer accept provider-only arguments.
- Persisted `ThreadSettingsSnapshot.model_provider_id` is removed; old rollout event objects remain readable because the field is ignored during deserialization, while `SessionConfiguredEvent.model_provider_id` remains only as current-provider client status.
- The provider status endpoint is singular: `modelProvider/list` and its array response were replaced with `modelProvider/read`, which returns the mounted provider and its authentication fields directly.

### Task 1: Add the provider descriptor contract

**Files:**
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Test: `crates/model-provider/src/provider.rs` unit tests

**Interfaces:**
- Consumes: existing `ModelProvider::info()` and `ModelProvider::provider_id()` values.
- Produces: `ProviderDescriptor`, `CURRENT_PROVIDER_PROTOCOL_VERSION`, and `ModelProvider::descriptor()` for later data-root marker validation.

- [x] **Step 1: Write the failing descriptor tests**

  Add tests for an explicitly identified configured provider and an unnamed configured provider. Assert that `descriptor().provider_kind` uses the explicit provider ID when present, otherwise the existing provider name, and that `provider_protocol_version` is the current constant. Assert serde round-trip for the descriptor because it will be stored in `genex-home.json` in Task 4.

- [x] **Step 2: Run the focused tests and verify the expected compile failure**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=4 just check --tests -p genex-model-provider`.

  Expected: FAIL because `ProviderDescriptor`, the protocol-version constant, and `ModelProvider::descriptor()` do not yet exist.

- [x] **Step 3: Implement the minimal descriptor contract**

  Define a serde-serializable, cloneable, equality-comparable `ProviderDescriptor` with `provider_kind: String` and `provider_protocol_version: String`. Add `CURRENT_PROVIDER_PROTOCOL_VERSION: &str = "1"`. Add the default `descriptor()` method to `ModelProvider`, deriving the kind from `provider_id()` or `info().name` without changing either existing value. Re-export both from `genex_model_provider`.

- [x] **Step 4: Run the focused tests and workspace checks**

  Run the focused `just check --tests -p genex-model-provider`, then run repository-required `just check --tests` and `just check` through the memory-limited scope. Expected: all commands pass; existing provider behavior is unchanged.

- [x] **Step 5: Commit the contract foundation**

  ```bash
  git add crates/model-provider/src/provider.rs crates/model-provider/src/lib.rs
  git commit -m "feat: add model provider descriptor contract"
  ```

### Task 2: Define provider-neutral operation and capability results

**Files:**
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Modify: affected core/app-server consumers after the audit
- Test: provider contract and app-server capability tests

Introduce normalized capability, auth-state, model-catalog, usage, stream-event, and provider-error types. Keep existing types as adapters until all consumers migrate. Add explicit `Unsupported`, `Unauthenticated`, `Unauthorized`, `InvalidRequest`, `RateLimited`, `Transient`, `Transport`, and provider-specific error classification without leaking credentials.

### Task 3: Inject one provider into runtime and AppServer

**Files:**
- Modify: `crates/core/src/session/mod.rs`, `crates/core/src/client.rs`, and runtime construction paths
- Modify: `crates/app-server/src/config_manager.rs` and request processors
- Test: core/AppServer construction and shared-client behavior

Make runtime construction accept one `SharedModelProvider`, remove active-provider selection from runtime state, and route model/chat/auth/usage operations through that instance. Keep configuration migration separate from this task where persisted fields are still required by readers.

### Task 4: Add configurable provider-scoped data roots and mount markers

**Files:**
- Create: provider-neutral data-root marker module in the lowest dependency crate that can be shared by config, state, login, and AppServer
- Modify: data-root resolution and runtime startup paths
- Test: marker initialization, reuse, missing marker, version mismatch, provider mismatch, and pre-open rejection

Use `.genex` as the fallback default. Resolve the effective path in the documented precedence order: builder, startup environment/CLI, build-time default, then `.genex`. Write and validate `genex-home.json` before opening SQLite, credentials, sessions, or caches.

### Task 5: Unify shared credential lifecycle

**Files:**
- Modify: `crates/login/src/auth/manager.rs` and provider adapters
- Modify: core/AppServer auth watchers and turn startup
- Test: credential update snapshot, logout rejection, in-flight turn behavior, and multi-client broadcast

Keep credential values out of threads and rollouts. Same-provider credential changes affect new turns; in-flight turns retain their resolved snapshot. Logout rejects new turns and exposes an authentication error to in-flight requests that cannot continue.

### Task 6: Remove provider routing from thread, rollout, and protocol persistence

**Files:**
- Audit and modify: `crates/state`, `crates/rollout`, `crates/core`, `crates/app-server`, `crates/protocol`, and `crates/app-server-protocol`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Test: resume/history/rollout/schema and migration tests

Enumerate every reader and writer of `model_provider` and equivalent fields. Record an explicit decision before changing each boundary. Remove provider selection from execution, migrate retained historical fields where necessary, and replace provider list/select/switch APIs with current-provider status, capabilities, model catalog, auth, and usage operations.

### Task 7: Migrate host surfaces and complete the audit

**Files:**
- Modify: CLI, TUI, app-server clients, configuration, fixtures, generated schemas, and documentation identified by the producer/consumer audit
- Modify: `docs/provider-decoupling-candidates.md` and `docs/pre-release-compatibility-inventory.md`
- Test: focused host tests, `just generate` where generated artifacts are affected, `just check --tests`, and `just check`

Remove dead provider registry and switching paths, document separate runtime/AppServer startup for another provider, and verify the final completion gate from the design document.
