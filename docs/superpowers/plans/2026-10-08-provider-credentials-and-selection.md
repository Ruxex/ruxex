# Provider Credentials and Selection Implementation Plan (Historical / Retired)

> **Historical note (2026-10-09):** This plan described the former multi-provider
> credential and selection model. It is retired and is not an implementation target.
> Genex now mounts one built-in provider per runtime/AppServer; provider selection,
> switching, and per-thread provider metadata are removed. The active design is
> [`2026-10-08-single-model-provider-library-design.md`](../specs/2026-10-08-single-model-provider-library-design.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single ChatGPT-style stored login with provider-scoped credentials, provider-aware account/usage status, and explicit `login`, `switch`, `logout`, and `delete` operations.

**Architecture:** Keep `genex-login` as the persistence and credential-state boundary, but evolve its single `AuthDotJson`/`CredentialManager` model into provider-scoped records. Reuse the existing configured `model_provider_id` as the effective active-provider selector rather than adding a competing selection source; `CredentialManager` supplies credentials and selection operations through a config-aware adapter. `ModelProvider` remains responsible for translating credentials and exposing provider-specific account/usage capabilities.

**Tech Stack:** Rust, Tokio, serde, keyring/secret storage, `genex-login`, `genex-model-provider`, `genex-models-manager`, `genex-core`, app-server protocol, CLI/TUI, checked-in schema generation.

**Spec:** `docs/superpowers/specs/2026-10-08-provider-credentials-and-selection-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Preserve keyring, `auth.json`, refresh, ephemeral storage, and authentication-change notification behavior unless an inventory decision explicitly removes a ChatGPT-specific capability.
- Preserve provider-specific endpoint fields and OpenAI-compatible wire fields; this work changes ownership and selection semantics, not upstream request schemas.
- Before each production change, audit producers and consumers and record an explicit `remove`, `retain`, or `migrate-in-place` decision in `docs/pre-release-compatibility-inventory.md`.
- Do not run standalone Rust test commands; use `just check --tests` to compile test targets and `just check` for the production workspace check.
- Run `just generate` whenever protocol, config, or checked-in generated artifacts are affected and inspect the resulting diff.
- Run Rust build/check commands through the repository-required user systemd scope with `CARGO_BUILD_JOBS=2` for workspace checks.

## Review Focus

- A stored credential for Provider A is never used for Provider B; cover storage lookup and `ModelProvider` resolution by provider ID.
- `login` activates a provider, `switch` changes only the active selection, `logout` clears only the active selection, and `delete` removes credentials; cover all four transitions.
- Legacy single-provider `auth.json` data remains readable and is migrated on the next write without exposing raw secrets in logs or status output.
- A provider with environment/static credentials works without a stored record, while a provider requiring stored auth reports the correct missing/invalid state.
- Account and usage information is optional and provider-scoped; unsupported capabilities do not appear as a fake global Genex account.

---

### Task 1: Introduce provider-scoped credential storage

**Files:**
- Modify: `crates/login/src/auth/storage.rs`
- Modify: `crates/login/src/auth/manager.rs`
- Modify: `crates/login/src/auth/mod.rs`
- Modify: `crates/login/src/lib.rs`
- Test: `crates/login/tests/all.rs` and its included test modules
- Test: `crates/app-server/tests/common/auth_fixtures.rs`

**Interfaces:**
- Add a serializable provider record containing the persisted auth mode and provider credential payload currently represented by `AuthDotJson`.
- Change `AuthDotJson` to represent provider-keyed records and an optional active-provider marker, while accepting the current single-provider shape on read.
- Add provider-scoped operations with `&str` provider IDs: `load_provider_auth`, `save_provider_auth`, `delete_provider_auth`, and `list_provider_ids`.

- [ ] **Step 1: Define the new persisted data model and migration reader**

  Add provider-keyed storage types in `crates/login/src/auth/storage.rs`. Accept the existing top-level `auth_mode`/`api_key` shape as a legacy record; do not treat an unrecognized auth mode as a valid credential. Keep file, keyring, secrets, and ephemeral backends using the same serialized value.

- [ ] **Step 2: Add provider-scoped storage operations**

  Implement load/save/delete for one provider without deleting other providers. `delete_provider_auth` must remove only that provider's record. Preserve existing storage permissions, keyring service, and secret namespace behavior.

- [ ] **Step 3: Add storage tests for isolation and migration**

  Test that two provider records round-trip independently, deletion leaves the other record intact, legacy single-provider data loads for the requested provider, and serialization contains no unrelated provider secret.

- [ ] **Step 4: Commit the storage layer**

  ```bash
  git add crates/login/src/auth crates/login/src/lib.rs crates/login/tests crates/app-server/tests/common/auth_fixtures.rs
  git commit -m "feat: store credentials per model provider"
  ```

### Task 2: Implement credential lifecycle and selection semantics

**Files:**
- Modify: `crates/login/src/auth/manager.rs`
- Modify: `crates/login/src/auth/change_state.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/core/src/config/auth_keyring.rs`
- Modify: `crates/core/src/session/mod.rs`
- Modify: `crates/core/src/state/service.rs`
- Test: `crates/login/tests/all.rs` and its included test modules
- Test: `crates/core/src/config/`

**Interfaces:**
- Make `CredentialManager` the provider-scoped public manager. No compatibility alias is retained because Genex is unreleased and all in-repository consumers are migrated together.
- Expose operations equivalent to:
  - `login(provider_id, credential)`: persist/update credentials and select the provider;
  - `switch(provider_id)`: select an existing usable provider without mutating credentials;
  - `logout()`: clear active provider selection without deleting credentials;
  - `delete(provider_id)`: delete only that provider's credentials and clear selection if active.
- Use the existing `Config.model_provider_id` as the effective runtime selection. Do not create a second uncoordinated active-provider field; the manager/config adapter must make the persistence and effective config agree.

- [ ] **Step 1: Separate deactivation from deletion**

  Replace the current `CredentialManager::logout` behavior that deletes all auth stores with a deactivation operation. Add an explicit provider deletion operation that calls the provider-scoped storage delete path. Preserve external-auth refresh and unauthorized recovery until their provider ownership is audited.

- [ ] **Step 2: Add provider-aware change state**

  Extend `AuthChangeState` or its replacement so consumers can determine which provider changed and whether the active selection changed, without including raw credentials. Preserve connection/user-verification invalidation semantics for the selected provider.

- [ ] **Step 3: Add config-aware login/switch helpers**

  Implement the adapter that validates a provider ID against the configured `model_providers` map, updates the effective selection, and rejects `switch` when no usable credential or configured environment credential exists. Explicit profile/session provider configuration remains higher priority than the default selection.

- [ ] **Step 4: Test the four lifecycle transitions**

  Add tests asserting: `login(A)` stores and selects A; `switch(B)` selects B while retaining A; `logout()` leaves A/B credentials intact and clears selection; `delete(B)` removes only B and clears selection when B is active.

- [ ] **Step 5: Commit the lifecycle layer**

  ```bash
  git add crates/login crates/core/src/config crates/core/src/session/mod.rs crates/core/src/state/service.rs
  git commit -m "feat: add provider credential lifecycle"
  ```

### Task 3: Make ModelProvider authentication provider-scoped

**Files:**
- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`
- Modify: `crates/model-provider/src/cloudflare_provider.rs`
- Modify: `crates/model-provider/src/my_provider.rs`
- Modify: `crates/model-provider/src/models_identity.rs`
- Modify: `crates/models-manager/src/manager.rs`
- Modify: `crates/core/src/client.rs`
- Test: `crates/model-provider/src/provider.rs`
- Test: `crates/model-provider/src/models_endpoint.rs`
- Test: `crates/model-provider/src/models_identity_tests.rs`

**Interfaces:**
- Replace the implicit single-manager lookup with provider-ID-scoped credential resolution.
- Keep `ModelProvider::api_auth`, `api_auth_for_scope`, `list_models`, and request recovery behavior, but resolve their credentials from the configured provider ID.
- Preserve explicit provider credentials (`env_key`, configured bearer token, headers, command auth) as higher-priority provider-local sources.

- [ ] **Step 1: Update provider auth resolution**

  Refactor `credential_manager_for_provider` and `resolve_provider_auth` so a configured provider asks for its own stored credential. Preserve unauthenticated providers, environment-key providers, Cloudflare Account ID handling, and My Provider header behavior.

- [ ] **Step 2: Update model listing and catalog identity**

  Make `ModelsEndpoint` and `ModelsManager` use the selected provider's credential record. Keep model cache identity partitioned by provider, endpoint, account identity, and effective auth scope without persisting raw credentials.

- [ ] **Step 3: Update Chat and provider-backed extensions**

  Update `core/src/client.rs`, Web Search, and History/Notes construction paths to request credentials for the configured provider. Do not reintroduce a global app-login check; capability checks must use provider auth status.

- [ ] **Step 4: Add cross-provider isolation tests**

  Verify that model list and Chat request construction for provider A never use provider B's stored token, explicit provider credentials still win, and switching providers changes the catalog identity and request auth source.

- [ ] **Step 5: Commit ModelProvider integration**

  ```bash
  git add crates/model-provider crates/models-manager crates/core/src/client.rs crates/ext/web-search crates/ext/history-notes
  git commit -m "refactor: resolve model auth per provider"
  ```

### Task 4: Add provider-scoped account and usage capabilities

**Files:**
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/cloudflare_provider.rs`
- Modify: `crates/model-provider/src/my_provider.rs`
- Modify: `crates/protocol/src/account.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/account.rs`
- Modify: `crates/app-server/src/request_processors/account_processor.rs`
- Test: `crates/protocol/src/account.rs`
- Test: `crates/model-provider/src/provider.rs`

**Interfaces:**
- Extend provider account state to include provider identity, optional account identifier/display name, auth status, and optional usage/rate-limit data.
- Add an optional `ModelProvider` usage capability that reports unsupported when the provider has no usage endpoint.
- Keep account and usage responses provider-scoped; do not present them as a global Genex account.

- [ ] **Step 1: Define provider-neutral account/usage values**

  Extend the existing `ProviderAccountState` and account protocol types instead of adding a second account model. Reuse existing rate-limit wire types where they match; represent unsupported provider capabilities explicitly.

- [ ] **Step 2: Add provider capability defaults**

  Implement default unsupported behavior on `ModelProvider`. Keep current API-key account-state behavior for Cloudflare and My Provider until their provider-specific account/usage endpoints are implemented.

- [ ] **Step 3: Adapt app-server account reads**

  Make `get_account_response` read the active provider's account and usage capability. Remove assumptions that a successful `CredentialManager` snapshot means a Genex account exists.

- [ ] **Step 4: Add protocol tests and generate schemas**

  Test account responses with supported, unsupported, unauthenticated, and invalid provider states. Run `just generate` and inspect only the intended account schema/generated diffs.

- [ ] **Step 5: Commit the provider account surface**

  ```bash
  git add crates/model-provider crates/protocol/src/account.rs crates/app-server-protocol crates/app-server/src/request_processors/account_processor.rs
  git commit -m "feat: expose provider account and usage state"
  ```

### Task 5: Rework CLI and app-server operations

**Files:**
- Modify: `crates/app-server/src/request_processors/account_processor.rs`
- Modify: `crates/app-server/src/message_processor.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/account.rs`
- Modify: `crates/cli/src/login.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/tui/src/onboarding/auth.rs`
- Modify: `crates/core-api/src/lib.rs`
- Test: `crates/app-server/tests/suite/v2/account.rs`
- Test: `crates/cli/src/login.rs`

**Interfaces:**
- Keep `login` as provider authentication plus activation.
- Add provider-targeted `switch` and `delete` operations.
- Change `logout` to deactivate only; credentials remain available for a later `switch` or `login`.
- Account/status output identifies the active provider and its optional account/usage data.

- [ ] **Step 1: Define provider-targeted protocol operations**

  Add the minimum request/response types needed for provider ID selection and deletion. Reuse existing account RPC routing only where the semantics remain clear; otherwise add provider-scoped methods in the same protocol wave.

- [ ] **Step 2: Implement app-server handlers**

  Route login, switch, logout, and delete through the provider-aware credential manager. Preserve auth-change notifications, plugin cache refresh, and connection invalidation only where they are still meaningful for ModelProvider state.

- [ ] **Step 3: Implement CLI/TUI behavior**

  Update status output to list configured providers, mark the active provider, and show masked credential/auth/usage state. Make `logout` deactivate only and add explicit `delete provider` behavior. Keep MCP login/logout commands on their independent MCP credential path.

- [ ] **Step 4: Update integration tests and schemas**

  Extend app-server account tests for multiple providers and the four transitions. Update snapshots and run `just generate` for changed protocol artifacts.

- [ ] **Step 5: Commit the user-facing operations**

  ```bash
  git add crates/app-server crates/app-server-protocol crates/cli/src/login.rs crates/cli/src/main.rs crates/tui/src/onboarding/auth.rs crates/core-api/src/lib.rs
  git commit -m "feat: manage model provider selection"
  ```

### Task 6: Remove obsolete global-login consumers and complete the audit

**Files:**
- Audit and modify: `crates/exec/src/lib.rs`
- Audit and modify: `crates/tui/src/startup_orchestration.rs`
- Audit and modify: `crates/app-server-transport/src/connection_auth.rs`
- Audit and modify: `crates/app-server/src/user_verification.rs`
- Audit and modify: `crates/app-server/src/user_verification_auth.rs`
- Audit and modify: `crates/core-plugins/src/manager.rs`
- Audit and modify: `crates/external-agent-migration/src/service.rs`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Replace global “logged in” checks with either active-provider availability or a generic credential revision, depending on the consumer.
- Preserve security invalidation and connection ownership checks; do not remove them merely because application login is gone.

- [ ] **Step 1: Classify each remaining `CredentialManager` consumer**

  For every production reference, record whether it needs provider credentials, active-provider selection, a generic revision, or no longer has a valid purpose. Add the explicit inventory decision before editing.

- [ ] **Step 2: Migrate valid consumers**

  Use provider-scoped credential state for model requests and provider account UI. Use a generic credential/owner revision for transport and user-verification security. Remove only dead plugin/startup/managed-auth paths with evidence.

- [x] **Step 3: Remove compatibility aliases and stale terminology**

  All consumers now use `CredentialManager`; the temporary compatibility alias was removed. ChatGPT-specific managed-auth terminology remains only where it describes an active security/configuration capability.

- [ ] **Step 4: Commit the final consumer audit**

  ```bash
  git add crates docs/provider-decoupling-candidates.md docs/pre-release-compatibility-inventory.md
  git commit -m "refactor: remove global model login assumptions"
  ```

### Task 7: Run repository verification and final provider audit

**Files:**
- Verify: all files changed by Tasks 1-6
- Verify: `docs/superpowers/specs/2026-10-08-provider-credentials-and-selection-design.md`
- Verify: `docs/provider-decoupling-candidates.md`
- Verify: `docs/pre-release-compatibility-inventory.md`

- [ ] **Step 1: Generate checked-in artifacts**

  Run `just generate`, inspect the diff, and confirm that only intentional provider/account/config changes are generated.

- [ ] **Step 2: Compile Rust test targets**

  ```bash
  systemd-run --user --scope --collect --quiet \
    -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
    --setenv=CARGO_BUILD_JOBS=2 just check --tests
  ```

  Expected: success with all affected Rust test targets compiled.

- [ ] **Step 3: Run the production workspace check**

  ```bash
  systemd-run --user --scope --collect --quiet \
    -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
    --setenv=CARGO_BUILD_JOBS=2 just check
  ```

  Expected: success with no provider-auth, protocol, or credential-storage errors.

- [ ] **Step 4: Search for stale semantics and inspect the final diff**

  Run `git diff --check` and search for `logged in`, `CredentialManager::logout`, `current_auth_uses_managed_auth`, single-provider `AuthDotJson`, and unscoped `GenexAuth` consumers. Every remaining match must have a documented provider, MCP, or security reason.

- [ ] **Step 5: Commit the verified result**

  ```bash
  git add docs crates
  git commit -m "refactor: complete provider credential management"
  ```
