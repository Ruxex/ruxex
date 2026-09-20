# Remove the Feedback Capability Design

## Goal

Remove the `codex-feedback` product capability from this pre-release fork. The
capability includes the user-facing feedback flow, its Sentry delivery path,
the in-memory log and metadata capture used only by that flow, and the
guardian failed-review attachments that are collected solely for feedback
reports.

The normal guardian safety/review decision flow, ordinary tracing, rollout
storage, diagnostics used by other current features, and provider-neutral
agent behavior remain active.

## Context and decision

`codex-feedback` is not an independently provider-neutral diagnostic service
in the current repository. Its active consumer is the Codex feedback report
flow: TUI `/feedback` sends an app-server `feedback/upload` request, and the
app-server serializes the report and attachments for the fixed Sentry DSN.
The feedback logger and metadata layers exist to populate those reports. The
guardian failed-review buffer is read by the same app-server feedback
processor so it can be attached to a report; it has no other production
consumer.

The capability is therefore classified as `remove`, rather than as a
provider-adapter-only split. Sentry is the transport implementation detail;
the product capability being removed is Codex's feedback/reporting surface.
This intentionally removes the user-facing ability to submit feedback in
this pre-release fork.

## Scope

### Remove

- The `codex-feedback` crate and its workspace/dependency declarations.
- The `FeedbackUpload` app-server request, response, parameters, generated
  schema entries, request processor, and related tests.
- The TUI `/feedback` command, feedback selection/consent UI, background
  request helpers, and feedback state held by TUI components.
- The `feedback.enabled` configuration field, TOML representation, schema,
  defaults, warnings, and configuration tests.
- `CodexFeedback` construction and installation of its logger and metadata
  layers in TUI and app-server startup paths.
- Feedback-only request metadata emitters and the tags whose sole purpose was
  attaching request/auth context to feedback reports.
- Guardian failed-review feedback recording and report attachment plumbing,
  while preserving the guardian review decision, approval, denial, and
  security behavior.
- Feedback-specific rollout attachment, diagnostic attachment, upload,
  retry, Sentry envelope, and report tests.
- Documentation and inventory entries that describe the removed capability.

### Retain

- Ordinary `tracing` subscribers and the state/log database paths that have
  current consumers outside feedback.
- Guardian review execution and all safety/permission enforcement.
- Rollout writing, reading, paging, and metadata behavior unrelated to report
  attachments.
- Generic provider, transport, authentication, and model behavior.
- Unrelated MCP references to the Sentry service; those are user-configured
  MCP integrations, not the removed feedback capability.

## Architecture after removal

The app-server no longer owns a feedback processor or exposes a feedback
request. TUI no longer stores a `CodexFeedback` instance, installs feedback
tracing layers, or exposes a feedback command. Startup paths retain only the
ordinary logging and observability layers required by current consumers.

Guardian review completion still returns its normal outcome. The
`record_failed_review` helper and its bounded feedback storage are removed so
failed review context is not retained solely for a deleted report flow. No
new replacement store is introduced.

The workspace has no `codex-feedback` dependency after the change. Any
remaining references to feedback-related names must either be removed or be
explicitly classified as unrelated current behavior in the compatibility
inventory.

## Producer/consumer audit

| Surface | Producer | Consumer | Decision |
| --- | --- | --- | --- |
| `CodexFeedback` logger/metadata layers | TUI and app-server startup | Feedback report construction | remove |
| `FeedbackUpload` protocol request | TUI background request | app-server feedback processor | remove |
| Sentry event/envelope upload | app-server feedback processor | Fixed Sentry endpoint | remove |
| Guardian failed-review attachment | Guardian failure handler | Feedback processor attachment list | remove |
| Guardian review outcome and enforcement | Guardian review session | Core session/approval flow | retain |
| Ordinary tracing/state logging | Core, app-server, TUI | Current runtime diagnostics and state consumers | retain |

## Error and compatibility behavior

There is no migration or replacement endpoint. Existing feedback-related
configuration is no longer read, and existing feedback protocol requests are
no longer accepted because this is a pre-release fork with no external
users. Existing rollout/config files are not deleted automatically.

Removed old feedback fields, requests, and report artifacts are not replayed
or converted. Current non-feedback configuration and app-server requests keep
their canonical behavior.

## Verification

Before the workspace check, inspect the repository-local `target/` size. If it
exceeds the documented 120 GB threshold, remove only that repository-local
`target/` directory before checking.

Use the repository cleanup policy's primary verification commands:

```bash
rg -n 'codex-feedback|FeedbackUpload|feedback_enabled|feedback/upload|CodexFeedback|SENTRY_DSN' \
  crates docs --glob '*.rs' --glob '*.md' --glob '*.toml' --glob '*.json'
cargo check -p codex-core -p codex-app-server -p codex-tui -p codex-protocol
just check
git diff --check
```

Do not run tests for this cleanup; `just check` is the repository's required
verification gate. Regenerate app-server/config schemas when their source
types change and inspect generated diffs for unrelated changes.
