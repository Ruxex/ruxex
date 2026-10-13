# Genex Desktop Codex Workflow Design

## Status

Design approved in conversation. This document defines the next desktop
implementation boundary. Git/worktree integration and line-level diff comments
are intentionally deferred.

## Goal

Turn `apps/desktop` into a useful local Genex Desktop application that follows
the current Codex Desktop interaction model while reusing the existing
app-server daemon and protocol.

The first implementation target is a reliable local workflow:

```text
launch Desktop -> reuse/start daemon -> choose project -> start/resume chat
  -> stream agent activity -> handle approvals -> continue/stop/steer
```

The Desktop application is a local control plane. The app-server daemon owns
agent execution, authentication, provider state, persistence, sandboxing, and
MCP behavior.

## Scope

### In scope

- Linux and macOS through Tauri.
- A Tauri Rust backend that reuses or starts the existing app-server daemon.
- A React frontend with a project/chat sidebar and a conversation workspace.
- Project and thread listing, creation, resume, fork, rename, archive, and
  search where the existing protocol supports them.
- Live turn and item streaming through app-server notifications.
- Approval and user-input requests sent by app-server.
- Model, provider, capability, account, login, logout, and rate-limit status.
- Plan, Goal, Queue, Steer, and Stop where the existing protocol and
  experimental capability surface support them.
- Reconnection and authoritative state resynchronization.
- Typed TypeScript access to existing experimental protocol methods.

### Deferred

- Git status, raw Git diff, staging, unstaging, reverting, and commit actions.
- Managed worktree creation, cleanup, and local/worktree handoff.
- Line-level diff comments and persisted review annotations.
- Remote or cloud app-server connections.
- A second daemon manager, second persistence layer, or Desktop-owned agent
  runtime.

## Product model

Desktop is organized around Project, Chat, and Runtime rather than around a
model picker.

- **Project** is a persisted group of local roots and related chats.
- **Chat** is a persisted app-server thread with its own conversation and
  execution settings.
- **Runtime** is the shared local app-server daemon and its injected single
  provider.
- **Execution context** is initially the local checkout represented by the
  thread `cwd`. Worktrees are reserved for a later phase.

The primary shell has three logical regions:

```text
┌──────────────┬──────────────────────────────┬─────────────────┐
│ Project/Chat │ Conversation                 │ Context drawer   │
│ sidebar      │ messages + live activity     │ Plan / Goal /    │
│              │ composer                     │ approvals / log  │
└──────────────┴──────────────────────────────┴─────────────────┘
```

The right context drawer is optional so the conversation remains the primary
surface. Git and worktree controls must not appear as partially working fake
controls in this phase.

## Startup and runtime lifecycle

1. Tauri resolves the Desktop Genex home and validates the provider-scoped
   data-root boundary before sensitive state is opened.
2. Tauri tries to connect to the existing local app-server socket.
3. If no compatible daemon is available, Tauri uses the existing daemon
   bootstrap/lifecycle path and bundled runtime package.
4. Tauri connects through `genex-app-server-client` and performs one
   `initialize` handshake with Desktop client metadata.
5. Desktop opts into the existing experimental API surface required by Project
   and Queue workflows, with typed client support generated from the repository
   schema.
6. The frontend loads runtime, provider, auth, model, and project state.

An already running compatible daemon is reused. Desktop must not create a
parallel PID manager or stop the shared daemon merely because its window
closes. A daemon/socket/protocol failure is shown as a distinct startup or
connection state with a retry action.

## Screen and route model

The existing pages are reorganized around a persistent shell.

- `/` — project/chat home and recent work.
- `/chat/new` — project and chat creation flow.
- `/chat/:threadId` — conversation workspace.
- `/settings` — runtime, provider, account, permissions, and application
  settings.
- `/archive` — archived chats and restoration.

The current standalone `ThreadsPage` becomes sidebar and archive data rather
than the main product surface. The current account page becomes a settings
section. The current home and new-chat pages become the initial shell and
composer workflow.

### New chat

The new-chat flow selects, in order:

1. Project or local root.
2. Approval and sandbox policy.
3. Model and reasoning effort.
4. Optional Plan mode.
5. Initial prompt and attachments.

The initial implementation uses `thread/start` with the selected project and
local working directory. Project assignment should use the existing
`thread/start.projectId` field when the experimental API is enabled, avoiding a
non-atomic start-then-assign sequence.

### Conversation

The conversation renders user messages, assistant messages, reasoning summary,
plan updates, tool calls, command output, file-change activity, warnings, and
errors as ordered activity items. Streaming deltas update the current item
without rebuilding the entire transcript on every event.

The composer supports:

- text and supported attachments;
- Plan/default mode;
- Queue as the default follow-up behavior;
- Steer while a turn is active;
- Stop while a turn is active;
- slash-command affordances only for commands backed by existing protocol or
  Desktop behavior.

## State architecture

### Server state

TanStack React Query owns request/response state and authoritative snapshots:

- project list and project details;
- thread list, thread details, turns, and items;
- model list;
- current provider and capabilities;
- account/auth and rate-limit snapshots;
- skills, plugins, MCP status, permissions, goals, and queue snapshots.

### Live execution state

An app-server event store owns ordered live state. It is not a React Query
cache. State is partitioned by `threadId` and contains:

- active turn;
- ordered activity items;
- streaming deltas;
- pending server requests;
- queue state;
- connection state.

The reducer must preserve event ordering and request identity. Duplicate or
late notifications must not create duplicate visible items.

### UI state

React-local or small app-level state owns selected project/thread, panel state,
composer drafts, expanded items, and visual preferences. UI state is never
persisted into threads or rollouts unless the app-server protocol explicitly
owns the field.

### Reconnection

After a disconnect or lagged event stream:

1. mark the runtime disconnected or resynchronizing;
2. reconnect with bounded retry;
3. reload the affected thread, turns, items, queue, and account snapshots;
4. resume notification reduction after the authoritative snapshot is applied.

The client must not infer completed turn state solely from missing events.

## Approval and server-request flow

The Rust bridge forwards typed server requests to the frontend as app-server
events. The frontend stores them as pending requests with thread and request
IDs. The UI presents command, file-change, permissions, tool-input, and MCP
elicitation requests in a consistent approval surface.

- Approve or answer calls the existing resolve path.
- Deny calls the existing reject path.
- A resolved notification removes the pending entry.
- Disconnect marks the request indeterminate and offers retry/reconnect rather
  than silently approving or denying it.

## Protocol audit and decisions

No new app-server method is required for the initial product workflow.

### Retain and expose

- `project/*` for project navigation and management.
- `thread/*` for chat lifecycle, history, goals, metadata, sections, and
  attachments.
- `turn/start`, `turn/steer`, and `turn/interrupt`.
- `thread/queue/*` for queued follow-ups.
- `review/start` and existing turn/item diff activity, without line comments or
  Git mutation controls.
- `model/list`, `modelProvider/read`, and
  `modelProvider/capabilities/read`.
- `account/*`, permission profiles, skills, plugins, MCP status, and filesystem
  methods already provided by app-server.

### Required protocol-surface work

The Rust protocol already defines several required methods as experimental, but
the generated TypeScript `ClientRequest` / `ClientResponse` unions do not yet
expose every one of them. The implementation must update schema generation and
the frontend typed facade for the experimental methods used by Desktop. The
Desktop connection must opt into that experimental surface explicitly.

This is a type/export and capability-gating change, not a new wire protocol.

### Native Desktop boundary

Daemon lifecycle, native file/folder dialogs, opening an external editor, and
future Git/worktree operations belong to Tauri/Rust. They must not be invented
as app-server methods in this phase.

## Error handling

Errors are represented by stable categories rather than raw strings:

- `dataRootMismatch` — provider/data-root marker prevents startup;
- `daemonUnavailable` — daemon could not be reused or bootstrapped;
- `protocolMismatch` — initialize or schema compatibility failure;
- `authenticationRequired` — new turns are rejected until login;
- `transport` — socket or connection failure;
- `server` — app-server rejected a request;
- `unsupported` — provider capability or server feature is unavailable;
- `validation` — client input is invalid.

Each error includes a user-facing message, a retry/remediation action where
possible, and diagnostic details only behind an expandable UI. Credentials and
secret values are never placed in UI state, thread metadata, or error text.

## Implementation milestones

Each milestone must build and remain reviewable.

1. **Protocol surface** — expose required experimental request/response types,
   enable the Desktop capability opt-in, and add typed facade tests.
2. **Runtime shell** — make daemon reuse/bootstrap, initialize, reconnect, and
   structured error states reliable in Tauri.
3. **Event foundation** — implement the reducer/event store, server-request
   handling, lagged-event recovery, and focused frontend tests.
4. **Project and chat shell** — replace the placeholder pages with the sidebar,
   project selection, thread list, resume, new-chat flow, rename, archive, and
   search.
5. **Live conversation** — render ordered items and deltas; add composer,
   Stop, Steer, Plan, Goal, and Queue where supported by the server.
6. **Runtime settings** — integrate auth, provider status, capabilities,
   models, permissions, rate limits, and connection diagnostics.
7. **Hardening** — verify Linux/macOS startup, reconnect, approval flows,
   provider/data-root failures, and the repository's required checks.

Git/worktree and line-comment milestones are separate follow-up designs.

## Acceptance criteria

- On Linux and macOS, Tauri reuses an existing compatible daemon or starts it
  through the existing daemon lifecycle.
- All Desktop clients attached to that daemon observe the same provider and
  auth state.
- A user can choose a project, start a chat, resume a chat, and see live agent
  activity without polling the transcript for every delta.
- Approval and user-input requests are visible and resolvable with stable
  request identity.
- Queue, Steer, Stop, Plan, and Goal use existing protocol capabilities and
  fail explicitly when unsupported.
- Reconnection resynchronizes from authoritative app-server snapshots.
- Experimental protocol methods used by Desktop are typed and capability-gated.
- Git mutation, worktree management, and line-level diff comments are absent
  from the initial scope rather than represented by nonfunctional controls.

## References

- OpenAI, [Codex App Server](https://learn.chatgpt.com/docs/app-server)
- OpenAI, [Projects and chats](https://learn.chatgpt.com/docs/projects)
- OpenAI, [Code review](https://learn.chatgpt.com/docs/code-review)
- OpenAI, [Git worktrees](https://learn.chatgpt.com/docs/environments/git-worktrees)
- Existing repository foundation:
  `docs/superpowers/specs/2026-10-07-genex-desktop-tauri-design.md`
