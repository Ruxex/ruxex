# Amazon Bedrock 対応削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Genex から Amazon Bedrock の runtime/provider、認証、protocol、設定、UI、テスト、専用 AWS 認証クレートを完全に削除する。

**Architecture:** Bedrock の producer/consumer を protocol 境界から内側へ順に削除し、最後に model-provider と `genex-aws-auth` の依存グラフを縮小する。provider-neutral な custom provider、API key/headers/external bearer 認証、一般的な `/models` 取得、OpenAI endpoint 契約は変更しない。

**Tech Stack:** Rust workspace, Cargo, app-server protocol/schema generator, JSON/TypeScript generated artifacts, `just` verification commands.

**Spec:** `docs/superpowers/specs/2026-10-05-remove-bedrock-design.md`

## Global Constraints

- Work directly on the existing `main` branch; do not create a worktree.
- This is a pre-release fork: do not add compatibility aliases/readers for removed Bedrock protocol, config, or persisted-data names.
- Preserve OpenAI endpoint URLs, wire fields, authentication variables/headers, and model identifiers.
- Preserve provider-neutral custom providers, API key/headers/external bearer auth, generic `/models`, platform fallbacks, and security behavior.
- Record every producer/consumer decision in `docs/pre-release-compatibility-inventory.md` and update `docs/provider-decoupling-candidates.md`.
- Do not run standalone test commands; use `just check --tests` for Rust test-target compilation and `just check` for production type-checking.
- Run `just generate` when checked-in generated artifacts are affected and inspect the resulting diff.

## Review Focus

- Removal of Bedrock protocol variants must not remove API-key or headers auth variants; verify generated JSON/TypeScript schemas and remaining account/login serialization.
- Removing AWS auth types must not break non-Bedrock workspace consumers; verify exact `genex-aws-auth` and `AwsAuth*` consumer audits before deleting the crate.
- Removing built-in Bedrock providers must not remove generic configured providers or provider-owned dynamic `/models` retrieval; verify the provider registry and model manager paths.
- Removing Bedrock config fields must not leave schema-only or config-editor paths that accept unsupported fields; verify `config.schema.json` and config parsing tests.
- Generated artifacts and documentation must not continue advertising Bedrock support while retained OpenAI/provider wire contracts remain unchanged; verify exact residual searches and generated diff.

---

### Task 1: Remove Bedrock protocol and generated API surface

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/common.rs`, `crates/app-server-protocol/src/protocol/v2/account.rs`, `crates/app-server-protocol/src/protocol/v2/mod.rs`, `crates/app-server-protocol/src/export.rs`
- Delete: `crates/app-server-protocol/src/protocol/v2/bedrock.rs`
- Modify: `crates/protocol/src/account.rs`, `crates/protocol/src/auth.rs` where Bedrock-only types are re-exported or consumed
- Modify: `crates/app-server-protocol/schema/json/**`, `crates/app-server-protocol/schema/typescript/**` through the repository generator
- Modify: `docs/pre-release-compatibility-inventory.md`, `docs/provider-decoupling-candidates.md` for the pre-change decision record
- Test targets: protocol serialization/schema fixture tests in `crates/app-server-protocol`

**Interfaces:**
- Consumes: Existing provider-neutral `AuthMode`, account, login, and app-server protocol definitions.
- Produces: Protocol types containing only the remaining auth/account variants; no Bedrock request/response routes or schema variants.

- [ ] **Step 1: Record the removal decision before changing production code**

  Add the audited Bedrock producer/consumer groups, the `genex-aws-auth` boundary, and explicit `remove`/`retain` decisions to `docs/pre-release-compatibility-inventory.md` and `docs/provider-decoupling-candidates.md`. Record that OpenAI endpoint/wire/auth/model contracts and provider-neutral capabilities are retained.

- [ ] **Step 2: Audit and remove Bedrock protocol producers and tests**

  Remove Bedrock API-key/access-key auth variants, Bedrock account/login variants, `account/bedrock/discover`, `account/bedrock/setup`, their request/response types, exports, modules, and serialization assertions. Keep API key, headers, and other active provider-neutral protocol shapes unchanged.

- [ ] **Step 3: Regenerate checked-in protocol artifacts**

  Run `just generate` and inspect the diff. Confirm generated JSON/TypeScript files remove only Bedrock-owned schemas and references.

- [ ] **Step 4: Compile affected targets**

  Run `just check --tests`.

  Expected: exit 0; protocol, app-server protocol, and all test targets compile without Bedrock types.

- [ ] **Step 5: Commit**

  ```bash
  git add crates/app-server-protocol crates/protocol
  git commit -m "refactor: remove Bedrock protocol surfaces"
  ```

### Task 2: Remove Bedrock runtime, auth, config, CLI, and TUI consumers

**Files:**
- Delete: `crates/app-server/src/request_processors/account_processor/bedrock_setup.rs`, `crates/app-server/src/request_processors/bedrock_auth.rs`, `crates/app-server/tests/suite/v2/bedrock_setup.rs`
- Modify: `crates/app-server/src/auth_mode.rs`, `crates/app-server/src/config_manager.rs`, `crates/app-server/src/message_processor.rs`, `crates/app-server/src/request_processors.rs`, `crates/app-server/src/request_processors/account_processor.rs`, `crates/app-server/README.md`
- Modify/delete: `crates/app-server/tests/common/auth_fixtures.rs`, `crates/app-server/tests/common/test_app_server.rs`, related v2 suite registrations and Bedrock-only cases
- Modify: `crates/app-server-test-client/src/lib.rs`, `crates/app-server-test-client/README.md`
- Modify: `crates/login/src/auth/manager.rs`, `crates/login/src/auth/mod.rs`, `crates/login/src/auth/storage.rs`, `crates/login/src/lib.rs`
- Delete: `crates/login/src/auth/bedrock_access_keys.rs`, `crates/login/src/auth/bedrock_api_key.rs`
- Modify/delete: `crates/cli/src/doctor.rs`, `crates/cli/src/doctor/network_tests.rs`, `crates/cli/src/login.rs`, `crates/cli/tests/daemon_startup.rs`
- Modify/delete: `crates/tui/src/onboarding/bedrock.rs`, `crates/tui/src/onboarding/bedrock_tests.rs`, `crates/tui/src/onboarding/{auth.rs,mod.rs,onboarding_screen.rs}`, `crates/tui/src/app_server_session.rs`, `crates/tui/src/startup_orchestration.rs`, `crates/tui/src/lib.rs`
- Modify/delete: Bedrock-specific config edit and config test paths under `crates/core/src/config`, `crates/config/src`, and `crates/core/config.schema.json`

**Interfaces:**
- Consumes: Task 1’s provider-neutral protocol types and routes.
- Produces: app-server, login, CLI, TUI, and config paths with no Bedrock runtime or authentication branch.

- [ ] **Step 1: Remove runtime and auth consumers**

  Delete Bedrock app-server processors, login auth modules, test-client modes, CLI/TUI onboarding branches, Bedrock-only fixtures, and config edit/validation paths. Remove now-unused imports, modules, and dependency declarations while preserving active generic auth flows.

- [ ] **Step 2: Audit config schemas and user-facing docs**

  Remove Bedrock-only TOML fields, schema entries, examples, help text, and README instructions. Ensure unknown/removed Bedrock configuration is not silently accepted by a leftover schema or parser path.

- [ ] **Step 3: Compile affected targets**

  Run `just check --tests`.

  Expected: exit 0; app-server, login, CLI, TUI, config, and integration test targets compile with no Bedrock references.

- [ ] **Step 4: Commit**

  ```bash
  git add crates/app-server crates/app-server-test-client crates/login crates/cli crates/tui crates/config crates/core
  git commit -m "refactor: remove Bedrock runtime consumers"
  ```

### Task 3: Remove Bedrock model provider and AWS signing crate

**Files:**
- Delete: `crates/model-provider/src/amazon_bedrock/**`, `crates/aws-auth/**`
- Modify: `crates/model-provider/src/lib.rs`, `crates/model-provider/src/provider.rs`, `crates/model-provider/src/shared_state.rs`, `crates/model-provider/Cargo.toml`
- Modify/delete: `crates/model-provider-info/src/lib.rs`, `crates/model-provider-info/src/model_provider_info_tests.rs`
- Modify: `crates/app-server/Cargo.toml`, root `Cargo.toml`, `Cargo.lock`
- Modify/delete: Bedrock-only tests under `crates/core/tests/suite/external_auth.rs`, `crates/config/src/bedrock_runtime_tests.rs`, and related test support

**Interfaces:**
- Consumes: Tasks 1–2’s protocol/auth/config surface with Bedrock consumers removed.
- Produces: model-provider and model-provider-info containing only provider-neutral/configured provider implementations; no AWS SigV4 dependency or `genex-aws-auth` workspace crate.

- [ ] **Step 1: Confirm AWS auth consumer boundary**

  Run `rg -n 'genex-aws-auth|AwsAuth|AwsAccessKeys|AwsRequestToSign|ModelProviderAwsAuthInfo|AwsAuthRefreshConfig' crates Cargo.toml Cargo.lock`. Classify every remaining match as Bedrock-only or unrelated and record the result in the compatibility inventory before deletion.

- [ ] **Step 2: Remove Bedrock provider implementation and metadata**

  Delete the Bedrock provider module/tests, remove built-in Bedrock registry entries, constants, AWS config structs/fields, validation, built-in override behavior, exports, and shared credential export/auth recovery state. Keep custom provider registration and generic model retrieval intact.

- [ ] **Step 3: Remove the AWS signing crate and dependencies**

  Delete `crates/aws-auth`, remove workspace member/dependency declarations and direct consumers, then update `Cargo.lock` through the normal Cargo/check flow. Confirm no AWS SDK/SigV4 package remains solely because of Bedrock.

- [ ] **Step 4: Compile affected targets**

  Run `just check --tests`.

  Expected: exit 0; model-provider, model-provider-info, core integration tests, and the remaining workspace test targets compile without Bedrock/AWS-auth code.

- [ ] **Step 5: Commit**

  ```bash
  git add Cargo.toml Cargo.lock crates/model-provider crates/model-provider-info crates/aws-auth crates/app-server crates/core crates/config
  git commit -m "refactor: remove Bedrock provider and AWS auth"
  ```

### Task 4: Update audit records, generated artifacts, and perform final verification

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`, `docs/provider-decoupling-candidates.md`, `docs/provider-decoupling-plan.md` where the active plan needs completion status
- Modify/delete: `chatgpt-openai-line-counts.tsv`, `chatgpt-openai-line-counts-needs-review.tsv`, `hawk.json`, `docs/hawk-dead-public-audit.tsv` only where deleted Bedrock paths or generated counts require updates
- Modify: any remaining active README/config documentation identified by the residual audit

**Interfaces:**
- Consumes: Tasks 1–3’s removed producers and consumers, plus generated schema changes.
- Produces: explicit audit trail, no active Bedrock support surface, and verified provider-neutral workspace.

- [ ] **Step 1: Record producer/consumer decisions**

  Add a dated inventory entry covering protocol/schema, login/auth, config, runtime/UI, model-provider, and `genex-aws-auth` removal. Update candidate/plan status without deleting the audit evidence needed to explain the decision.

- [ ] **Step 2: Regenerate and inspect artifacts**

  Run `just generate`, inspect all generated diffs, and update only artifacts whose deleted producers require changes. Run `git diff --check`.

- [ ] **Step 3: Run exact residual and retain-only audits**

  Run searches for `bedrock`, `amazon_bedrock`, `AmazonBedrock`, `genex-aws-auth`, `AwsAuth`, and Bedrock config keys across active Rust/config/schema/README/test paths. Separately verify retained OpenAI endpoint/wire/auth/model identifiers are unchanged and that any remaining Bedrock wording is limited to removal/audit history.

- [ ] **Step 4: Run repository verification**

  Run `just check --tests`, then `just check`. Read the complete results and resolve any failures before completion.

- [ ] **Step 5: Commit**

  ```bash
  git add docs chatgpt-openai-line-counts.tsv chatgpt-openai-line-counts-needs-review.tsv hawk.json
  git commit -m "docs: record Bedrock support removal"
  ```
