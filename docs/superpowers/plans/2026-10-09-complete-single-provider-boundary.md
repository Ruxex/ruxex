# Complete Single-Provider Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完了仕様に従い、Core・拡張・TUI・永続化・文書を単一Provider runtimeのprovider-neutral境界へ移行する。

**Architecture:** Provider adapterがHTTP transport、wire payload、credential、内部API型を所有し、CoreはProvider-neutralなrequest/event/auth snapshotだけを扱う。各AppServerは1つの共有Provider handleを持ち、TUIや拡張はそのstatus/capabilityを投影するだけにする。

**Tech Stack:** Rust workspace、Tokio、app-server protocol、SQLite migrations、`rg`、repository `just` recipes。

**Spec:** `docs/superpowers/specs/2026-10-09-complete-single-provider-boundary-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- `ModelProvider`の公開request/response/stream/auth型に`HttpClientFactory`、`HeaderMap`、`CredentialManager`、`GenexAuth`、`genex_api::Provider`、`SharedAuthProvider`を含めない。
- 外部OpenAI endpointのURL、request/response fields、authentication headers/variables、model identifiersはProvider adapter内で変更しない。
- 1つのAppServer/runtimeにProviderを1つだけ注入し、同一runtimeの全clientでauth stateを共有する。
- logout後の新規turnは拒否し、in-flight turnは開始時snapshotを使い続ける。
- Rust変更後はユーザー指定のmemory-limited systemd scopeで`just check`と`just check --tests`を実行する。standalone testコマンドは実行しない。
- protocol/schema生成物に影響する場合だけ`just generate`を実行し、生成差分を確認する。

## Review Focus

- Provider adapterがChat、WebSocket、検索、backend、model catalogの各経路で同じruntime transport/auth stateを使うこと。各経路のcompile-only regression coverageをTask 1〜3に置く。
- auth変更がsetup中・stream中・次turn開始時に競合した場合、既存turnのsnapshotを差し替えず、次turnだけが新stateを読むこと。Task 3にrevision/generation coverageを置く。
- Provider IDが未知・変更・空文字でもTUIが文字列比較で機能を切り替えないこと。Task 5にprovider-neutral capability/status coverageを置く。
- 旧session logやthread historyにProvider routing情報が残っていても、現行readerがrouting selectorとして再利用しないこと。Task 6にreader/migration auditを置く。
- OpenAI互換外部契約がadapter移行で変わらないこと。Task 2にwire payload/header/model identifier assertionsを置く。

---

### Task 1: Provider contractからHTTP実装型を隔離する

**Files:**
- Modify: `crates/model-provider/src/contract.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/cloudflare_provider.rs`
- Modify: `crates/model-provider/src/model_catalog_source.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Modify: `crates/ext/web-search/src/tool.rs`
- Modify: `crates/ext/history-notes/src/backend.rs`
- Test: `crates/model-provider/src/contract.rs` and adjacent provider contract tests

**Interfaces:**
- Produce `ProviderTransportOptions` with only session/thread metadata, generic string headers, compression, and turn state.
- Produce `ProviderWebsocketConnectRequest`, `ProviderSearchRequest`, `ProviderBackendRequest`, and `ListModelsRequest` without HTTP concrete types.
- Keep `ProviderRuntime` as the adapter-internal boundary; `RuntimeProviderContractAdapter` converts neutral requests to internal HTTP/wire requests and owns the configured `HttpClientFactory`.

- [ ] **Step 1: Write failing compile-only contract assertions**

  Add tests/types that construct every public request with generic headers and no HTTP factory, and assert the adapter still forwards the existing request payload, headers, compression, and model-catalog identity.

- [ ] **Step 2: Run the affected compile-only target and observe the expected failure**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=4 just check --tests -p genex-model-provider`.
  Expected: the new request construction fails until the contract and adapter are migrated.

- [ ] **Step 3: Replace public HTTP fields with neutral metadata**

  Change request structs to use `BTreeMap<String, String>` or equivalent provider-neutral header metadata. Remove `HttpClientFactory` from `ChatTransportOptions`, `ProviderSearchRequest`, `ProviderBackendRequest`, `ProviderWebsocketConnectRequest`, and `ListModelsRequest`.

- [ ] **Step 4: Move transport ownership into the adapter**

  Add the adapter-internal transport context and route all request conversion through it. Preserve the current OpenAI/Cloudflare URL, auth header, compression, and wire payload construction inside `provider.rs`/`cloudflare_provider.rs`.

- [ ] **Step 5: Migrate extension producers**

  Convert Web Search and History Notes request construction to the neutral metadata types. No extension may import HTTP header or client factory types solely to satisfy the Provider contract.

- [ ] **Step 6: Run the compile-only target and commit**

  Run the same `just check --tests -p genex-model-provider` command and expect success, then commit with `refactor: hide provider transport types behind adapter`.

### Task 2: Provider-neutral stream, auth snapshot, and recovery contract

**Files:**
- Modify: `crates/model-provider/src/contract.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/cloudflare_provider.rs`
- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Test: provider contract lifecycle and stream adapter tests

**Interfaces:**
- Produce `ProviderChatEvent` as a provider-neutral enum and convert wire/API events only inside the adapter.
- Produce `ProviderRequestAuth`/`ProviderRequestSnapshot` containing only auth telemetry, managed-auth capability, revision, owner generation, and request admission state; credential values remain private.
- Replace Core-facing compatibility methods (`model_provider_info`, `credential_manager`, `auth`, `api_provider`, `api_auth`, `api_auth_for_scope`, `map_api_error`) with neutral snapshot, connection-key, error, and recovery methods.

- [ ] **Step 1: Add failing lifecycle and event assertions**

  Cover: unauthenticated admission returns `ProviderError::Unauthenticated`; logout increments/invalidates the next-turn auth generation without changing an already-created snapshot; the fake provider emits and the host receives every normalized stream event.

- [ ] **Step 2: Run the provider compile-only target and confirm RED**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=4 just check --tests -p genex-model-provider`.
  Expected: the new neutral API is missing or incompatible with the existing adapter.

- [ ] **Step 3: Implement neutral event and auth snapshot types**

  Mirror the repository response-event vocabulary using protocol-owned value types, add explicit `Unauthenticated`/`Unauthorized` mapping, and expose only snapshot metadata needed by Core telemetry and turn admission.

- [ ] **Step 4: Move credential recovery into the provider adapter**

  Make provider-owned recovery perform its private credential-manager refresh/logout semantics and return the neutral recovery result. Core must not construct `UnauthorizedRecovery`, read credential values, or map API errors from provider internals.

- [ ] **Step 5: Remove the compatibility methods from the public ModelProvider trait**

  Update the adapter and test fake to implement the neutral methods. Keep provider-specific `ProviderRuntime` methods private to `model-provider`.

- [ ] **Step 6: Run provider checks and commit**

  Run `just check --tests -p genex-model-provider`; expect success, then commit with `refactor: make provider auth and stream contract neutral`.

### Task 3: Migrate Core client execution to the neutral contract

**Files:**
- Modify: `crates/core/src/client.rs`
- Modify: `crates/core/src/prompt_debug.rs`
- Modify: `crates/core/src/thread_manager.rs`
- Modify: `crates/core/src/test_support.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Test: `crates/core/tests/suite/prompt_debug_tests.rs`, client/auth regression tests, and compile-only test targets

**Interfaces:**
- `CurrentClientSetup` consumes `ProviderRequestSnapshot` and a provider-owned `ResponsesConnectionKey`, not internal API/auth types.
- `map_response_events` consumes `ProviderChatStream` and maps neutral events to the Core `ResponseEvent` type at one boundary.
- Core uses `provider.recover_from_unauthorized()` and neutral errors; it does not call `CredentialManager` or `GenexAuth`.

- [ ] **Step 1: Add failing Core compile assertions**

  Update the fake provider/client coverage so a provider implementation with no `ModelProviderInfo`, `CredentialManager`, `GenexAuth`, `genex_api::Provider`, or `SharedAuthProvider` access can drive chat setup, model listing, and prompt-debug construction.

- [ ] **Step 2: Run the Core compile-only targets and confirm RED**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=4 just check --tests -p genex-core`.
  Expected: existing Core calls to compatibility methods fail to compile.

- [ ] **Step 3: Refactor setup and WebSocket reuse**

  Use the provider snapshot for auth revision/generation/telemetry and the provider-owned connection key. Remove Core-side provider URL construction and API-provider filtering; keep endpoint paths as neutral telemetry labels only.

- [ ] **Step 4: Refactor HTTP and WebSocket stream paths**

  Pass neutral transport metadata to `ChatRequest`/WebSocket requests, map neutral provider events to Core events, and preserve existing fallback, continuation, timing, tracing, and OpenAI request/response behavior.

- [ ] **Step 5: Refactor unauthorized recovery and prompt-debug/model catalog consumers**

  Route recovery through the injected provider; make prompt-debug and catalog operations consume the same runtime handle without constructing a provider in Core production code. Test-only provider construction remains explicitly classified as retained.

- [ ] **Step 6: Run Core compile-only checks and commit**

  Run `just check --tests -p genex-core`; expect success, then commit with `refactor: remove core provider compatibility dependencies`.

### Task 4: Verify shared AppServer authentication lifecycle

**Files:**
- Modify: `crates/app-server/src/runtime.rs`
- Modify: `crates/app-server/src/in_process.rs`
- Modify: `crates/app-server/src/request_processors/account_processor.rs`
- Modify: `crates/core/src/client.rs` if admission hooks require a narrow adjustment
- Test: app-server runtime/account tests and Core lifecycle tests

**Interfaces:**
- Runtime construction continues to inject exactly one shared `SharedModelProvider` and its credential state.
- `login`, credential update, and `logout` mutate the shared provider state; new turn admission observes the current neutral auth snapshot.
- An in-flight `ModelClientSession` retains its start-time snapshot and does not re-read credentials after the turn begins.

- [ ] **Step 1: Add failing lifecycle regression coverage**

  Add tests for two clients sharing one AppServer: logout rejects a newly admitted turn, while a turn that captured credentials before logout continues using its snapshot; login makes subsequent turns admissible.

- [ ] **Step 2: Run affected compile-only targets and confirm RED**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests -p genex-app-server`.
  Expected: the lifecycle test harness fails until admission/snapshot wiring is complete.

- [ ] **Step 3: Implement the smallest admission/snapshot fix**

  Keep the shared runtime/provider ownership unchanged. Reject only new turns after logout, preserve in-flight request state, and keep auth revision notifications/cache invalidation behavior intact.

- [ ] **Step 4: Run app-server compile-only checks and commit**

  Run `just check --tests -p genex-app-server`; expect success, then commit with `test: cover shared provider auth lifecycle`.

### Task 5: Remove TUI Provider-specific feature branches and session-log routing fields

**Files:**
- Modify: `crates/tui/src/daybreak.rs`
- Modify: `crates/tui/src/chatwidget/turn_runtime.rs`
- Modify: `crates/tui/src/chatwidget/settings.rs`
- Modify: `crates/tui/src/chatwidget/rate_limits.rs`
- Modify: `crates/tui/src/chatwidget/thread_usage.rs`
- Modify: `crates/tui/src/session_log.rs`
- Modify: adjacent TUI tests and snapshots

**Interfaces:**
- TUI receives `ProviderCapabilities`, `ProviderAuthState`, and `ProviderUsage` projections; it never enables a feature by comparing `model_provider_id` to a literal.
- Cyber refusal and rate-limit notices use neutral capability/status data. Hosted billing/Genex-specific presentation is removed or recorded as a retained external contract only when a current consumer proves it.
- Session-start logs contain model/session context but no `model_provider_id` or `model_provider_name` routing fields.

- [ ] **Step 1: Add failing TUI tests**

  Assert that unknown and non-default provider IDs have identical behavior when capabilities/status are identical, and assert that a session log header omits provider routing fields.

- [ ] **Step 2: Run the TUI compile-only target and confirm RED**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests -p genex-tui`.
  Expected: the new tests fail against literal Provider ID branches and current session-log fields.

- [ ] **Step 3: Replace literal branches with capability/status checks**

  Remove `== "provider"`/`!= "provider"` feature gates, keep generic security and rate-limit behavior, and use explicit Provider capability/auth/usage state for optional surfaces.

- [ ] **Step 4: Remove session-log routing metadata and update fixtures**

  Stop writing provider selector/name fields; preserve only mounted-provider status where it is an explicitly read-only diagnostic response.

- [ ] **Step 5: Run TUI compile-only checks and commit**

  Run `just check --tests -p genex-tui`; expect success, then commit with `refactor: neutralize tui provider surfaces`.

### Task 6: Audit persisted protocol fields and synchronize migration records

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/superpowers/plans/2026-09-30-remove-chatgpt-hosted-surfaces.md` only for completion/status synchronization when justified
- Modify: existing schema/migration files only if the audit finds an active reader/writer
- Test: migration/schema compile-only targets where applicable

**Interfaces:**
- `model_provider`/`provider_id` retained only in mounted-provider status responses and explicitly classified in the inventory.
- Thread/rollout/history/session-log readers never use provider fields for routing or credential lookup.
- Every changed production match has producer, consumer, and `remove`/`retain`/`migrate-in-place` decision in inventory order.

- [ ] **Step 1: Run the ordered producer/consumer audit**

  Scan persisted rollout/thread-history readers, protocol/app-server aliases, CLI/config/import paths, UI/observability/runtime shims, build scaffolding, and dependencies. Record findings before changing any additional code.

- [ ] **Step 2: Add failing migration/reader assertions where a stale routing read remains**

  Add compile-only or migration assertions proving that the obsolete field is absent from new writes and ignored by current readers; do not add a compatibility layer without a current consumer.

- [ ] **Step 3: Apply only audited decisions**

  Remove stale routing/credential persistence, retain platform fallbacks and external OpenAI identifiers, and mark test-only/provider-adapter construction as retained with evidence.

- [ ] **Step 4: Synchronize plan and candidate statuses**

  Mark completed checklist items and resolve contradictory candidate text only when the corresponding code and verification are complete. Record remaining provider-specific follow-ups separately rather than hiding them in the rename.

- [ ] **Step 5: Run migration/schema checks and commit**

  Run the affected `just check --tests` target and commit with `docs: synchronize single-provider audit decisions`.

### Task 7: Full verification and final boundary audit

**Files:**
- Modify: only files required by verification findings
- Test: repository check targets and generated artifacts if affected

- [ ] **Step 1: Run full production verification**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check`.

- [ ] **Step 2: Compile all Rust test targets**

  Run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests`.

- [ ] **Step 3: Regenerate only when applicable**

  If protocol/schema/generated files changed, run `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just generate` and inspect the complete diff.

- [ ] **Step 4: Run final static audit**

  Verify with `rg` that production Core no longer references Provider-specific compatibility methods, public contract requests contain no HTTP implementation types, TUI has no Provider ID literal feature gate, and thread/rollout/session-log writes contain no routing selector or credentials.

- [ ] **Step 5: Run `git diff --check`, inspect status, and commit any final fixes**

  Expected: all required checks pass, generated changes are intentional, and the working tree is clean except for no uncommitted changes.
