# Remove residual ChatGPT references from Rust sources Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Audit and remove or provider-neutralize every case-insensitive `chatgpt` match in Rust sources, retaining only explicitly justified compatibility or active external-contract matches documented in the compatibility inventory.

**Architecture:** Work from the existing producer/consumer boundaries rather than replacing text mechanically. Process persisted/protocol compatibility first, then provider auth and endpoint adapters, runtime/security shims, and finally tests/fixtures; each slice updates the inventory before code changes and ends with the repository's required checks.

**Tech Stack:** Rust workspace, `rg`, `just check`, `just check --tests`, `just generate`, Cargo metadata, checked-in protocol artifacts, and Git commits on `main`.

**Spec:** `docs/superpowers/specs/2026-10-01-remove-chatgpt-rust-residuals-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Include Rust code, tests, comments, doc comments, and string literals in the case-insensitive audit.
- Audit every production match and update `docs/pre-release-compatibility-inventory.md` before changing that match.
- Do not split, encode, or otherwise obscure a retained string merely to make the search pass.
- Preserve current provider-neutral behavior, platform fallbacks, security predicates, and active persisted readers unless the audit proves the ChatGPT-specific path is obsolete.
- Do not add migrations for obsolete pre-release persisted data; retain a reader only when current compatibility requires it.
- Run `just check --tests` for Rust changes and always run `just check`; run `just generate` when generated artifacts may be affected.
- Do not run standalone tests unless explicitly requested.
- Make one focused commit per completed task, including the task's inventory and verification changes.

## Review Focus

- Persisted session/history values: existing records must remain readable or the removal must be justified by the pre-release compatibility boundary.
- Account/auth metadata: removing a product-specific header must not silently send a renamed header to an unrelated provider or weaken account/cache isolation.
- External protocol metadata: active privacy/security and wire contracts must be isolated or explicitly held, not broken by a source-level rename.
- Provider-neutral fixtures: test helpers and snapshots must continue to exercise generic API-key, header, Bedrock, MCP, plugin, and security paths without hosted product names.
- Generated artifacts and final search: source, generated schema, inventory, and script output must agree, with no unexplained Rust match.

### Task 1: Build the residual inventory

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Inspect: `scripts/count-provider-chatgpt.sh`
- Inspect: all Rust files returned by `rg -ni 'chatgpt' --glob '*.rs' .`

**Interfaces:**
- Consumes: the current Rust search result and existing inventory rows.
- Produces: one inventory row per production capability/match group with producer, consumer, canonical replacement, decision, and verification command.

- [ ] **Step 1: Capture the baseline search**

  Run:

  ```bash
  rg -ni 'chatgpt' --glob '*.rs' .
  rg -ni 'chatgpt' --glob '*.rs' . | wc -l
  rg -ni 'chatgpt' crates --glob '*.rs' --glob '!**/tests/**' --glob '!**/*_tests.rs'
  ```

  Separate compiled production code, `#[cfg(test)]` modules, integration tests, comments, and string literals. Do not classify a source path as production without checking its module/test configuration.

- [ ] **Step 2: Reconcile existing inventory rows**

  For each match already covered by a completed row, verify that the current path and symbol still agree with the row. Add or revise rows for residuals in the ordered sections: persisted/history, protocol, CLI/config/import, UI/runtime/observability, build/experimental, and final dependency/documentation audit.

- [ ] **Step 3: Commit the audit record**

  Run `git diff --check`, inspect the inventory diff, and commit only the inventory changes:

  ```bash
  git add docs/pre-release-compatibility-inventory.md
  git commit -m "docs: inventory residual ChatGPT Rust references"
  ```

### Task 2: Resolve persisted and protocol-bound matches

**Files:**
- Modify: `crates/protocol/src/protocol.rs`
- Modify: `crates/rollout/src/tests.rs`
- Inspect/modify: `crates/app-server/src/request_processors/thread_resume_redaction.rs`
- Inspect/modify: `crates/app-server/src/user_verification_test_support.rs`
- Inspect/modify: `crates/codex-api/src/sse/responses.rs`
- Modify: adjacent protocol/app-server/codex-api tests and generated artifacts only when the producer/consumer audit requires it
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: persisted `SessionSource`/product restriction values, remote client names, response metadata, and test verification payloads.
- Produces: the current persisted/history and privacy behavior with no obsolete product naming in ordinary Rust code, or narrowly documented hold rows for values that are externally immutable.

- [ ] **Step 1: Trace persisted producers and readers**

  Inspect `SessionSource`, product restriction conversion, rollout readers, and thread-resume redaction callers. Determine whether each `chatgpt` value is produced by current repository code, read from persisted history, or only used by a test. Record the decision before editing.

- [ ] **Step 2: Remove or generalize obsolete protocol metadata**

  Remove dead ChatGPT-only response metadata parsing and fixtures when no current consumer remains. Preserve response-only redaction and generic safety/privacy behavior; if named external client identifiers are still an active contract, isolate them and record an explicit hold with the exact consumer and removal condition.

- [ ] **Step 3: Update protocol tests and generated outputs**

  Replace test-only product labels with generic unknown/custom values where the test intent is restriction behavior, or delete obsolete hosted tests. Run `just generate` if protocol/schema source changes, inspect the full diff, then run `just check --tests` and `just check`.

- [ ] **Step 4: Commit the persisted/protocol slice**

  ```bash
  git add crates/protocol crates/rollout crates/app-server crates/codex-api docs/pre-release-compatibility-inventory.md
  git commit -m "refactor: remove ChatGPT-specific protocol remnants"
  ```

### Task 3: Remove ChatGPT-specific auth and endpoint adapters

**Files:**
- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/bearer_auth_provider.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`
- Modify: `crates/response-debug-context/src/lib.rs`
- Inspect/modify: `crates/login/src/auth/manager.rs`
- Modify: related model-provider, login, codex-api, core, and TUI auth fixtures
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: provider auth resolution, model discovery routing, response debug context, and account-continuity checks.
- Produces: explicit provider-owned or provider-neutral auth/endpoint behavior without leaking a ChatGPT header or hosted URL into arbitrary configured providers.

- [ ] **Step 1: Audit the auth boundary**

  Trace `BearerAuthProvider`, account-id propagation, `ChatGPT-Account-ID`, FedRAMP/account identity fields, and all callers. Decide whether the header belongs to an active provider-owned adapter, is dead, or can be removed while preserving cache separation and account continuity. Do not rename the header to a generic wire value without an identified consumer.

- [ ] **Step 2: Audit model discovery and debug URLs**

  Trace `models_endpoint` URL selection and response debug context construction. Remove implicit hosted endpoints when no current producer requires them; preserve explicit configured provider routing and generic request diagnostics.

- [ ] **Step 3: Implement the selected boundary changes**

  Remove obsolete adapters and their tests, or move unavoidable external wire strings to the smallest provider-owned boundary and add an inventory hold. Update auth fixtures to use generic provider/API-key terminology where they do not test the external contract.

- [ ] **Step 4: Verify and commit**

  Run:

  ```bash
  just check --tests
  just check
  git diff --check
  git add crates/model-provider crates/response-debug-context crates/login crates/codex-api crates/core crates/tui docs/pre-release-compatibility-inventory.md
  git commit -m "refactor: remove ChatGPT-specific auth endpoints"
  ```

### Task 4: Remove ChatGPT-specific runtime and security labels

**Files:**
- Inspect/modify: `crates/core/src/thread_manager.rs`
- Inspect/modify: `crates/ext/web-search/src/tool.rs`
- Inspect/modify: `crates/cli/src/doctor/desktop/*_security_tests.rs`
- Inspect/modify: `crates/cli/src/doctor/security_tests.rs`
- Inspect/modify: `crates/network-proxy/src/network_policy.rs`
- Inspect/modify: `crates/core/src/config/auth_keyring_tests.rs`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: web-search request metadata, security diagnostics, network audit metadata, and auth-policy tests.
- Produces: unchanged security predicates and provider-neutral diagnostics with no obsolete ChatGPT product labels.

- [ ] **Step 1: Separate labels from security behavior**

  Confirm whether each occurrence is only a test label/fixture, an external service metadata value, or part of a trust/redaction predicate. Preserve generic security checks and remove only obsolete product aliases or hosted routing.

- [ ] **Step 2: Generalize or delete test cases**

  Rename generic test helpers/metadata and replace hosted-only values with neutral fixtures. Delete tests whose only behavior is a removed ChatGPT-hosted path. Keep negative security cases that prove untrusted processes/origins remain rejected.

- [ ] **Step 3: Verify and commit**

  Run `just check --tests`, `just check`, and `git diff --check`, inspect platform-specific test source changes, then commit:

  ```bash
  git add crates/core crates/ext/web-search crates/cli crates/network-proxy docs/pre-release-compatibility-inventory.md
  git commit -m "refactor: generalize ChatGPT runtime fixtures"
  ```

### Task 5: Clean up provider-neutral TUI, CLI, MCP, plugin, and app-server fixtures

**Files:**
- Modify: `crates/tui/src/chatwidget/tests/helpers.rs` and its importing test modules
- Modify: `crates/tui/src/app/tests.rs`, `crates/tui/src/app/agents_overview_tests.rs`, and related TUI fixtures
- Modify: residual files under `crates/app-server/tests/suite/v2/`
- Modify: the files under `crates/cli/tests/`, `crates/core/tests/`, `crates/ext/skills/`, and `crates/exec/tests/` returned by Task 1's baseline `rg -ni 'chatgpt' --glob '*.rs'` search, plus `crates/core-plugins/src/marketplace_tests.rs` and `crates/codex-api/tests/clients.rs`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: existing generic API-key, external-auth, Bedrock, MCP, plugin, skills, model, and UI test helpers.
- Produces: equivalent test coverage without ChatGPT names, or removal of tests that exercise deleted hosted capabilities.

- [ ] **Step 1: Group fixtures by behavior**

  For every remaining test match, classify it as generic auth setup, obsolete hosted plugin/Apps flow, external protocol fixture, security negative case, persisted compatibility case, or test-only wording. Do not mass-replace values that are part of an active external contract.

- [ ] **Step 2: Rename generic helpers and fixtures**

  Rename `set_chatgpt_auth` and similar helper names to describe the auth mechanism, update all imports/callers, and replace generic test labels/tokens/URLs with neutral values. Preserve assertions about headers only where the header is an active external contract and inventory records the hold.

- [ ] **Step 3: Delete orphaned hosted suites**

  Remove tests, fixtures, and imports whose only producer/consumer was a deleted ChatGPT Apps, plugin marketplace, managed-auth, or hosted web-search capability. Keep mixed tests for ordinary MCP, OAuth, plugins, skills, sandboxing, and provider-neutral security.

- [ ] **Step 4: Verify and commit**

  Run `just check --tests`, `just check`, and `git diff --check`, then commit the fixture slice:

  ```bash
  git add crates/tui crates/app-server/tests crates/cli/tests crates/core/tests crates/core-plugins crates/ext/skills crates/exec/tests crates/codex-api/tests docs/pre-release-compatibility-inventory.md
  git commit -m "test: remove ChatGPT-specific fixtures"
  ```

### Task 6: Final generated-artifact, dependency, and search audit

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md` only to record this slice's completion or remaining explicit holds
- Modify: generated artifacts only when produced by `just generate`
- Inspect: `scripts/count-provider-chatgpt.sh` output and all Rust source

**Interfaces:**
- Consumes: all preceding cleanup commits and their inventory decisions.
- Produces: verified final state with zero unexplained case-insensitive `chatgpt` Rust matches and a complete hold ledger if any exact external strings remain.

- [ ] **Step 1: Regenerate and inspect**

  Run `just generate`, inspect the complete diff, and retain only generated changes caused by the cleanup. Run `cargo metadata --no-deps` or the repository dependency audit if workspace edges were removed.

- [ ] **Step 2: Run the final searches**

  Run:

  ```bash
  rg -ni 'chatgpt' --glob '*.rs' .
  rg -ni 'chatgpt' crates --glob '*.rs' --glob '!**/tests/**' --glob '!**/*_tests.rs'
  scripts/count-provider-chatgpt.sh /tmp/chatgpt-line-counts.tsv
  git diff --check
  ```

  Expected: no unexplained Rust match; every remaining match is listed exactly in the inventory with a valid hold reason. The script output is inspected separately because it also counts `provider`.

- [ ] **Step 3: Run repository verification**

  Run `just check --tests` and `just check`. Do not run standalone tests. Inspect the final `git diff`, status, commit log, and inventory verification commands.

- [ ] **Step 4: Commit the final audit**

  Stage `docs/pre-release-compatibility-inventory.md`, `docs/provider-decoupling-candidates.md`, and the exact generated files reported by `just generate`, then commit:

  ```bash
  git commit -m "docs: close ChatGPT Rust residual audit"
  ```
