# Remove ChatGPT-Hosted Surfaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove or provider-neutralize the remaining ChatGPT-only update, desktop/security detection, and usage/billing surfaces without removing active provider-neutral capabilities or security guarantees.

**Architecture:** Treat each hosted surface as an independent cleanup slice. Audit its producers and consumers first, record `remove`, `retain`, or `migrate-in-place` decisions in the compatibility inventory, then remove only the ChatGPT-specific boundary and preserve generic update, platform security, error, and usage primitives where they have current consumers.

**Tech Stack:** Rust workspace, app-server protocol, TUI, CLI/doctor, `rg`, `just check`, `just check --tests`, and `just generate` for affected generated artifacts.

**Spec:** `docs/provider-decoupling-candidates.md`, `docs/pre-release-compatibility-inventory.md`, and the ChatGPT residual audit recorded in this plan.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Audit every production match before changing code and record an explicit inventory decision.
- Preserve provider-neutral model providers, configured update mechanisms, platform security checks, and generic error/usage handling.
- Do not add migrations for obsolete ChatGPT-only persisted data unless a current reader or writer requires one.
- Always run `just check`; run `just check --tests` for Rust changes; run `just generate` when checked-in generated artifacts may change.
- Do not run standalone tests unless explicitly requested.

## Review Focus

- A configured non-ChatGPT provider must not inherit a ChatGPT URL or billing message; cover the default and explicit-provider paths in the owning config/error tests.
- macOS and Windows security checks must retain the current trusted-process and sandbox behavior after ChatGPT process names are removed; compile the platform-specific targets.
- Existing update commands must fail clearly or use a configured provider-neutral update source after the hosted installer URLs are removed; cover both shell and PowerShell command generation.
- Existing rollout/session records containing `chatgpt` must remain readable if the audit classifies the source as persisted compatibility; cover the retained reader or explicitly remove the obsolete branch.
- Usage-limit errors from non-ChatGPT providers must remain actionable without Plus/Pro/credits URLs; cover provider-neutral error rendering.

---

## Current Residual Classification

The `.rs` search found `chatgpt` in 107 files and 496 matching lines. These
matches are not all removable product dependencies; they are grouped below so
the implementation tasks can distinguish active capability from stale hosted
surface.

| Classification | Main paths | Initial disposition |
|---|---|---|
| Authentication and account continuity | `crates/login`, `crates/model-provider`, `crates/core`, `crates/app-server` | Retain active security/account checks; migrate the adapter boundary only after an equivalent provider capability exists |
| Fixed ChatGPT endpoints | `crates/config`, `crates/model-provider/src/models_endpoint.rs`, `crates/response-debug-context`, `crates/network-proxy` | Remove implicit hosted defaults; migrate to explicit provider-owned/configured endpoints |
| ChatGPT-specific updates | `crates/cli/src/doctor/updates.rs`, `crates/app-server-daemon/src/update_loop.rs`, `crates/tui/src/update_action.rs` | Remove hosted installer/appcast URLs; retain generic update orchestration where reachable |
| Desktop/security detection | `crates/cli/src/doctor/desktop/*` | Retain trust and sandbox predicates; remove or migrate ChatGPT-only process/product identifiers |
| Cyber and trusted-access behavior | `crates/core/src/session/mod.rs`, `crates/protocol/src/turn_input.rs`, `crates/tui/src/chatwidget.rs` | Audit separately; remove only hosted authorization branches without weakening security checks |
| Usage and billing presentation | `crates/protocol/src/error.rs`, `crates/tui/src/history_cell/notices.rs` | Remove Plus/Pro/credits copy and fixed URLs; retain generic rate-limit/error rendering |
| Session and history compatibility | `crates/protocol/src/protocol.rs`, `crates/rollout`, `crates/app-server/src/request_processors/thread_resume_redaction.rs` | Audit persisted readers and client-name filters before deciding remove/retain |
| ChatGPT-specific response metadata | `crates/codex-api/src/sse/responses.rs` | Remove if no active consumer remains; retain generic response diagnostics |
| Test fixtures and helpers | `crates/**/tests`, `*_tests.rs` | Remove obsolete hosted fixtures; retain tests covering generic auth, MCP, security, and provider behavior |
| Comments, examples, and documentation strings | scattered comments and user-facing text | Clean up after runtime decisions; do not classify as runtime dependency by name alone |

The classification is an investigation map, not a deletion decision. Every
production row must be confirmed in
`docs/pre-release-compatibility-inventory.md` before implementation.

---

### Task 1: Remove ChatGPT-specific update and installer surfaces

**Files:**
- Modify: `crates/cli/src/doctor/updates.rs`
- Modify: `crates/app-server-daemon/src/update_loop.rs`
- Modify: `crates/tui/src/update_action.rs`
- Modify: update-related config/protocol/schema files discovered by the producer/consumer audit
- Test: existing update/doctor/TUI test modules adjacent to the modified files
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: current update checks, platform detection, and any configured update source.
- Produces: provider-neutral update behavior or an explicit unsupported/configuration error; no hard-coded `chatgpt.com` installer or appcast URL.

- [x] **Step 1: Audit update producers and consumers**

  Run:

  ```bash
  rg -n -i 'chatgpt\.com|ChatGPT\.app|codex/install|appcast|update' \
    crates/cli/src/doctor/updates.rs \
    crates/app-server-daemon/src/update_loop.rs \
    crates/tui/src/update_action.rs \
    crates --glob '*.rs' --glob '*.json' --glob '*.toml'
  ```

  Classify each match as active update behavior, platform detection, test fixture, or documentation. Record the producer, consumer, and decision in the inventory before code changes.

- [x] **Step 2: Define the replacement boundary**

  Keep generic version/update orchestration if it has current consumers. Remove fixed ChatGPT installer/appcast URLs and either consume an explicit configured update source or return a provider-neutral unsupported result. Keep shell and PowerShell command formatting only when backed by a non-hosted source.

- [x] **Step 3: Remove the hosted branches and stale fixtures**

  Delete only the ChatGPT-specific constants, URL checks, and unreachable test fixtures. Update adjacent tests to assert the configured/provider-neutral path and the clear unsupported result.

- [x] **Step 4: Verify the update slice**

  Run:

  ```bash
  rg -n -i 'chatgpt\.com/codex/install|chatgpt\.com/backend-api/wham/app/appcast|ChatGPT\.app' \
    crates/cli crates/app-server-daemon crates/tui --glob '*.rs'
  just check --tests
  just check
  ```

  Expected: no production update references to the removed hosted URLs; type-check and test-target compilation succeed.

---

### Task 2: Remove ChatGPT-specific security and desktop detection

**Files:**
- Modify: `crates/cli/src/doctor/desktop/macos_security.rs`
- Modify: `crates/cli/src/doctor/desktop/windows_security.rs`
- Modify: `crates/cli/src/doctor/desktop/platform.rs`
- Modify: `crates/cli/src/doctor/network.rs` if its ChatGPT-only default is part of the same reachability path
- Test: adjacent desktop/security/network test modules
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: current macOS/Windows trust checks, process/path inspection, network diagnostics, and Codex desktop support.
- Produces: provider-neutral platform/security diagnostics with no ChatGPT-specific process or URL assumptions.

- [x] **Step 1: Separate security behavior from product-name detection**

  Audit `chatgpt`, `ChatGPT.app`, `chatgpt.exe`, and `electron.exe` matches. For each one, identify whether it protects a process, identifies a Codex Desktop installation, or only improves diagnostics. Record `retain` for generic security behavior and `remove`/`migrate-in-place` for ChatGPT-only identification.

- [x] **Step 2: Replace or remove hosted identifiers**

  Preserve trusted-process validation, sandbox checks, and platform-specific security policy. Remove ChatGPT-only process names and detection branches unless a current Codex Desktop integration requires a provider-neutral executable identity. Do not weaken the security predicate while removing labels.

- [x] **Step 3: Update platform-specific tests**

  Add or revise tests for trusted/untrusted process handling, missing application bundles, and non-matching executables. Tests must prove that removing ChatGPT names does not broaden trust or bypass security checks.

- [x] **Step 4: Verify the platform slice**

  Run:

  ```bash
  rg -n -i 'chatgpt|ChatGPT\.app|chatgpt\.exe' crates/cli/src/doctor --glob '*.rs'
  just check --tests
  just check
  ```

  Expected: only explicitly retained provider-neutral or documented compatibility matches remain; platform/security code compiles for the supported targets.

---

### Task 3: Remove ChatGPT-specific usage and billing error presentation

**Files:**
- Modify: `crates/protocol/src/error.rs`
- Modify: `crates/tui/src/history_cell/notices.rs`
- Modify: related app-server/TUI protocol and snapshot files discovered by the audit
- Test: protocol error and TUI notice tests/snapshots
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: provider/model errors, rate-limit information, and current TUI error/notice rendering.
- Produces: provider-neutral rate-limit/quota errors; no ChatGPT plan, credits, promo, reset-time, or upgrade metadata is attached to the shared error type.

- [x] **Step 1: Audit billing and usage producers**

  Run:

  ```bash
  rg -n -i 'chatgpt\.com/(explore|codex/settings/usage)|Plus|Pro|credits|usage limit|purchase more' \
    crates/protocol crates/tui crates/app-server --glob '*.rs' --glob '*.json' --glob '*.ts'
  ```

  Trace each error from its producer to protocol serialization and TUI rendering. Record which fields are provider-neutral and which are ChatGPT-only.

- [x] **Step 2: Define provider-neutral error semantics**

  Keep generic rate-limit and quota classification, retry behavior, and the separate account usage snapshot/status surfaces. Remove the provider-specific `UsageLimitReachedError` payload, plan/credits/promo/reset-time formatting, and `usage_not_included` error variant. Provider-specific adapters may map their responses to generic errors at the boundary.

- [x] **Step 3: Remove hosted messages and update snapshots**

  Change protocol/API output to the provider-neutral form and remove tests for the deleted hosted error payload. Keep account usage snapshot coverage separate from request-failure rendering.

- [x] **Step 4: Verify the error slice**

  Run:

  ```bash
  rg -n -i 'chatgpt\.com/(explore|codex/settings/usage)|upgrade to (Plus|Pro)|purchase more credits' \
    crates/protocol crates/tui crates/app-server --glob '*.rs' --glob '*.json' --glob '*.ts'
  just check --tests
  just check
  ```

  Expected: no production ChatGPT billing copy remains; generic usage-limit behavior and affected test targets compile successfully.

---

### Task 4: Final inventory, generated artifacts, and dependency audit

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: generated protocol/schema artifacts only if `just generate` changes them
- Modify: `docs/provider-decoupling-candidates.md` to record completion and remaining follow-ups

- [x] **Step 1: Record final decisions**

  For every production match touched by Tasks 1–3, record the exact path/symbol, producer/consumer, canonical replacement, decision, and verification command in the ordered inventory section.

- [x] **Step 2: Regenerate and inspect artifacts**

  Run `just generate`, inspect the complete diff, and retain only generated changes caused by the cleanup.

- [x] **Step 3: Run repository verification**

  Run:

  ```bash
  just check --tests
  just check
  git diff --check
  ```

- [x] **Step 4: Confirm residual matches are intentional**

  Re-run the ChatGPT/provider search restricted to Rust production code and classify every remaining match as active security behavior, provider-owned adapter, persisted compatibility, test-only fixture, or documentation. Do not mark the plan complete while an unexplained production match remains.
