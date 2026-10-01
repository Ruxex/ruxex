# Codex-only Product Filtering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the `products` attribute on skills and plugins while making the current runtime recognize Codex as the only supported product and discard non-Codex entries.

**Architecture:** Preserve the existing product-restriction pipeline and metadata fields. Replace the built-in ChatGPT/Atlas product variants with a Codex variant plus an unknown value used when deserializing unsupported product names; only the Codex runtime product can satisfy a restriction. Remove ChatGPT/Atlas session-source mappings and interactive filters while keeping arbitrary custom session sources readable.

**Tech Stack:** Rust, Serde, schemars/TypeScript generation, existing skill/plugin loaders, repository `just` checks.

**Spec:** User-approved in-chat design: retain `products`; load unqualified entries and entries containing `codex`; discard entries restricted to ChatGPT, Atlas, or other non-Codex products; preserve `Product::Codex` as a compatibility exception if the repository name changes.

## Global Constraints

- Keep the `products` metadata attribute on skills and plugins.
- Do not reintroduce ChatGPT or Atlas runtime/product implementations.
- Unknown product names must not become Codex-eligible accidentally.
- `Product::Codex` is a stable compatibility identifier and must remain even if the repository or executable name changes.
- Preserve generic `SessionSource::Custom(String)` decoding for old persisted session records.
- Run `just check --tests` for Rust changes and `just check` for the production workspace; run `just generate` if generated protocol/schema artifacts change.

## Review Focus

- `products: [chatgpt]` and `products: [atlas]` must parse without crashing and be excluded from Codex skill/plugin loading.
- `products: [codex, atlas]` must remain eligible for Codex.
- Missing or empty product restrictions must preserve their existing skill/plugin semantics.
- Old persisted `SessionSource::Custom("chatgpt")` and `SessionSource::Custom("atlas")` values must remain readable but must not be treated as current Codex product sources.
- Product serialization/schema generation must not accidentally advertise ChatGPT or Atlas as supported built-in products.

### Task 1: Normalize the protocol product model

**Files:**
- Modify: `crates/protocol/src/protocol.rs`
- Test: protocol unit tests in `crates/protocol/src/protocol.rs`
- Modify: generated artifacts only if `just generate` changes them

**Interfaces:**
- Produces `Product::Codex` as the sole supported runtime product and an internal `Product::Unknown` representation for unsupported serialized names.
- `Product::from_session_source_name` recognizes only `codex`; ChatGPT, Atlas, and other custom names return `None`.

- [x] Add failing protocol tests covering case-insensitive `codex`, unsupported ChatGPT/Atlas names mapping to `None` for session-source restriction, and unsupported serialized product names becoming `Product::Unknown`.
- [x] Implement the reduced product enum and custom deserialization for unsupported string values. Add a code comment beside `Product::Codex` stating that it must remain as a stable compatibility identifier even if the repository name changes.
- [x] Remove `to_app_platform` and built-in ChatGPT/Atlas mappings if no current consumer remains.
- [x] Update existing protocol tests to assert Codex-only behavior.
- [x] Run `just check --tests` and inspect any generated diff.

### Task 2: Remove obsolete non-Codex interactive session entries

**Files:**
- Modify: `crates/rollout/src/lib.rs`
- Test: `crates/rollout/src/tests.rs`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

**Interfaces:**
- `INTERACTIVE_SESSION_SOURCES` contains only current Codex-compatible interactive sources.
- Generic custom session-source decoding remains unchanged.

- [x] Extend the regression test to assert that both `chatgpt` and `atlas` are absent from `INTERACTIVE_SESSION_SOURCES`.
- [x] Remove the Atlas entry and retain the already-removed ChatGPT entry.
- [x] Record the updated producer/consumer decision in the ordered compatibility inventory and candidate queue.
- [x] Run `just check --tests` and `just check`.

### Task 3: Verify Codex-only skill and plugin filtering

**Files:**
- Test/modify: existing skill loader and plugin manager tests under `crates/ext/skills`, `crates/skills`, and `crates/core-plugins`
- Modify: `crates/core-plugins/src/marketplace.rs` or `crates/skills/src/model.rs` only if current filtering cannot enforce the approved semantics
- Modify: generated artifacts only if affected

**Interfaces:**
- Existing `products` fields remain typed through `Product`.
- Existing filtering continues to allow absent/unrestricted metadata and Codex-compatible metadata while rejecting lists without `Product::Codex`.

- [x] Add or update tests for ChatGPT/Atlas/unknown-only restrictions being excluded and mixed Codex/non-Codex restrictions being accepted.
- [x] Confirm empty/missing restriction behavior remains unchanged for both skill and plugin policies.
- [x] Keep the implementation on the existing filtering path; do not add a second product-policy mechanism.
- [x] Run `just check --tests` and `just check`.

### Task 4: Regenerate, audit, and finalize the cleanup

**Files:**
- Modify: generated protocol/schema artifacts only if produced by `just generate`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

- [x] Run `just generate` when Task 1 changes generated protocol/schema output and inspect the complete diff.
- [x] Search production code and generated artifacts for built-in ChatGPT/Atlas product variants and session-source filters; classify intentional historical/test references.
- [x] Run `just check --tests`, `just check`, and `git diff --check`.
- [x] Confirm the final diff preserves `products` metadata and the `Product::Codex` compatibility comment.
