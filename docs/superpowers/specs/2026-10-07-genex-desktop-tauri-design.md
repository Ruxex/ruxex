# Genex Desktop Tauri Design

## Status

Approved conversational design. This document defines the initial desktop
application boundary; implementation planning is intentionally separate.

## Goal

Add a local-only Tauri desktop application to the repository. The application
uses the existing app-server daemon and app-server protocol, with a Vite/React
frontend. The daemon remains the authoritative runtime for agent work, thread
state, persistence, and LLM access.

## Decisions

- Place the application under `apps/desktop/`.
- Use Tauri for the native shell and Rust-side integration.
- Use `pnpm`, Vite, React, React Router, TanStack React Query, Tailwind CSS,
  and daisyUI for the frontend.
- Reuse `genex-app-server-daemon` instead of creating a second daemon manager.
- Use the existing local Unix-socket app-server endpoint and shared
  `GENEX_HOME`.
- Connect to the Unix socket from Tauri Rust through
  `genex-app-server-client`; the frontend never connects to the socket
  directly.
- Treat app-server notifications and server requests as an event stream,
  independent from React Query's request/cache model.
- Use React Query selectively for typed request handling, loading/error state,
  and stable server-state such as the model list. It is not the primary store
  for live turn state or notification streams.

## Repository layout

```text
apps/desktop/
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/
│   ├── main.tsx
│   ├── app/
│   │   ├── router.tsx
│   │   ├── providers.tsx
│   │   └── query-client.ts
│   ├── components/
│   │   ├── layout/
│   │   └── ui/
│   ├── features/
│   │   ├── threads/
│   │   ├── turns/
│   │   ├── models/
│   │   └── settings/
│   ├── protocol/
│   │   ├── index.ts
│   │   └── rpc.ts
│   ├── ipc/
│   │   ├── invoke.ts
│   │   ├── commands.ts
│   │   └── errors.ts
│   ├── events/
│   │   ├── app-server-events.ts
│   │   ├── event-store.ts
│   │   └── subscriptions.ts
│   └── styles/
│       └── index.css
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/
    ├── icons/
    ├── binaries/
    │   └── genex-<target-triple>
    └── src/
        ├── main.rs
        ├── commands.rs
        └── app_server/
            ├── mod.rs
            ├── lifecycle.rs
            ├── client.rs
            └── bridge.rs
```

The Tauri crate is added to the root Cargo workspace. The frontend is kept
outside `crates/` so app-specific UI and shell code do not become shared Rust
library responsibilities.

## Runtime architecture

```text
Tauri Rust backend
  ├─ ensure `genex app-server daemon start`
  ├─ connect with `genex-app-server-client`
  ├─ expose typed Tauri commands
  └─ emit typed app-server events
       │
       └─ local Unix socket
            │
            └─ app-server daemon
```

At startup, the Tauri backend calls the existing daemon lifecycle entry point.
The operation is idempotent: an existing daemon is reused. All desktop and
CLI processes use the same `GENEX_HOME`, socket path, and daemon state. The
daemon is allowed to remain running after the desktop window closes, because it
is shared local infrastructure rather than a window-owned child process.

The bundled `genex` executable is supplied as a Tauri sidecar. The first
implementation must verify that the existing daemon bootstrap/package flow can
consume this bundle. If it cannot, the follow-up should generalize the daemon's
managed-binary selection; it must not introduce a parallel PID manager in the
Tauri layer.

## Frontend and IPC

The generated TypeScript under
`crates/app-server-protocol/schema/typescript/` remains the source of truth and
is imported through `src/protocol/index.ts`. It is not copied into a second
checked-in generated tree.

`src/protocol/rpc.ts` provides a typed method facade. Where the current schema
generator does not emit a method-to-result map, the generator should be
extended to emit one alongside the existing request and response types. The
facade should expose method-specific parameter and result types rather than a
general `unknown` response cast.

React Query is used as follows:

- `useQuery` for stable reads such as `model/list`.
- `useMutation` for user actions such as starting or interrupting a turn.
- Typed request helpers for all app-server calls, even when a call is not
  cached.
- Limited invalidation or cache updates when an event makes a stable query
  stale; events do not flow through React Query.

App-server notifications and server-initiated requests use a dedicated
`AppServerEventBus`. The event store is the source for live turn output,
message deltas, approval prompts, tool input, and connection state. A small
`useSyncExternalStore`-compatible adapter may expose this state to React; no
additional state library is required initially.

## Event and request flow

1. Tauri ensures the shared daemon is running and establishes one Rust-side
   app-server client connection.
2. Tauri commands send typed JSON-RPC requests through that connection.
3. Rust forwards server notifications as typed Tauri events.
4. The frontend event store reduces live notifications into UI state.
5. Server-initiated requests are represented as pending event-store entries;
   UI actions resolve or reject them through a typed command.
6. Completed or authoritative snapshots may refresh selected React Query
   entries, but live turn state is not reconstructed from Query cache.

## Failure handling

- Daemon start failure is surfaced as an application startup error with the
  daemon's diagnostic context.
- Socket disconnect transitions the event store to a disconnected state and
  triggers bounded reconnect/status handling.
- Protocol or initialization mismatch is reported separately from transport
  failure.
- A daemon that is already running is reused; the Tauri app does not launch a
  second app-server process.
- Closing the Tauri window does not stop the shared daemon by default.

## Verification

- `pnpm check` type-checks the frontend.
- `pnpm build` validates the Vite production build.
- Event reducers and typed IPC adapters receive focused frontend tests.
- Rust changes use the repository-required `just check` command through the
  memory-limited systemd scope; `just check --tests` is also run when Rust test
  targets or `cfg(test)` code are affected.
- Protocol generator changes run `just generate` and the resulting diff is
  inspected.

## Out of scope for the initial desktop shell

- Remote or network app-server connections.
- A new OS-level service manager integration.
- A second Tauri-specific daemon/PID implementation.
- Automatic daemon binary updates from the desktop UI.
- Treating every app-server notification as React Query cache data.
