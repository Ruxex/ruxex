# Realtime Voice Capability Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the provider/ChatGPT-specific Realtime voice capability while preserving ordinary text conversations, generic audio utilities, and non-voice app-server behavior.

**Architecture:** Remove the Realtime transport and voice device boundary first, then remove the protocol, persistence, core lifecycle, app-server, and TUI consumers. Keep shared crates such as `codex-api`, `codex-core`, `codex-protocol`, and `codex-tui`; only their Realtime-specific modules, fields, commands, schemas, and registrations are removed.

**Tech Stack:** Rust workspace, Cargo manifests, app-server JSON/TypeScript schemas, SQLite migrations, TUI state and snapshots.

**Spec:** `docs/provider-decoupling-plan.md` and `docs/provider-specific-crates.md` (Realtime voice capability entry).

## Global Constraints

- Work directly on `main`; do not create a worktree.
- This is a pre-release fork; remove the capability and its persisted compatibility surface rather than preserving old Realtime history readers.
- Preserve ordinary text conversation, generic audio preparation/utilities, provider-neutral media handling, sandbox/security behavior, and non-Realtime app-server APIs.
- Do not delete user rollout files automatically; remove only Realtime readers, writers, migrations, and projections.
- Do not add deletion-specific tests first; use focused reference audits and `just check` as the primary verification gate.
- Do not remove `codex-api`, `codex-core`, `codex-protocol`, or `codex-tui` as whole crates.

## Review Focus

- Realtime-only API and protocol types disappear without removing ordinary Responses/WebSocket or text-agent types.
- Realtime persistence and thread projections disappear without breaking ordinary paginated history.
- TUI startup, feature flags, settings, key bindings, and session lifecycle no longer reference voice state.
- Generic audio preparation and non-Realtime media handling remain available.
- Workspace manifests, generated schemas, prompts, snapshots, and documentation contain no orphaned Realtime voice surface.

### Task 1: Remove Realtime API and dedicated transport crates

**Files:**
- Modify: `Cargo.toml`, `crates/codex-api/src/endpoint/mod.rs`, `crates/codex-api/src/endpoint/realtime_call.rs`, and `crates/codex-api/src/endpoint/realtime_websocket/` registrations.
- Delete: `crates/realtime-webrtc/` and `crates/voice-host/`.
- Modify: TUI and packaging references that launch `codex-voice-host`.

- [x] Remove the Realtime call/WebSocket endpoint modules and their dedicated tests after confirming no non-Realtime consumer remains.
- [x] Remove `codex-realtime-webrtc` and `codex-voice-host` workspace members, dependencies, packaging/resource references, and logging registrations.
- [x] Run `rg -n -i 'realtime/calls|realtime_websocket|codex-realtime-webrtc|codex-voice-host|realtime-webrtc|voice-host' Cargo.toml crates scripts` and classify remaining matches.
- [x] Run `cargo check -p codex-api -p codex-tui`.

### Task 2: Remove Realtime protocol, app-server, and generated schemas

**Files:**
- Modify or delete: `crates/protocol/src/realtime.rs`, `crates/protocol/src/lib.rs`, and Realtime event/request registrations.
- Modify: `crates/app-server-protocol/src/protocol/v2/realtime.rs`, `v2/mod.rs`, shared protocol types, request/notification exports, and generated JSON/TypeScript schemas.
- Modify: `crates/app-server/src/request_processors/turn_processor.rs`, dispatch/notification modules, and Realtime tests.

- [x] Remove Realtime request, response, notification, voice, session, and audio-delta types and their conversions from the active protocol.
- [x] Remove app-server Realtime request processors and dispatch arms while retaining ordinary thread/turn requests.
- [x] Remove Realtime schema files and update generated aggregate schemas without unrelated changes.
- [x] Run `rg -n -i 'ThreadRealtime|RealtimeVoice|RealtimeConversation|realtime_start|realtime_append|realtime_stop' crates/app-server-protocol crates/app-server crates/protocol` and classify remaining matches.
- [x] Run `cargo check -p codex-protocol -p codex-app-server-protocol -p codex-app-server`.

### Task 3: Remove Realtime persistence and core lifecycle

**Files:**
- Modify: `crates/state/migrations/0005_thread_realtime_items.sql`, migration tests, state extraction/runtime code, and thread metadata projections.
- Modify or delete: `crates/thread-store/src/local/thread_history/realtime.rs`, thread-history modules, types, and materialization/projection code.
- Modify: `crates/rollout/`, `crates/core/src/realtime_*`, `crates/core/src/context/realtime_*`, `crates/core/src/session/realtime_history.rs`, and Realtime prompt registrations/templates.
- Modify: Realtime-specific config and feature declarations in `crates/core/src/config/` and `crates/features/src/lib.rs`.

- [x] Remove Realtime rollout item types, persistence policy, SQLite projection, state migration, history reader, model-context handling, and metadata fields.
- [x] Remove core Realtime conversation/context/prompt/session lifecycle and its event mappings without changing ordinary turn/history handling.
- [x] Remove Realtime-only config fields, feature flags, and prompt templates.
- [x] Run `rg -n -i 'realtime|voice' crates/state crates/thread-store crates/rollout crates/core/src crates/features/src crates/prompts/src crates/prompts/templates` and retain only generic audio/media matches with documented reasons.
- [x] Run `cargo check -p codex-state -p codex-thread-store -p codex-rollout -p codex-core -p codex-features -p codex-prompts`.

### Task 4: Remove TUI voice UI and lifecycle wiring

**Files:**
- Modify: `crates/tui/src/app.rs`, app event/command/input/session modules, `app_server_session/`, and chatwidget modules.
- Delete or modify: TUI Realtime/voice UI modules, tests, snapshots, settings, key bindings, and tooltips.
- Modify: TUI config/startup/experimental feature displays and any app-server session state initialization.

- [x] Remove voice controls, microphone shortcuts, voice settings, Realtime replay state, transcript lifecycle, app-server calls, and rendering branches.
- [x] Remove voice-only tests and snapshots while preserving ordinary composer, replay, status, and generic audio behavior.
- [x] Remove feature availability checks and startup log filters for the deleted transport.
- [x] Run `rg -n -i 'realtime|voice|microphone|speech' crates/tui/src --glob '*.rs' --glob '*.snap'` and classify every remaining match.
- [x] Run `cargo check -p codex-tui`.

### Task 5: Documentation, inventory, and workspace verification

**Files:**
- Modify: `docs/provider-decoupling-plan.md`, `docs/provider-specific-crates.md`, and `docs/pre-release-compatibility-inventory.md`.
- Modify any remaining active documentation, packaging metadata, or generated schema indexes identified by the audits.

- [x] Record the Realtime capability as removed, including the retained generic audio/non-voice boundaries and the unsupported-old-history policy.
- [x] Re-run the repository-wide Realtime/voice searches and ensure every production match is removed or explicitly retained with a producer/consumer rationale.
- [x] Run `git diff --check`.
- [x] Run `just check` as the final verification gate. It completed with 0 errors; existing compiler warnings remain.
- [x] Confirm `git status`, the dependency graph impact, and generated-schema diff contain only this capability removal.
