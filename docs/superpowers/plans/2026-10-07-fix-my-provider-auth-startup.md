# Fix My Provider Authentication Startup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make an unauthenticated `my_provider` installation enter the authentication screen instead of failing TUI bootstrap with an empty model catalog.

**Architecture:** Keep the existing startup flow unchanged: app-server reports provider account state, and TUI decides whether to show onboarding from `requires_auth`. Make `MyModelProvider::account_state()` follow the same contract as the generic configured provider, and mark the built-in My Provider as requiring stored authentication. Dynamic `/models` discovery remains provider-owned and is attempted only after authentication in the normal TUI startup flow.

**Tech Stack:** Rust workspace, `genex-model-provider`, `genex-model-provider-info`, app-server account protocol, TUI startup, `just` verification recipes.

**Spec:** User report and root-cause audit from this task; no separate design document.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Preserve the provider-specific `APIKEY: apikey <token>` request authentication behavior.
- Do not change the generic TUI login-screen predicate or weaken the configured-provider authentication contract.
- Always run `just check` for the production workspace type-check.
- Run `just check --tests` because Rust production and test code will be changed; do not run standalone tests unless explicitly requested.
- Run Rust checks through the repository's user systemd memory-limited scope with `CARGO_BUILD_JOBS=2`.

## Review Focus

- Built-in My Provider with no stored credential: `GetAccountResponse` must be `{ account: None, requires_auth: true }`, allowing the TUI to show login before model discovery.
- Built-in My Provider with a stored API key: account state must be `{ account: Some(ApiKey), requires_auth: true }` and retain `APIKEY` header authentication.
- My Provider explicitly configured with `requires_auth = false`: it must remain unauthenticated and must not show the login screen.
- Stored authentication with a refresh failure: account state must not be reported as authenticated, matching the generic configured-provider behavior.
- An authenticated provider returning an empty `/models` response: this remains a separate model-catalog problem and must not be hidden by this authentication fix.

### Task 1: Align My Provider account state with provider configuration

**Files:**
- Modify: `crates/model-provider-info/src/lib.rs:334-340`
- Modify: `crates/model-provider/src/my_provider.rs:110-115`
- Test: `crates/model-provider/src/my_provider.rs` test module

**Interfaces:**
- Consumes: `ModelProviderInfo.requires_auth`, `AuthManager::auth_cached()`, `AuthManager::refresh_failure_for_auth()`, and `GenexAuth`.
- Produces: `MyModelProvider::account_state() -> ProviderAccountResult` that reports `requires_auth` from `ModelProviderInfo` and reports an account only for valid stored credentials.

- [x] **Step 1: Write failing account-state tests**

  Add focused tests for the My Provider implementation:

  - `my_provider_without_auth_requires_login`: construct the built-in provider without an `AuthManager` credential and assert `account == None` and `requires_auth == true`.
  - `my_provider_with_api_key_reports_api_key_account`: construct it with `GenexAuth::from_api_key("test-key")` and assert `account == Some(ProviderAccount::ApiKey)` and `requires_auth == true`.
  - `my_provider_without_auth_can_disable_login`: construct a My Provider `ModelProviderInfo` with `requires_auth = false` and assert `account == None` and `requires_auth == false`.

- [x] **Step 2: Compile the new tests against the unfixed implementation**

  Run the repository-compatible test-target compilation check for the affected crate through the memory-limited scope. The repository policy uses `just check --tests` as the test substitute; functional test execution is not part of the default cleanup verification. The assertions document the current failure: the implementation reports `Some(ApiKey)` and `requires_auth: false` even without credentials.

- [x] **Step 3: Make the built-in provider require stored authentication**

  In `ModelProviderInfo::create_my_provider()`, set `requires_auth: true` explicitly while retaining its existing name, base URL, and Chat wire API.

- [x] **Step 4: Implement `MyModelProvider::account_state()` using the configured-provider contract**

  In `crates/model-provider/src/my_provider.rs`, derive the account value only when `self.info.requires_auth` is true. Read the cached auth from the provider’s `AuthManager`; return no account when there is no cached auth or when `refresh_failure_for_auth()` reports a refresh failure. Map a valid API-key credential to `ProviderAccount::ApiKey`, and return `self.info.requires_auth` unchanged. Keep the existing My Provider request auth adapter untouched.

- [x] **Step 5: Compile the corrected tests**

  Re-run the affected crate's test-target compilation check. Expected result: the new account-state tests and existing My Provider API/header tests compile successfully. Execute the focused tests separately only if the user explicitly requests standalone test execution.

- [x] **Step 6: Commit the focused implementation**

  ```bash
  git add crates/model-provider-info/src/lib.rs crates/model-provider/src/my_provider.rs
  git commit -m "fix: route unauthenticated my provider through login"
  ```

### Task 2: Verify the startup contract and regression boundary

**Files:**
- Modify: `crates/model-provider/src/provider.rs` test module
- Test: `crates/model-provider/src/provider.rs` test module

**Interfaces:**
- Consumes: Task 1's `ProviderAccountState` behavior and the existing `GetAccountResponse` / `should_show_login_screen` startup boundary.
- Produces: regression coverage at the executable model-provider factory boundary, proving that missing My Provider credentials are represented as authentication-required before model-list bootstrap.

- [x] **Step 1: Add a factory-boundary regression test**

  Construct the built-in provider through `create_model_provider` with no credential, call `account_state()`, and assert exactly `account: None` and `requires_auth: true`. Do not call `/models` in this test; the purpose is to pin the authentication decision boundary used by app-server.

- [x] **Step 2: Run repository verification**

  Run, through the required user systemd scope:

  ```bash
  systemd-run --user --scope --collect --quiet \
    -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
    --setenv=CARGO_BUILD_JOBS=2 just check --tests
  systemd-run --user --scope --collect --quiet \
    -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
    --setenv=CARGO_BUILD_JOBS=2 just check
  git diff --check
  ```

  Expected result: both checks complete with zero errors; warnings are acceptable if they are pre-existing; `git diff --check` is clean.

- [x] **Step 3: Review the final diff and commit the regression test**

  Confirm that only the provider account-state/default-auth files and narrowly scoped regression tests changed, then commit the regression test with a message such as `test: cover my provider login bootstrap`.

## Handoff

After implementation, verify that an unauthenticated My Provider run reaches the authentication screen, and separately verify that an authenticated provider whose `/models` endpoint returns no models still reports the existing empty-catalog error rather than being misclassified as an authentication problem.
