# Chat Completion wire adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Introduce a runtime Chat Completion adapter trait, route built-in `my-provider` through its adapter, and propagate Chat SSE Usage into the existing Core usage path while leaving Responses concrete.

**Architecture:** Keep `WireApi` as serialized provider metadata, but resolve it to a runtime `ProviderApi` descriptor in `ModelProvider`. Define an object-safe `ChatCompletionApi` in `codex-api`; keep transport, authentication, retry, and the Responses implementation unchanged. Reuse the common Chat SSE parser with an injectable Usage decoder so `my-provider` can customize Usage handling without owning HTTP transport.

**Tech Stack:** Rust workspace, `Arc<dyn Trait>`, `serde_json::Value`, SSE streams, existing `ResponseEvent`/`TokenUsage` protocol types, repository `just` checks.

**Spec:** `docs/superpowers/specs/2026-09-28-chat-completion-adapter-design.md`

## Global Constraints

- Work directly on the existing `main` branch; do not create a Git worktree.
- Keep shared architecture provider-neutral; the custom adapter is selected only for built-in `my-provider` in this change.
- Preserve the existing concrete Responses HTTP/WebSocket paths.
- Always run `just check` after production changes.
- Run `just check --tests` whenever Rust code is added, removed, moved, or refactored.
- Run `just generate` only when checked-in generated artifacts may be affected, then inspect the diff.
- Do not run standalone test commands as final verification for this cleanup-oriented repository task.

## Review Focus

- Usage-only Chat chunks with `choices: []` must update Usage instead of being discarded; test in Task 2.
- Multiple Usage snapshots must use the last valid snapshot and ignore malformed snapshots; test in Task 2.
- `stop`, `tool_calls`, `[DONE]`, and EOF completion paths must carry Usage consistently; test in Task 2.
- Missing or malformed Usage must not interrupt text/tool streaming; test in Task 2.
- Responses HTTP/WebSocket dispatch and existing Chat request payloads must remain unchanged; tests in Tasks 1 and 4.

### Task 1: Define the Chat adapter interface and standard implementation

**Files:**
- Create: `crates/codex-api/src/chat_completion.rs`
- Modify: `crates/codex-api/src/lib.rs`
- Modify: `crates/codex-api/src/requests/chat.rs`
- Modify: `crates/codex-api/src/endpoint/chat.rs`
- Test: `crates/codex-api/src/chat_completion.rs` and existing request tests

**Interfaces:**
- Produce `ChatRequestInput<'a>` containing the model, instructions, input items, tools, provider metadata, conversation ID, and session source needed by the existing builder.
- Produce object-safe `ChatCompletionApi: Send + Sync` with synchronous `build_request(...) -> Result<ChatRequest, ApiError>` and `spawn_stream(...) -> ResponseStream` methods.
- Produce `StandardChatCompletionApi`, which delegates request construction to the existing Chat request conversion and stream construction to the standard Chat SSE adapter.

- [ ] **Step 1: Write the failing adapter contract test**

  Add a unit test that constructs `StandardChatCompletionApi`, supplies a minimal `ChatRequestInput`, and asserts that the returned request preserves the existing `model`, `messages`, `stream`, and `tools` payload shape. Add a stream construction test fixture only if the existing endpoint test helpers can exercise it without a mock-only assertion.

- [ ] **Step 2: Run the focused test to verify it fails**

  Run: `cargo test -p codex-api chat_completion --lib`

  Expected: FAIL because `ChatRequestInput`, `ChatCompletionApi`, and `StandardChatCompletionApi` do not exist yet.

- [ ] **Step 3: Implement the adapter boundary**

  Add the new public types and standard implementation. Keep the trait free of generic transport types and async methods. Update `ChatClient<T>` to receive `Arc<dyn ChatCompletionApi>` and invoke `spawn_stream` after `EndpointSession` returns the HTTP stream. Keep `ChatRequestBuilder` behavior unchanged; move only the construction call behind `StandardChatCompletionApi` and update exports.

- [ ] **Step 4: Run the focused test to verify it passes**

  Run: `cargo test -p codex-api chat_completion --lib`

  Expected: PASS, with the existing request-builder tests still compiling.

- [ ] **Step 5: Run the package type/test-target checks**

  Run: `cargo check -p codex-api` and `cargo check -p codex-api --tests`

  Expected: PASS.

### Task 2: Add Usage decoding and terminal-event propagation to Chat SSE

**Files:**
- Modify: `crates/codex-api/src/chat_completion.rs`
- Modify: `crates/codex-api/src/sse/chat.rs`
- Modify: `crates/codex-api/src/lib.rs`
- Test: `crates/codex-api/src/sse/chat.rs`

**Interfaces:**
- Produce `ChatUsageDecoder: Send + Sync` with a method that maps one top-level Chat `usage` JSON value to `Option<TokenUsage>`.
- Produce the standard decoder for `prompt_tokens`, `completion_tokens`, `total_tokens`, cached input details, and reasoning output details.
- Extend `spawn_chat_stream`/`process_chat_sse` to accept the decoder, retain the latest valid Usage snapshot, and attach it to every terminal `ResponseEvent::Completed`.

- [ ] **Step 1: Write failing SSE Usage tests**

  Add tests covering:

  - a normal text chunk followed by a `choices: []` Usage-only chunk and `[DONE]`, asserting `Completed.token_usage` contains the mapped counts;
  - multiple Usage snapshots where the last valid snapshot wins and an invalid snapshot is ignored;
  - `finish_reason = "stop"` and `finish_reason = "tool_calls"` carrying Usage;
  - missing Usage preserving normal completion with `None`.

- [ ] **Step 2: Run the focused tests to verify they fail**

  Run: `cargo test -p codex-api sse::chat --lib`

  Expected: FAIL because Usage is currently ignored and terminal events contain `None`.

- [ ] **Step 3: Implement the decoder and parser state**

  Parse top-level `usage` before requiring a non-empty `choices` array. Store only the latest valid decoded snapshot. Thread the stored value through `stop`, `tool_calls`, sentinel, and EOF completion helpers. Do not turn malformed Usage into a stream error and do not estimate missing values.

- [ ] **Step 4: Run the focused tests to verify they pass**

  Run: `cargo test -p codex-api sse::chat --lib`

  Expected: PASS, including the existing Chat tool/text tests.

- [ ] **Step 5: Run package checks**

  Run: `cargo check -p codex-api` and `cargo check -p codex-api --tests`

  Expected: PASS.

### Task 3: Resolve runtime provider APIs and select the built-in adapter

**Files:**
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/my_provider.rs`
- Modify: `crates/model-provider/src/lib.rs`
- Modify: `crates/model-provider-info/src/lib.rs`
- Test: `crates/model-provider/src/provider.rs` and `crates/model-provider-info/src/model_provider_info_tests.rs`

**Interfaces:**
- Produce `ProviderApi::{Responses, Chat(Arc<dyn ChatCompletionApi>)}` in `codex-model-provider`.
- Produce `ModelProvider::api() -> ProviderApi`, with a default mapping from `ModelProviderInfo.wire_api` to `Responses` or `StandardChatCompletionApi`.
- Make `MyModelProvider::api()` return `ProviderApi::Chat` with the built-in My Provider adapter.
- Set `ModelProviderInfo::create_my_provider()` to `wire_api: WireApi::Chat` so the built-in catalog advertises the selected protocol.

- [ ] **Step 1: Write failing provider-selection tests**

  Add assertions that the built-in `my-provider` metadata has `WireApi::Chat`, that `create_model_provider` returns the My Provider runtime API descriptor, and that an ordinary Responses provider still resolves to `ProviderApi::Responses`.

- [ ] **Step 2: Run the focused tests to verify they fail**

  Run: `cargo test -p codex-model-provider-info model_provider_info_tests --lib` and `cargo test -p codex-model-provider provider --lib`

  Expected: FAIL because the built-in metadata remains `Responses` and `ProviderApi`/`ModelProvider::api` do not exist.

- [ ] **Step 3: Implement provider runtime selection**

  Add the runtime descriptor and default mapping. Keep `WireApi` serialization unchanged. Add the My Provider adapter selection in `my_provider.rs`; do not move authentication behavior or add a general external adapter registry.

- [ ] **Step 4: Run the focused tests to verify they pass**

  Run: `cargo test -p codex-model-provider-info model_provider_info_tests --lib` and `cargo test -p codex-model-provider provider --lib`

  Expected: PASS.

- [ ] **Step 5: Run package checks**

  Run: `cargo check -p codex-model-provider` and `cargo check -p codex-model-provider --tests`

  Expected: PASS.

### Task 4: Dispatch Core through `ProviderApi` and preserve Responses

**Files:**
- Modify: `crates/core/src/client.rs`
- Modify: `crates/codex-api/src/endpoint/chat.rs`
- Test: `crates/core/tests/suite/client.rs` or the nearest existing Chat/Core integration test location

**Interfaces:**
- Consume `ModelProvider::api()` from Task 3.
- Build Chat requests through `ChatCompletionApi::build_request` and construct `ChatClient` with the resolved adapter.
- Keep the `ProviderApi::Responses` branch on the existing HTTP/WebSocket methods.

- [ ] **Step 1: Write the failing dispatch/propagation test**

  Add a Core-facing fixture that selects the built-in My Provider, returns Chat SSE text plus Usage, and asserts that the completed response records the expected `TokenUsage`. Add an assertion that an existing Responses provider still chooses the Responses request path.

- [ ] **Step 2: Run the focused test to verify it fails**

  Run: `cargo test -p codex-core --test suite client --no-run`

  Expected: FAIL to compile because Core still dispatches directly on `WireApi` and does not pass an adapter.

- [ ] **Step 3: Implement Core dispatch**

  Replace the direct `WireApi` match in `ModelClient::stream` with `ProviderApi` dispatch. In the Chat branch, create `ChatRequestInput`, call the adapter's `build_request`, and pass the same adapter into `ApiChatClient`. Preserve the existing auth recovery loop, telemetry, retry behavior, output-schema rejection, and Responses WebSocket fallback.

- [ ] **Step 4: Run the focused compile/test-target check**

  Run: `cargo check -p codex-core` and `cargo check -p codex-core --tests`

  Expected: PASS.

- [ ] **Step 5: Run the focused integration test**

  Run: the existing Core client suite command selected by the repository's test harness for the added case.

  Expected: PASS with Chat Usage reaching the existing session aggregation path and Responses behavior unchanged.

### Task 5: Refactor, verify the workspace, and inspect generated artifacts

**Files:**
- Modify: only files needed to remove duplication or fix lint/type issues found in Tasks 1-4.
- Test: all affected package test targets through the repository checks.

- [ ] **Step 1: Review the complete diff**

  Run: `git diff --check`, `git diff --stat`, and inspect all modified files for provider-neutral naming, accidental public API expansion, and unchanged Responses behavior.

- [ ] **Step 2: Run required repository verification**

  Run: `just check`.

  Expected: PASS.

- [ ] **Step 3: Compile test targets without executing standalone tests**

  Run: `just check --tests`.

  Expected: PASS.

- [ ] **Step 4: Generate only if applicable**

  If checked-in generated artifacts or protocol schemas changed, run `just generate` and inspect the resulting diff. Otherwise record that no generated artifacts were affected.

- [ ] **Step 5: Commit the implementation**

  ```bash
  git add crates/codex-api crates/model-provider crates/model-provider-info crates/core
  git commit -m "feat: add chat completion provider adapter"
  ```
