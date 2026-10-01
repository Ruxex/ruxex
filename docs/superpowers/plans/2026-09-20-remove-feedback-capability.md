# Remove the Feedback Capability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the `codex-feedback` product capability and every current
producer/consumer of its report flow while preserving guardian safety behavior,
ordinary tracing, rollout behavior, and unrelated provider-neutral features.

**Architecture:** Delete the feedback capability at its public boundaries:
configuration, app-server protocol, TUI command, app-server processor, and
crate dependency. Remove the feedback-only tracing and guardian attachment
plumbing, then regenerate schemas and use repository-wide search to classify
remaining names such as hook feedback or Sentry MCP references as unrelated.

**Tech Stack:** Rust workspace, app-server protocol/schema generation, TOML
configuration schema generation, TUI, guardian reviewer, Cargo, `just check`.

**Spec:** `docs/superpowers/specs/2026-09-20-remove-feedback-capability-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- This is a pre-release fork with zero external users; remove the obsolete
  feedback protocol and configuration surfaces without migration code.
- Do not delete existing rollout/config files automatically.
- Preserve guardian review decisions, approvals/denials, permission checks,
  security behavior, ordinary tracing, state/log database consumers, and
  unrelated Sentry MCP configuration.
- Do not run tests for this cleanup; use targeted `cargo check` and `just check`
  as the primary verification.
- Before `just check`, inspect repository-local `target/`; if it exceeds the
  documented 120 GB threshold, remove only that repository-local directory.
- Regenerate app-server and config schemas from their Rust sources and inspect
  generated diffs for unrelated changes.

## Review Focus

- `HookOutputEntryKind::Feedback` is a current hook protocol value and must be
  retained; it is not the removed user feedback capability. Verify by searching
  the hook protocol and its consumers after the deletion.
- Sentry MCP examples and tests are user-configured MCP integrations and must
  remain; only the hard-coded feedback Sentry transport is removed.
- Guardian review safety behavior must remain unchanged; only the failed-review
  report retention and attachment path is removed.
- App-server generated schemas must remove `feedback/upload` and its payloads
  without removing unrelated account, config, or guardian protocol types.
- Existing config files are not rewritten or deleted; only the active
  `feedback.enabled` reader/schema/requirements path is removed.

---

### Task 1: Remove feedback configuration and protocol contracts

**Files:**
- Modify: `crates/config/src/types.rs`
- Modify: `crates/config/src/config_requirements.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/core/src/config/requirements.rs`
- Modify: `crates/app-server-protocol/src/protocol/common.rs`
- Modify: `crates/app-server-protocol/src/protocol/v2/config.rs`
- Delete: `crates/app-server-protocol/src/protocol/v2/feedback.rs`
- Regenerate: `crates/app-server-protocol/schema/` and config schema output
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Removes `FeedbackConfigToml`, `Config.feedback`, and `Config.feedback_enabled`.
- Removes `ClientRequest::FeedbackUpload`, `FeedbackUploadParams`, and
  `FeedbackUploadResponse`.
- Retains `HookOutputEntryKind::Feedback` and unrelated feedback-named values.

- [ ] **Step 1: Remove the configuration contract**

  Delete `FeedbackConfigToml` from `crates/config/src/types.rs`. Remove the
  `feedback` field and every merge, projection, origin, requirement, default,
  and managed-requirement branch in `crates/config/src/config_requirements.rs`.
  Remove `feedback_enabled` and its TOML projection from
  `crates/core/src/config/mod.rs`, and remove the corresponding requirement
  warning/projection in `crates/core/src/config/requirements.rs`.

- [ ] **Step 2: Remove the app-server protocol contract**

  Delete the `FeedbackUpload` entry from the request macro in
  `crates/app-server-protocol/src/protocol/common.rs`, remove the `feedback`
  config type from `crates/app-server-protocol/src/protocol/v2/config.rs`, and
  delete `crates/app-server-protocol/src/protocol/v2/feedback.rs`.

- [ ] **Step 3: Regenerate schemas and inspect the diff**

  Run `just write-app-server-schema` and `just write-config-schema`. Confirm
  generated files no longer contain `feedback/upload`,
  `FeedbackUploadParams`, `FeedbackUploadResponse`, or the feedback config
  object, while hook feedback and unrelated Sentry MCP schema entries remain.

- [ ] **Step 4: Record the compatibility decision**

  Add an inventory row marking the exact feedback config and app-server
  protocol symbols as `remove`, with the producer/consumer audit and schema
  generation commands from this task.

### Task 2: Remove app-server feedback processing

**Files:**
- Delete: `crates/app-server/src/request_processors/feedback_processor.rs`
- Delete: `crates/app-server/src/request_processors/feedback_doctor_report.rs`
- Delete: `crates/app-server/src/request_processors/feedback_thread_index.rs`
- Delete: related feedback processor/thread-index test modules
- Modify: `crates/app-server/src/request_processors.rs`
- Modify: `crates/app-server/src/message_processor.rs`
- Modify: `crates/app-server/src/lib.rs`
- Modify: `crates/app-server/src/in_process.rs`
- Modify: `crates/app-server/src/user_verification_test_support.rs`
- Modify: `crates/app-server/tests/suite/v2/feedback.rs`
- Modify: feedback-only portions of `crates/app-server/tests/suite/v2/guardian_v2.rs`
- Modify: `crates/cli/tests/doctor_path_safety.rs`

**Interfaces:**
- Removes `FeedbackRequestProcessor` and its `feedback_upload` dispatch.
- Removes feedback-only app-server state, attachments, Sentry upload, and
  doctor-report plumbing.
- Keeps ordinary app-server request processing, guardian requests, doctor
  command behavior, and log database setup.

- [ ] **Step 1: Remove request processor registration and ownership**

  Remove the `FeedbackRequestProcessor` module/import/field/constructor
  argument from app-server request processor registration and message
  processing. Remove `CodexFeedback`, feedback-only auth/thread/config fields,
  and any feedback-specific semaphore or state from the app-server runtime and
  test support constructors.

- [ ] **Step 2: Delete feedback-only processor modules**

  Delete the feedback processor, doctor report helper, thread index helper,
  and their dedicated test modules. Remove imports that existed only to build
  Sentry feedback attachments, including connector cache attachments,
  guardian failure attachments, and rollout attachment paths.

- [ ] **Step 3: Remove integration tests and fixtures**

  Delete the app-server feedback suite. Remove only feedback-upload portions
  of guardian and doctor path-safety suites; retain tests for guardian
  authorization, ordinary doctor output, path validation, and unrelated
  protocol requests.

- [ ] **Step 4: Check the app-server boundary**

  Run `cargo check -p codex-app-server -p codex-app-server-protocol -p codex-app-server-client`.
  Confirm no app-server production reference remains for `feedback/upload` or
  `FeedbackUpload`.

### Task 3: Remove the TUI feedback surface and startup capture

**Files:**
- Delete: `crates/tui/src/bottom_pane/feedback_view.rs`
- Delete: `crates/tui/src/bottom_pane/feedback_note_view.rs` when it is used
  only by the feedback command
- Modify: `crates/tui/src/bottom_pane/mod.rs` and feedback UI modules
- Modify: `crates/tui/src/chatwidget/slash_dispatch.rs`
- Modify: `crates/tui/src/app/background_requests.rs`
- Modify: `crates/tui/src/chatwidget.rs`
- Modify: `crates/tui/src/app.rs` and app state/test support
- Modify: TUI startup/constructor files under `crates/tui/src/`
- Modify: TUI fixtures and tests that pass `CodexFeedback`

**Interfaces:**
- Removes the `/feedback` slash command, selection/consent views, upload
  request helper, and feedback state fields.
- Removes feedback logger/metadata layer construction from TUI startup.
- Retains ordinary slash commands, app-server requests, startup logging,
  model interaction, and unrelated `Feedback` hook output rendering.

- [ ] **Step 1: Remove the user-facing command and views**

  Delete the feedback command branch in `slash_dispatch.rs`, remove feedback
  selection and note view modules from bottom-pane registration, and delete
  their associated UI state/actions. Remove any help/completion text that
  advertises `/feedback`.

- [ ] **Step 2: Remove the upload request path**

  Delete feedback parameter construction and `fetch_feedback_upload` from
  `crates/tui/src/app/background_requests.rs`. Remove `ClientRequest::FeedbackUpload`
  call sites and the response handling that displays upload success/failure.

- [ ] **Step 3: Remove feedback state and startup layers**

  Remove `CodexFeedback` fields and constructor parameters from TUI app,
  chatwidget, startup orchestration, worktree startup, onboarding, session
  archive, and test support. Remove installation of `logger_layer()` and
  `metadata_layer()` while retaining any ordinary tracing layers.

- [ ] **Step 4: Check the TUI boundary**

  Run `cargo check -p codex-tui`. Confirm the TUI no longer contains a
  `/feedback` command or `CodexFeedback` production reference.

### Task 4: Remove feedback-only telemetry and dependencies

**Files:**
- Delete: `crates/feedback/`
- Modify: root `Cargo.toml` and `Cargo.lock`
- Modify: `crates/app-server/Cargo.toml`
- Modify: `crates/app-server-client/Cargo.toml`
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/exec/Cargo.toml`
- Modify: `crates/model-provider/Cargo.toml`
- Modify: `crates/tui/Cargo.toml`
- Modify: `crates/core/src/client.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`
- Modify: `crates/core/src/util.rs` and feedback-only utility tests
- Modify: `crates/core/src/session/turn_tests.rs`

**Interfaces:**
- Removes `codex-feedback`, its Sentry/http upload dependencies, and all
  feedback-specific public types.
- Removes `FeedbackRequestTags` and feedback auth/request telemetry emission.
- Retains ordinary model request telemetry and non-feedback HTTP diagnostics.

- [ ] **Step 1: Remove feedback request telemetry**

  Delete `FeedbackRequestTags`, snapshot/tag emitters, and auth-environment
  feedback fields from `crates/feedback/src/lib.rs` by deleting the crate with
  the rest of its implementation. Remove their call sites and imports from
  `crates/core/src/client.rs`, `crates/model-provider/src/models_endpoint.rs`,
  `crates/core/src/util.rs`, and related tests. Preserve request IDs, status,
  auth recovery behavior, and ordinary tracing where those are still consumed.

- [ ] **Step 2: Remove feedback crate dependencies**

  Remove the workspace member and `codex-feedback` dependency declarations
  from every manifest found by the producer/consumer search. Delete the crate
  directory, including report upload, diagnostics, guardian storage, and
  feedback-only tests. Update `Cargo.lock` through Cargo's normal metadata
  resolution rather than hand-editing unrelated lock entries.

- [ ] **Step 3: Remove feedback-only tests and fixtures**

  Remove the test-only subscriber setup that exists only to assert feedback
  capture. Keep tests that cover model request behavior, guardian enforcement,
  state logging, and generic tracing.

- [ ] **Step 4: Check dependency removal**

  Run `cargo check -p codex-core -p codex-model-provider -p codex-exec` and
  inspect `cargo metadata --no-deps` or the manifest diff to confirm no
  workspace crate depends on `codex-feedback`.

### Task 5: Remove guardian failed-review feedback retention

**Files:**
- Delete: `crates/core/src/guardian/feedback.rs`
- Modify: `crates/core/src/guardian/mod.rs`
- Modify: `crates/core/src/guardian/review_session.rs`
- Modify: `crates/core/src/guardian/review_session_setup.rs`
- Modify: guardian tests that assert feedback attachments
- Delete: `crates/ext/guardian-reviewer/src/feedback.rs`
- Delete: `crates/ext/guardian-reviewer/src/feedback_tests.rs`
- Modify: `crates/ext/guardian-reviewer/src/lib.rs`
- Modify: `crates/ext/guardian-reviewer/Cargo.toml`

**Interfaces:**
- Removes `FailedReviewFeedback`, `ReviewFeedbackContext`,
  `ReviewFeedbackSettings`, and process-local failed-review storage.
- Keeps `GuardianReviewSessionOutcome`, decision parsing, approval/denial,
  timeout, abort, and security enforcement.

- [ ] **Step 1: Remove host-side recording**

  Delete `record_failed_review` and its call from the guardian review session
  setup. Remove `mod feedback` from core guardian and delete the helper file.
  Remove feedback-only imports and the test assertion that reads retained
  failed-review attachments.

- [ ] **Step 2: Remove reviewer feedback module**

  Delete the guardian-reviewer feedback module and tests, remove its public
  exports, and remove its `codex-feedback` dependency. Do not alter the
  reviewer outcome or decision/reporting modules.

- [ ] **Step 3: Check guardian behavior**

  Run `cargo check -p codex-core -p codex-guardian-reviewer` and verify the
  remaining guardian call graph has no feedback storage or report attachment
  path.

### Task 6: Update documentation, regenerate graphs, and verify completion

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-plan.md`
- Modify: `docs/provider-specific-crates.md`
- Modify: generated dependency graph only if dependency edges changed
- Modify: any user-facing docs that still advertise `/feedback` or
  `feedback.enabled`

- [ ] **Step 1: Refresh the broad searches**

  Run:

  ```bash
  rg -n -i 'feedback|sentry|codex-feedback' crates docs --glob '*.rs' --glob '*.md' --glob '*.toml' --glob '*.json'
  ```

  Classify every remaining production match. Keep hook feedback, Sentry MCP,
  and unrelated diagnostic/observability names only with their current
  producer/consumer evidence.

- [ ] **Step 2: Record the completed capability removal**

  Update the inventory with exact removed symbols, the app-server/TUI/config/
  guardian producer-consumer audit, the retained guardian/security behavior,
  and `cargo check`/`just check` verification. Update Phase 3 and the crate
  candidate document to mark `codex-feedback` as fully removed.

- [ ] **Step 3: Regenerate the dependency graph if needed**

  Because the workspace member and dependency edges change, run
  `just crate-dependency-graph`. Confirm `codex-feedback` is absent and no
  unrelated edge changes were introduced.

- [ ] **Step 4: Run the required verification gate**

  Check the repository-local target size, remove only that directory if it is
  above 120 GB, then run:

  ```bash
  cargo check
  just check
  git diff --check
  ```

  Review the final diff for accidental removal of hook feedback, Sentry MCP,
  guardian enforcement, ordinary tracing, or current provider behavior.

