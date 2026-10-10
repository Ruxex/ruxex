# Provider-Free Client Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the TUI and other AppServer clients provider-free at their boundary: provider identity, authentication, capabilities, model catalog, usage, and provider-owned persistence are obtained from the connected AppServer, while client-local UI state remains local and reusable across AppServers.

**Architecture:** Each AppServer owns exactly one provider runtime and exposes an `AppServerIdentity` consisting of a stable opaque `signature` and compatibility `version`. The signature is persisted in the provider-scoped home marker; the version is the sole compatibility namespace and includes the AppServer/provider contract compatibility level. Client caches are keyed by `(signature, version)`. `providerKind` remains only in `genex-home.json` for mount validation and is not exposed as a client cache/provider descriptor. Remote clients do not open provider-owned SQLite or credentials.

**Tech Stack:** Rust workspace, Tokio, serde, AppServer protocol v1/v2, TUI ratatui/crossterm surfaces, SQLite state runtime, repository `just` recipes.

**Spec:** `docs/superpowers/specs/2026-10-10-provider-free-client-boundary-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers.
- Do not add a provider registry, provider picker, provider switch, or fallback provider to the core runtime.
- Do not persist credentials, provider routing, or provider selection in threads or rollouts.
- Treat the remote AppServer as the owner of provider credentials, provider SQLite, model catalog, capabilities, and usage.
- Run Rust compilation only through a memory-scoped `systemd-run --user --scope --collect --quiet` invocation of the repository `just` recipes, with `CARGO_BUILD_JOBS=2` for workspace checks.
- Do not run standalone test commands. Use `just check`, `just check --tests`, and `just generate` where applicable.

## Review Focus

- A client connected to AppServer A cannot display or reuse provider-derived state from AppServer B.
- A remote TUI startup never opens the provider-owned `state_5.sqlite`, `logs_2.sqlite`, `goals_1.sqlite`, `queue_1.sqlite`, or `thread_history_1.sqlite` through the local state runtime.
- A server restart preserves its signature, while a compatibility-version change invalidates client caches without changing the signature.
- The TUI does not require local `model_provider` values, `wire_api`, or `base_url` to render the connected server’s model/auth/status state.
- Existing local/embedded AppServer behavior remains intact, and switching providers still requires a separate runtime/data root.

---

## Task 1: Record the producer/consumer boundary decisions

**Files:**

- Modify `docs/pre-release-compatibility-inventory.md` in the UI/observability/runtime and CLI/config sections.
- Modify `docs/provider-decoupling-candidates.md` with the exact TUI/AppServer producer and consumer paths.

- [ ] Record `migrate-in-place` for TUI provider-derived configuration: `crates/tui/src/lib.rs`, `crates/tui/src/app_server_session.rs`, `crates/tui/src/status/card.rs`, and `crates/tui/src/chatwidget/model_popups.rs` consume server responses instead of local provider routing fields.
- [ ] Record `retain` for client-local presentation/configuration settings such as theme, keymap, editor, and UI preferences.
- [ ] Record `remove` or `migrate-in-place` for remote use of the local `StateRuntime`, based on the audited consumers of `state_db` and `log_db`; explicitly state that provider-owned SQLite is not a remote-client dependency.
- [ ] Record `migrate-in-place` for `RemoteServerMetadata` and the initialize response to carry AppServer identity, with no provider descriptor in the client identity.
- [ ] Confirm each decision has a producer, consumer, and reason before Rust changes begin.
- [ ] Run `git diff --check`.

## Task 2: Persist the stable AppServer signature and expose the identity

**Files:**

- Modify `crates/model-provider/src/home_marker.rs` and its tests.
- Modify `crates/app-server-protocol/src/protocol/v1.rs` and related protocol serialization/schema tests.
- Modify `crates/app-server/src/request_processors.rs` or the current initialize-response producer.
- Modify `crates/app-server-client/src/remote.rs` and initialize-response fixtures/tests.
- Update generated protocol artifacts only through `just generate` if the repository marks them as generated.

- [ ] Add a stable opaque `serverSignature` to the provider-home marker. Generate it only for a new root and reuse it on later mounts.
- [ ] Bump the marker format to the next format version and reject old/missing/incompatible markers before opening SQLite, sessions, caches, or credentials. Do not silently convert an old marker.
- [ ] Compare the provider-home descriptor fields (`providerKind` and `providerProtocolVersion`) for mount compatibility, but do not make the descriptor a client identity field.
- [ ] Define `AppServerIdentity { signature, version, build_version? }` in the shared protocol surface. `version` is the single compatibility namespace and must include the AppServer/provider contract compatibility level; `build_version` is diagnostic only and is excluded from cache keys.
- [ ] Return the identity from in-process and remote initialization, and expose it through the common AppServer client/session abstraction.
- [ ] Add tests for: new signature creation, signature reuse, descriptor mismatch, old marker rejection, initialize response round-trip, and identity stability across restart.
- [ ] Run `just generate` and inspect every generated diff, then run the required scoped `just check --tests` and `just check`.

## Task 3: Make bootstrap and reconnect server-derived

**Files:**

- Modify `crates/tui/src/app_server_session.rs` and `crates/tui/src/app_server_session/model_provider.rs`.
- Modify `crates/tui/src/app/reconnect.rs` and startup/bootstrap code under `crates/tui/src/app/`.
- Modify `crates/tui/src/model_catalog.rs` and affected TUI tests.

- [ ] Extend `AppServerBootstrap` with the current `AppServerIdentity`, provider-neutral provider status, and capabilities needed by the status/model UI.
- [ ] Fetch provider status/capabilities/model catalog from the AppServer during bootstrap and on reconnect; do not reconstruct them from `ConfigToml.model_provider`.
- [ ] Select the local configured model only when it is present in the connected server’s catalog. Otherwise use the server-marked default and then the first available model.
- [ ] On reconnect, replace the catalog, capabilities, auth/usage status, and identity as one server-context update; clear stale selected-model/reasoning state when the new catalog does not contain it.
- [ ] Add tests covering a stale local model, a server-provided default, and reconnecting from server context A to server context B.
- [ ] Run the required scoped `just check --tests` and `just check`.

## Task 4: Remove provider-specific reads from TUI presentation and configuration

**Files:**

- Modify `crates/tui/src/lib.rs` startup and target initialization.
- Modify `crates/tui/src/status/card.rs`.
- Modify `crates/tui/src/chatwidget/model_popups.rs`.
- Modify `crates/tui/src/config*`, `crates/tui/src/local_settings.rs`, and tests only where the producer/consumer audit proves a field is client-local or dead.

- [ ] Stop using the local provider ID as the source of the active provider in remote/connected-server mode.
- [ ] Stop using local `wire_api` to decide provider-specific presentation. Use provider-neutral server capabilities/model metadata for optional UI, and hide unsupported controls explicitly.
- [ ] Remove the local custom-provider base-URL warning from the connected-server path; the AppServer owns endpoint configuration and reports actionable auth/connection errors.
- [ ] Keep only client-local UI settings in the TUI-owned configuration projection. Leave generic execution policy settings intact unless their producer/consumer audit proves they are obsolete.
- [ ] Add tests proving a remote client with no provider-specific local configuration can display server status and catalog data.
- [ ] Run the required scoped `just check --tests` and `just check`.

## Task 5: Separate remote client state and scope caches by AppServer identity

**Files:**

- Modify `crates/tui/src/lib.rs` target state initialization.
- Modify `crates/rollout/src/state_db.rs` only if the audit requires an API clarification; do not change server-owned state semantics.
- Modify `crates/tui/src/model_catalog.rs` or add the smallest client cache-scope type in the TUI/AppServer-client boundary.
- Add focused tests for remote startup and cache scope behavior.

- [ ] Make the `Remote` target avoid `StateRuntime::init` and all provider-owned SQLite opens. Preserve any explicitly client-local UI state through a separate path, or use no local SQLite when no such consumer exists.
- [ ] Ensure remote logging/telemetry does not accidentally reintroduce provider-owned state through `state_db` or `log_db`.
- [ ] Define a cache scope from `(AppServerIdentity.signature, AppServerIdentity.version)`; never use endpoint URL, provider kind, or local provider configuration as the scope.
- [ ] Invalidate in-memory and persisted client caches when either identity component changes. Preserve client-local preferences that are intentionally independent of an AppServer.
- [ ] Add a test that connects to A, switches/reconnects to B, and observes no catalog/auth/status data from A.
- [ ] Run the required scoped `just check --tests` and `just check`.

## Task 6: Enforce the connection-boundary behavior

**Files:**

- Modify `crates/tui/src/app/reconnect.rs`, `crates/tui/src/app_server_session.rs`, and any request-state owner found by the audit.
- Modify AppServer client integration tests/fixtures for identity changes.
- Update protocol/client documentation where it currently implies provider selection by the client.

- [ ] Treat an identity change as a new server context: clear pending request handles/events, model catalog, capabilities, account/usage presentation, selected-model validation, and provider-derived UI state before applying the new bootstrap.
- [ ] Ensure in-flight requests are not silently attributed to the new server context; finish them against the old session or surface a reconnect/authentication error.
- [ ] Do not add same-TUI provider switching. Another provider continues to require another AppServer/runtime and provider-scoped data root.
- [ ] Add an integration-level regression test for context A → context B and a unit-level test for stale-event rejection.
- [ ] Run the required scoped `just check --tests` and `just check`.

## Task 7: Final generated-artifact and dependency audit

**Files:**

- Any generated protocol/schema artifacts changed by the implementation.
- `docs/pre-release-compatibility-inventory.md` and `docs/provider-decoupling-candidates.md`.
- Documentation or fixtures found by the final `rg` audit.

- [ ] Run `rg` for `model_provider`, provider selection/switch APIs, `wire_api`, `base_url`, provider cache keys, and local state initialization across TUI, AppServer client, protocol, rollout, and docs.
- [ ] For every remaining match, record `remove`, `retain`, or `migrate-in-place` with producer/consumer evidence; do not delete names required by the OpenAI endpoint or third-party/upstream contracts.
- [ ] Run `just generate` and inspect the complete diff.
- [ ] Run the required scoped `just check --tests` and `just check`.
- [ ] Run `git diff --check` and review the final diff for accidental provider descriptors, provider pickers, credential persistence, or broad unrelated cleanup.

## Commit Boundaries

Create reviewable commits at these boundaries, keeping each buildable:

1. Compatibility inventory/candidate decisions.
2. Home-marker signature and AppServer identity protocol.
3. Server-derived bootstrap and reconnect context.
4. TUI provider-specific read removal and remote state/cache isolation.
5. Final generated artifacts, documentation, and audit cleanup.

Each commit must pass the applicable scoped `just` checks before moving to the next boundary.
