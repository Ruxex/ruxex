# Finish Single-Provider Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完了ゲートに残る公開 adapter、capability protocol、認証 provider ID、互換性文書、Remote TUI 境界を単一 provider 設計に揃える。

**Architecture:** `ModelProvider` と provider-neutral 型を唯一の公開 provider 契約とし、既存の OpenAI 互換実装は host factory の内部 adapter として隠す。認証は mounted provider の flat credential store に限定し、provider identity は data-root marker と内部 descriptor 検証にだけ使う。Remote TUI は provider data root の所有者ではないことをコードと台帳で明示する。

**Tech Stack:** Rust workspace, serde/JSON-RPC schemas, checked-in generated schemas, Markdown compatibility inventory.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`

## Global Constraints

- Host/AppServer は exactly one `ModelProvider` instance を注入する。
- provider switching は runtime protocol に存在せず、別 provider は別 runtime/data root で起動する。
- `genex-home.json` は SQLite、sessions、cache、credentials より先に検証する。
- OpenAI endpoint の URL、wire fields、headers/environment variables、model identifiers は変更しない。
- Rust の検証は user systemd scope 経由の `just` recipe で実行する。
- standalone test execution は行わず、`just check`、`just check --tests`、`just generate` を使用する。

## Review Focus

- 公開 crate consumer が concrete `ProviderRuntime` や `ProviderApi` を直接取得できないこと。
- capability の未対応値が false/unsupported として protocol 上で区別できること。
- flat credential store が provider ID をキーにせず、単一 mounted provider の lifecycle だけを扱うこと。
- mismatched marker の root で credential/database initialization が発生しないこと。
- Remote TUI の local state と remote AppServer の provider-owned state が混同されないこと。

---

### Task 1: Public provider boundary

**Files:**
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Modify: all in-repository consumers of `create_model_provider*` and `provider_contract`
- Test: `crates/model-provider/src/provider.rs` compile-level provider factory assertion

**Interfaces:**
- Produces public `create_model_provider*` functions returning `SharedModelProvider`.
- Keeps `ProviderRuntime`, `ProviderApi`, `SharedProviderRuntime`, and `provider_contract` private to the adapter crate.

- [ ] Write a compile-level test/use site assigning the factory result directly to `SharedModelProvider`.
- [ ] Run `just check --tests` and confirm the old return type/API causes the intended compile failure.
- [ ] Change the factory to return the provider-neutral shared handle and update consumers to remove adapter wrapping.
- [ ] Remove legacy adapter re-exports and keep concrete adapter internals crate-private.
- [ ] Run `just check --tests` and verify all workspace test targets compile.

### Task 2: Complete capability protocol

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/v2/model.rs`
- Modify: `crates/app-server/src/request_processors/config_processor.rs`
- Modify: protocol/schema fixtures if generated
- Test: app-server protocol compile/schema generation checks

**Interfaces:**
- `ModelProviderCapabilitiesReadResponse` mirrors all provider-neutral capability fields, including explicit remote-compaction support.

- [ ] Add a protocol-level construction/use that requires every capability field.
- [ ] Run `just check --tests` and observe the missing-field failure.
- [ ] Add the full response mapping with explicit unsupported values and update generated schemas through `just generate`.
- [ ] Run `just check --tests` and `just generate`; inspect generated diff.

### Task 3: Single-provider credential API

**Files:**
- Modify: `crates/login/src/auth/manager.rs`
- Modify: `crates/core/src/config/auth_keyring.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Test: login manager and model-provider auth compile/test targets

**Interfaces:**
- Flat storage helpers accept only home, storage mode, and credentials; no provider target argument.
- `CredentialManager::{auth,login,delete,logout}` operate on the mounted provider context.
- Provider identity remains only in descriptor/marker validation and does not route credential records.

- [ ] Add/update tests asserting flat auth helpers have no provider-keyed operation and mismatched provider targeting is impossible at the API boundary.
- [ ] Run `just check --tests` and observe failures from the old signatures/callers.
- [ ] Remove provider-targeting arguments and selection-shaped manager methods; update provider adapter calls.
- [ ] Run `just check --tests` and verify auth lifecycle consumers compile.

### Task 4: Documentation and Remote TUI boundary

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `crates/tui/src/lib.rs`
- Modify: relevant TUI/app-server tests or comments

**Interfaces:**
- Compatibility inventory records current `remove`/`retain`/`migrate-in-place` decisions without obsolete provider-switch claims.
- Remote TUI explicitly documents that local state is client-owned and provider-home marker validation belongs to the owning AppServer.

- [ ] Add a focused assertion/comment fixture for the Remote target ownership rule.
- [ ] Run the repository checks to establish the documentation/code expectation.
- [ ] Synchronize stale audit rows and add the Remote ownership explanation without changing the deliberate remote behavior.
- [ ] Run `just check`, `just check --tests`, `just generate`, `git diff --check`, and inspect all changes.

### Task 5: Final audit, commit, and push

- [ ] Re-run exact provider-selection, provider-keyed-auth, and generated-schema searches.
- [ ] Verify marker ordering and thread/rollout provider persistence invariants.
- [ ] Run all required checks with the repository memory scope.
- [ ] Commit the complete change on `main`.
- [ ] Push `main` to `origin` and verify the remote tip.
