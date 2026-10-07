# Genex Desktop Tauri Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Linux/macOS-only local Tauri desktop application that reuses the existing app-server daemon, Unix-socket client, and generated protocol while providing a Vite/React frontend.

**Architecture:** The Tauri Rust backend owns daemon startup, one app-server client connection, typed commands, and event forwarding. The frontend uses generated protocol types, React Query for typed request lifecycle and selective stable data such as model lists, and an independent event store for live notifications and server requests.

**Tech Stack:** Rust workspace, Tauri 2, `genex-app-server-daemon`, `genex-app-server-client`, Vite, React, React Router, pnpm, TypeScript, TanStack React Query, Tailwind CSS, daisyUI, and `useSyncExternalStore`.

**Spec:** `docs/superpowers/specs/2026-10-07-genex-desktop-tauri-design.md`

## Global Constraints

- The desktop app is local-only and targets Linux and macOS; remote app-server connections are out of scope.
- Reuse `genex-app-server-daemon`; do not add a second Tauri-specific PID or daemon manager.
- Use the shared `GENEX_HOME` and existing local Unix-socket endpoint.
- The frontend never connects to the Unix socket directly; Tauri Rust uses `genex-app-server-client`.
- Use `pnpm` for frontend dependency management and scripts.
- App-server notifications and server requests are event-stream data, not React Query data by default.
- Run Rust build/check commands only through the repository's required memory-limited `systemd-run --user --scope --collect --quiet` wrapper.
- Run `just check`; run `just check --tests` when Rust code or test targets are changed; run `just generate` when checked-in generated artifacts are affected.
- Work directly on `main`; do not create a worktree.

## Review Focus

- A concurrent desktop/CLI start must return `Started` or `AlreadyRunning` and never create a second daemon; test the shared `GENEX_HOME` lifecycle path in the daemon integration target.
- A missing or stale managed package must fail with an actionable diagnostic rather than silently launching an incomplete binary; test package staging and metadata validation.
- A Unix-socket disconnect or initialize/protocol mismatch must produce a distinct connection state and recoverable error; test the Rust bridge state transitions.
- Interleaved notifications, deltas, and server-initiated requests must preserve event ordering and pending-request identity; test the event reducer independently from React Query.
- Generated request/response method types must reject mismatched params/results at compile time; test the generated `ClientResponse` export and a typed frontend RPC wrapper.

---

### Task 1: Export a generated typed client-response union

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/common.rs` in `client_request_definitions!`
- Modify: `crates/app-server-protocol/src/schema_fixtures.rs`
- Test: `crates/app-server-protocol/src/precomputed_exports_tests.rs`
- Generated: `crates/app-server-protocol/schema/typescript/ClientResponse.ts`
- Generated: `crates/app-server-protocol/schema/typescript/index.ts`
- Generated: `crates/app-server-protocol/schema/precomputed/app-server-exports-stable.json.zst`

**Interfaces:**
- Produces `ClientResponse` TypeScript output with `{ method, id, response }` discriminants.
- Enables frontend types equivalent to `Extract<ClientRequest, { method: M }>` for params and `Extract<ClientResponse, { method: M }>` for results.

- [ ] **Step 1: Add the TypeScript derive and wire-name metadata to `ClientResponse`**

  Update the macro-generated response enum so its TypeScript variant names use the wire method strings and its response payload remains associated with the same method.

- [ ] **Step 2: Include `ClientResponse` in the TypeScript schema fixture traversal**

  Collect the response union and its payload dependencies alongside `ClientRequest`, notifications, and server requests.

- [ ] **Step 3: Add a fixture assertion for the generated response union**

  Assert that the generated tree contains `ClientResponse.ts`, exports it from the generated index, and contains representative entries for `thread/list` and `turn/start`.

- [ ] **Step 4: Regenerate and inspect artifacts**

  Run:

  ```bash
  CARGO_BUILD_JOBS=2 systemd-run --user --scope --collect --quiet \
    -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
    --setenv=CARGO_BUILD_JOBS=2 just generate
  git diff -- crates/app-server-protocol/schema
  ```

  Expected: only the intentional generated TypeScript/index/precomputed export changes appear.

- [ ] **Step 5: Compile the affected Rust targets**

  Run `just check --tests` through the same memory-limited scope. Expected: the protocol crate and its test targets compile successfully.

- [ ] **Step 6: Commit**

  ```bash
  git add crates/app-server-protocol
  git commit -m "feat: export typed app-server responses to typescript"
  ```

### Task 2: Scaffold the pnpm frontend

**Files:**
- Create: `apps/desktop/package.json`
- Create: `apps/desktop/pnpm-lock.yaml`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/vite.config.ts`
- Create: `apps/desktop/tsconfig.json`
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/app/router.tsx`
- Create: `apps/desktop/src/app/providers.tsx`
- Create: `apps/desktop/src/app/query-client.ts`
- Create: `apps/desktop/src/styles/index.css`
- Create: `apps/desktop/src/components/layout/AppLayout.tsx`
- Create: `apps/desktop/src/protocol/index.ts`

**Interfaces:**
- Produces `pnpm dev`, `pnpm check`, `pnpm build`, and `pnpm tauri` scripts.
- Produces a `createHashRouter` root route suitable for a packaged Tauri asset without server-side history fallback.
- Produces a protocol re-export that resolves to `crates/app-server-protocol/schema/typescript/index.ts` without copying generated files.

- [ ] **Step 1: Create the package manifest and install the pinned dependency graph**

  Add React, React DOM, React Router, `@tanstack/react-query`, `@tauri-apps/api`, Vite, TypeScript, the React Vite plugin, Tailwind CSS, the Tailwind Vite integration, and daisyUI. Generate the lockfile with pnpm.

- [ ] **Step 2: Add Vite and TypeScript configuration**

  Configure the React plugin, the `@genex/protocol` path alias to the repository-owned generated schema directory, and the Vite filesystem allowlist needed to bundle that source outside `apps/desktop`.

- [ ] **Step 3: Add the application providers and router**

  Mount `QueryClientProvider`, the router, and a minimal layout with a connection/status placeholder. Keep routing hash-based for the initial packaged app.

- [ ] **Step 4: Add Tailwind and daisyUI styles**

  Configure the Tailwind/daisyUI entry stylesheet and verify a small layout component renders a daisyUI button/card without introducing a custom component library.

- [ ] **Step 5: Type-check and build the frontend**

  Run:

  ```bash
  cd apps/desktop
  pnpm check
  pnpm build
  ```

  Expected: both commands pass and the build consumes the repository-owned generated protocol source.

- [ ] **Step 6: Commit**

  ```bash
  git add apps/desktop
  git commit -m "feat: scaffold pnpm desktop frontend"
  ```

### Task 3: Add the complete Genex runtime package and Tauri shell

**Files:**
- Modify: `Cargo.toml` workspace members
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/capabilities/default.json`
- Create: `apps/desktop/scripts/stage-genex-package.py`
- Modify: `apps/desktop/package.json`
- Create/modify: `apps/desktop/src-tauri/resources/genex-package/**` through the staging command

**Interfaces:**
- Produces a Tauri application crate as a root Cargo workspace member.
- Produces a resource package with `bin/genex`, `genex-package.json`, `genex-path/`, and `genex-resources/` in the layout expected by `genex-install-context` and the daemon package installer.
- Produces a scoped Tauri capability that permits only the bundled Genex launcher/package commands; arbitrary shell execution is not enabled.

- [ ] **Step 1: Add the Tauri crate to the workspace**

  Add `apps/desktop/src-tauri` to the root workspace and configure the crate to depend on the existing app-server daemon/client/protocol crates plus Tauri. Keep the crate package name provider-neutral (`genex-desktop`).

- [ ] **Step 2: Implement deterministic runtime package staging**

  Stage the complete package shape used by the existing daemon tests and install-context code: `bin/genex`, the code-mode host helper, `genex-path/rg`, `genex-resources/bwrap`, and `genex-package.json` containing version, target, and `bin/genex` entrypoint. Fail if a required input is missing; do not create placeholder executables.

- [ ] **Step 3: Configure Tauri resources and capabilities**

  Bundle the staged runtime package as an application resource, configure the Linux/macOS targets, and expose only the commands/events required by the Rust bridge. Do not add a network permission or remote endpoint.

- [ ] **Step 4: Add a package-layout validation test**

  Test that the staging output is recognized by `InstallContext::from_exe`/the daemon package validation rules and that its metadata target matches the current build target.

- [ ] **Step 5: Build the shell and staged package**

  Run the frontend build and the repository-required Rust check through the memory-limited scope. Expected: the Tauri crate, package staging tool, and generated resource layout are valid on Linux; macOS path/target logic remains platform-gated rather than hard-coded to Linux.

- [ ] **Step 6: Commit**

  ```bash
  git add Cargo.toml apps/desktop
  git commit -m "feat: add tauri desktop shell and runtime package"
  ```

### Task 4: Reuse the daemon and connect the Rust app-server client

**Files:**
- Create: `apps/desktop/src-tauri/src/app_server/mod.rs`
- Create: `apps/desktop/src-tauri/src/app_server/lifecycle.rs`
- Create: `apps/desktop/src-tauri/src/app_server/client.rs`
- Create: `apps/desktop/src-tauri/src/app_server/bridge.rs`
- Create: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/main.rs`
- Modify: `crates/app-server-daemon/src/lib.rs`
- Modify: `crates/app-server-daemon/src/prepare_install.rs`
- Test: `apps/desktop/src-tauri/src/app_server/**` unit tests and the existing daemon integration target where lifecycle behavior changes

**Interfaces:**
- `lifecycle::ensure_started(app: &AppHandle) -> Result<DaemonInfo, DesktopAppServerError>` ensures the shared daemon is bootstrapped from the bundled package when needed, then calls the existing idempotent daemon start path.
- `client::connect(info: &DaemonInfo) -> Result<DesktopAppServerConnection, DesktopAppServerError>` connects through `RemoteAppServerEndpoint::UnixSocket` and performs initialize with desktop client metadata.
- `bridge::AppServerBridge` owns one connection, routes typed requests, and emits typed notifications/server requests to the Tauri app handle.
- Tauri commands expose `app_server_status`, `app_server_request`, and `app_server_resolve_server_request`; they return structured errors rather than strings.
- `genex_app_server_daemon::bootstrap_from_package(package_dir: &Path) -> Result<BootstrapOutput>` installs a validated complete package through the existing operation-lock/PID lifecycle and then starts the managed server.

- [ ] **Step 1: Add `bootstrap_from_package` to the existing daemon lifecycle**

  Refactor the existing package preparation path so a caller-provided complete package root can be validated and installed into `GENEX_HOME/packages/app-server-daemon/current`. The new function must delegate to the existing operation lock, package validation, PID backend, socket readiness probe, and recovery behavior; preserve the existing CLI `bootstrap` behavior unchanged.

- [ ] **Step 2: Implement startup and idempotent lifecycle handling**

  Resolve the shared `GENEX_HOME`, use the existing daemon lifecycle command, return `Started`/`AlreadyRunning` distinctly, and leave the daemon running when the Tauri window closes. Do not stop the shared daemon from the Tauri `RunEvent::Exit` path.

- [ ] **Step 3: Implement the Unix-socket client connection**

  Build `RemoteAppServerConnectArgs` with `RemoteAppServerEndpoint::UnixSocket`, client name `genex-desktop`, the desktop version, experimental capability setting, and the existing channel capacity policy. Keep the client in Tauri managed state.

- [ ] **Step 4: Implement bridge routing and Tauri commands**

  Route request responses by `RequestId`, forward `ServerNotification` and `ServerRequest` events, and provide resolve/reject handling for server-initiated requests. Distinguish transport, JSON-RPC server, decode, and initialization errors.

- [ ] **Step 5: Test lifecycle and bridge failure modes**

  Add tests for `AlreadyRunning`, a stale PID/socket recovery path, initialization failure classification, request/response correlation, interleaved notification ordering, and server-request resolution. Reuse the existing `crates/cli/tests/app_server_daemon.rs` fixtures rather than creating another fake daemon implementation.

- [ ] **Step 6: Run Rust verification**

  Run `just check --tests` through the required memory-limited systemd scope, then `just check`. Expected: all affected Rust production and test targets compile without standalone test execution.

- [ ] **Step 7: Commit**

  ```bash
  git add apps/desktop/src-tauri crates/app-server-daemon
  git commit -m "feat: connect desktop shell to local app server"
  ```

### Task 5: Add typed frontend IPC, event store, and selective React Query hooks

**Files:**
- Create: `apps/desktop/src/protocol/rpc.ts`
- Create: `apps/desktop/src/ipc/invoke.ts`
- Create: `apps/desktop/src/ipc/commands.ts`
- Create: `apps/desktop/src/ipc/errors.ts`
- Create: `apps/desktop/src/events/app-server-events.ts`
- Create: `apps/desktop/src/events/event-store.ts`
- Create: `apps/desktop/src/events/subscriptions.ts`
- Create: `apps/desktop/src/queries/models.ts`
- Create: `apps/desktop/src/features/threads/api.ts`
- Create: `apps/desktop/src/features/turns/api.ts`
- Test: `apps/desktop/src/protocol/*.test.ts`, `apps/desktop/src/events/*.test.ts`, `apps/desktop/src/queries/*.test.ts`

**Interfaces:**
- `invokeRpc<M extends RpcMethod>(method: M, params: RpcParams<M>): Promise<RpcResult<M>>`
- `useAppServerMutation<M extends RpcMethod>(method: M)` for typed actions.
- `useModelList()` as the first cached `useQuery` hook.
- `AppServerEventStore` exposes `subscribe`, `getSnapshot`, and `getServerRequest(requestId)` for `useSyncExternalStore` consumers.

- [ ] **Step 1: Implement the generated-type RPC map**

  Derive method-specific params and result types from generated `ClientRequest` and `ClientResponse`; make an intentionally incorrect params/result test fail at TypeScript compilation.

- [ ] **Step 2: Implement the typed Tauri invoke wrapper**

  Encode the method and params as the Rust bridge command expects, normalize structured bridge errors, and avoid an `as unknown as` cast in the public method-specific API.

- [ ] **Step 3: Implement the independent event store**

  Subscribe once to Tauri app-server events, reduce notification envelopes and server requests in arrival order, preserve pending request IDs, and expose snapshots through `useSyncExternalStore`.

- [ ] **Step 4: Add selective React Query integration**

  Add `useModelList` with explicit stale/cache policy. Add mutations for turn actions where loading/error state is useful, but keep live deltas and approval/input state in the event store.

- [ ] **Step 5: Test event and query boundaries**

  Verify that event delivery updates the event snapshot without requiring a Query observer, that model-list requests are typed and cacheable, and that a completed turn may invalidate a selected query without storing the entire stream in Query cache.

- [ ] **Step 6: Run frontend verification**

  ```bash
  cd apps/desktop
  pnpm check
  pnpm build
  ```

  Expected: type-check and production build pass.

- [ ] **Step 7: Commit**

  ```bash
  git add apps/desktop
  git commit -m "feat: add typed desktop app-server IPC"
  ```

### Task 6: Add the initial desktop vertical slice and final verification

**Files:**
- Create/modify: `apps/desktop/src/features/models/**`
- Create/modify: `apps/desktop/src/features/threads/**`
- Create/modify: `apps/desktop/src/features/turns/**`
- Modify: `apps/desktop/src/app/router.tsx`
- Modify: `apps/desktop/src/components/layout/AppLayout.tsx`
- Modify: `apps/desktop/package.json`
- Modify: relevant README/build documentation

**Interfaces:**
- Produces a startup/status view that proves daemon reuse and a model-list view that proves typed React Query IPC.
- Produces a thread/turn shell that consumes event-store state without making React Query the live turn store.

- [ ] **Step 1: Add the connection/startup screen**

  Render connecting, ready, disconnected, and initialization-error states from the Tauri bridge/event store.

- [ ] **Step 2: Add the model-list query view**

  Use `useModelList` and generated `ModelListResponse` types; show loading, error, empty, and success states.

- [ ] **Step 3: Add a minimal thread/turn event view**

  Display thread status and agent-message deltas from the event store, and wire start/interrupt actions through typed mutations.

- [ ] **Step 4: Add app-level frontend tests**

  Cover startup state rendering, model-list loading/error/success, and a notification sequence that renders a delta followed by completion.

- [ ] **Step 5: Run the complete repository verification**

  Run frontend `pnpm check` and `pnpm build`; run `just check` through the required memory-limited scope; run `just check --tests` if any Rust test-only code changed; run `just generate` and inspect the diff if protocol generation changed; run `git diff --check`.

- [ ] **Step 6: Commit**

  ```bash
  git add apps/desktop justfile README.md docs
  git commit -m "feat: add initial genex desktop app"
  ```
