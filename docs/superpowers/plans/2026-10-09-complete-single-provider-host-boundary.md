# Single-provider host boundary completion plan

## Goal

Complete the remaining single-provider Genex migration so that hosts construct one runtime-owned
`ModelProvider`, clients consume the provider-neutral contract, and no CLI/TUI/Exec path rebuilds or
selects a provider independently. Preserve the existing external OpenAI endpoint contract and the
repository's provider-neutral agent, MCP, sandbox, and security behavior.

## Architecture

The host selects one concrete provider and injects it into `AppServerRuntime`. `AppServerRuntime`
owns the shared provider contract and credential lifecycle. `core` receives only the provider
contract plus the host-owned request/telemetry context it needs; it must not depend on provider
registries, persisted provider selection, or a second runtime provider handle. Daemon and in-process
startup use the same construction path, with different process lifetimes only.

## Technical approach

1. Finish host construction and injection. Add a single public construction seam for configured
   provider runtimes and migrate CLI, TUI, Exec, login/logout/delete, and doctor setup to it. Remove
   host-side direct calls that create a second provider or only initialize a marker from an unrelated
   instance.
2. Migrate `core` and model-catalog construction from `SharedProviderRuntime` to
   `SharedModelProvider`. Extend the provider-neutral contract only for data needed by core
   (authentication snapshot, provider routing identity, attestation/error mapping, or explicit
   unsupported results), keeping wire payloads and concrete HTTP clients in the adapter.
3. Complete WebSocket and unauthorized-recovery/continuation handling through the same contract.
   Keep request-scoped auth snapshots and in-flight turn semantics; do not reintroduce provider
   selection into a thread or rollout.
4. Audit persisted thread/rollout/history/protocol/config readers and writers. Remove obsolete
   routing fields only after recording each producer/consumer decision in the compatibility
   inventory; retain external OpenAI names and required migration readers where they are still
   needed.
5. Audit host surfaces, generated artifacts, docs, and dependency edges; run repository-required
   checks and inspect all resulting diffs.

## Specifications

- Exactly one provider instance is associated with each runtime/AppServer.
- `InProcessAppServerBuilder` and daemon startup are lifecycle/configuration entry points, not
  separate provider factories.
- CLI/TUI/Exec clients receive an existing runtime or use the one shared configured construction
  seam; they never choose a provider from thread state.
- Model listing, auth, usage, capability, chat, WebSocket, and recovery operations route through
  `SharedModelProvider`.
- Unsupported optional capabilities return the contract's explicit unsupported result.
- Credentials are not persisted in threads or rollouts. A turn keeps its start-time credential
  snapshot; new turns observe credential changes or logout.
- A provider-scoped data root is validated before opening sensitive state.

## Global constraints

- Work directly on `main`; do not create a worktree.
- Follow `docs/provider-decoupling-plan.md`, `AGENTS.md`, and the phased order in the project
  instructions.
- Before changing an existing contract or persisted field, inspect all in-repository producers and
  consumers and record `remove`, `retain`, or `migrate-in-place` in
  `docs/pre-release-compatibility-inventory.md`.
- Rust build/check commands must run through the repository `just` recipes inside the user systemd
  memory scope. Use `just check` and, for Rust changes, `just check --tests`; use `just generate`
  when checked-in generated artifacts are affected.
- Do not run standalone tests unless explicitly requested.

## Review focus

- Verify there is no second provider construction or provider picker in CLI/TUI/Exec startup.
- Verify `core` no longer requires `SharedProviderRuntime` for normal execution.
- Verify WebSocket reconnect, unauthorized recovery, continuation, and telemetry preserve behavior.
- Verify provider identity is not read from thread/rollout routing state.
- Verify marker validation still occurs before SQLite, sessions, caches, or credentials are opened.
- Verify external OpenAI URLs, fields, auth headers/variables, and model identifiers are unchanged.

## Tasks

### 1. Host-side injection and construction audit

Files to inspect/change:

- `crates/app-server/src/runtime.rs`
- `crates/app-server/src/lib.rs`
- `crates/app-server/src/in_process.rs`
- `crates/app-server-client/src/lib.rs`
- `crates/cli/src/main.rs`
- `crates/cli/src/login.rs`
- `crates/cli/src/doctor.rs`
- `crates/tui/src/lib.rs`
- `crates/exec/src/lib.rs`
- `crates/exec/src/worktree.rs`

Steps:

1. Trace every provider construction and marker initialization from the host entry points.
2. Add/update a common runtime/provider factory API without exposing provider selection to thread
   execution.
3. Route login/logout/delete, doctor, TUI initialization, Exec, and daemon startup through that
   seam or an explicitly injected runtime.
4. Add compile-level contract coverage for explicit runtime injection and missing-runtime errors.
5. Run targeted app-server/client/CLI/TUI/Exec checks.

### 2. Direct `ModelProvider` boundary in core

Files to inspect/change:

- `crates/model-provider/src/contract.rs`
- `crates/model-provider/src/provider.rs`
- `crates/model-provider/src/model_catalog_source.rs`
- `crates/core/src/client.rs`
- `crates/core/src/thread_manager.rs`
- `crates/core/src/session/mod.rs`
- `crates/core/src/session/session.rs`
- `crates/core/src/session/turn_context.rs`
- `crates/core/src/session_startup_prewarm.rs`
- `crates/core/src/genex_delegate.rs`

Steps:

1. Inventory every remaining `SharedProviderRuntime` method used by core and classify it as
   provider-neutral contract data, host request context, or adapter-only implementation detail.
2. Add only the minimal provider-neutral methods/types needed by core, with explicit unsupported
   behavior for optional capabilities.
3. Change model client, session, thread manager, and model catalog constructors/getters to accept
   one `SharedModelProvider`; remove the paired runtime/contract fields.
4. Keep credential snapshot and auth lifecycle ownership in the runtime/provider implementation,
   not in persisted thread state.
5. Compile test targets and inspect all call sites before moving to transport migration.

### 3. WebSocket, continuation, and recovery contract migration

Files to inspect/change:

- `crates/core/src/client.rs`
- `crates/core/src/session_startup_prewarm.rs`
- `crates/core/src/responses_retry.rs`
- `crates/model-provider/src/provider.rs`
- `crates/model-provider/src/contract.rs`

Steps:

1. Move remaining auth setup, unauthorized recovery, API error mapping, and continuation decisions
   behind the provider contract where they describe provider behavior.
2. Keep WebSocket connection reuse, turn state, request telemetry, and HTTP fallback in core as
   provider-neutral transport orchestration.
3. Ensure an in-flight turn retains its captured auth snapshot while subsequent turns see updates.
4. Verify upgrade-required fallback, unauthorized recovery, retry limits, and provider-specific
   unsupported operations through compile-level tests/contracts.

### 4. Thread, rollout, history, and protocol persistence audit

Files to inspect/change:

- all production matches for `model_provider`, `provider_id`, provider selection, and provider
  routing in `crates/core`, `crates/protocol`, `crates/app-server`, fixtures, and schema paths
- `docs/pre-release-compatibility-inventory.md`
- `docs/provider-decoupling-candidates.md`

Steps:

1. Enumerate readers and writers in the required order: persisted rollout/thread-history, protocol/
   app-server schemas, CLI/config/import, UI/observability/runtime shims, and build scaffolding.
2. For every match, record producer, consumer, and an explicit `remove`, `retain`, or
   `migrate-in-place` decision before changing it.
3. Remove provider routing from execution and new protocol requests; preserve only necessary
   migration/history readers and external-contract identifiers.
4. Run `just generate` if checked-in schema/generated artifacts are affected and inspect the diff.

### 5. Final host/documentation/dependency audit

Files to inspect/change:

- `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`
- `docs/provider-decoupling-plan.md`
- `docs/pre-release-compatibility-inventory.md`
- `docs/provider-decoupling-candidates.md`
- host fixtures, UI, package manifests, and generated artifacts

Steps:

1. Search for provider picker/switch/fallback assumptions and stale builder names.
2. Document that another provider requires another runtime/AppServer and data root.
3. Remove dead dependencies only after the producer/consumer audit is complete.
4. Run `git diff --check`, targeted checks, full required checks, and inspect status/diff.
5. Commit and push only after verification, then report remaining items (if any) with file/line
   references.

## Self-review checklist

- [x] The plan's architecture matches the single-provider runtime requirement.
- [x] Each task identifies concrete repository surfaces and verification.
- [x] Persisted/protocol changes are gated by producer/consumer decisions.
- [x] The plan distinguishes lifecycle entry points from provider construction.
- [x] External OpenAI contracts and security behavior are explicitly preserved.
