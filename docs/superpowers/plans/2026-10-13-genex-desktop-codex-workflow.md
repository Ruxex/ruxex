# Genex Desktop Codex Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the initial Linux/macOS Genex Desktop workflow from the approved design: Tauri-managed local app-server, typed experimental protocol access, project/chat shell, live event state, approvals, and a usable conversation composer.

**Architecture:** Keep app-server authoritative for runtime, provider, auth, persistence, turns, and server requests. Tauri owns daemon lifecycle and the Unix-socket bridge. React Query owns authoritative snapshots while a reducer-backed event store owns live execution state. Git, worktree, and line-level diff comments remain out of scope.

**Tech Stack:** Rust/Tauri 2, existing Genex app-server client/daemon/protocol crates, React 19, TypeScript, Vite, React Router, TanStack React Query, Vitest, pnpm, and the repository's `just` verification recipes.

**Spec:** `docs/superpowers/specs/2026-10-13-genex-desktop-codex-workflow-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Target Linux and macOS only for the initial Desktop application.
- Reuse an existing compatible app-server daemon; bootstrap it through the existing lifecycle when it is unavailable.
- The frontend must never connect directly to the Unix socket.
- Use the existing provider-scoped Genex runtime and do not add provider selection to threads.
- Git status/diff mutation, managed worktrees, handoff, and line-level diff comments are deferred.
- Rust build/check commands run only through the repository's user systemd memory scope.
- Run `just check` always; run `just check --tests` when Rust code or test targets change; run `just generate` when checked-in generated artifacts change.
- Use `pnpm check`, `pnpm build`, and focused Vitest tests for the frontend.
- Commit each independently reviewable task and push `main` after each completed milestone group.

## Review Focus

- Stable TypeScript clients must not accidentally receive experimental methods; the Desktop-only experimental schema must be generated and selected explicitly.
- A second Desktop window or concurrent startup must reuse one daemon and not create a second client lifecycle.
- A lagged or disconnected event stream must trigger authoritative thread resynchronization instead of silently losing turn state.
- Server requests must preserve request IDs and never default to approval on disconnect or malformed payloads.
- Project/thread selection and new-chat submission must not create a thread with the wrong project or stale composer state.

---

### Task 1: Add a checked-in experimental TypeScript protocol surface

**Files:**
- Modify: `crates/app-server-protocol/src/schema_fixtures.rs`
- Modify: `crates/app-server-protocol/src/schema_fixtures_tests.rs`
- Modify: `justfile`
- Modify: `crates/app-server-protocol/scripts/write_schema_fixtures.py`
- Create/modify generated: `crates/app-server-protocol/schema/typescript-experimental/**`
- Modify: `apps/desktop/tsconfig.json`
- Modify: `apps/desktop/vite.config.ts`
- Test: `crates/app-server-protocol/src/schema_fixtures_tests.rs`

**Interfaces:**
- Produces a generated `typescript-experimental` tree containing the same generated unions as the protocol's experimental precomputed export, including `project/*`, `thread/queue/*`, `thread/search*`, and experimental fields such as `thread/start.projectId`.
- Keeps stable `schema/typescript` unchanged for existing consumers.
- Makes `@genex/protocol` resolve to the experimental tree only inside `apps/desktop`.

- [ ] **Step 1: Write the failing schema fixture assertion**

  Extend the schema fixture test to generate/check an experimental TypeScript tree and assert that `ClientRequest.ts` contains `project/list` and `thread/queue/add`, `ClientResponse.ts` contains their response variants, and `v2/ThreadStartParams.ts` contains `projectId`. Assert the stable tree still omits those experimental entries.

- [ ] **Step 2: Run the affected Rust test substitute and confirm the new assertion fails**

  Run the repository-required `just check --tests` through the memory-limited systemd scope. The new assertion must fail or the generated experimental output must be absent before implementation.

- [ ] **Step 3: Implement experimental fixture output**

  Extend the schema fixture writer with a deterministic `typescript-experimental` output path and update the generation recipe so `just generate` refreshes it. Preserve the existing experimental precomputed archive and stable output behavior.

- [ ] **Step 4: Point Desktop TypeScript resolution at the experimental tree**

  Update the Vite alias and TypeScript path mappings only for `apps/desktop`; do not change other repository consumers.

- [ ] **Step 5: Regenerate and inspect generated artifacts**

  Run the required scoped `just generate`, inspect the diff, and verify the experimental tree contains the required method/type files while stable fixtures remain unchanged.

- [ ] **Step 6: Run verification and commit**

  Run scoped `just check` and `just check --tests`. Commit with `feat: expose experimental desktop protocol types` and push `main`.

### Task 2: Make the Tauri runtime capability-aware and structured

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/capabilities/default.json`
- Test: `apps/desktop/src-tauri/src/lib.rs`

**Interfaces:**
- `connect_app_server` opts into the existing experimental API and continues using the shared Unix socket.
- Runtime errors map to structured categories required by the design.
- Existing daemon reuse/bootstrap behavior remains the single lifecycle path.

- [ ] **Step 1: Add failing Rust unit tests for capability and error classification**

  Add tests proving Desktop uses the experimental handshake setting and that initialization/transport/server failures map to distinct serializable error kinds without exposing credentials.

- [ ] **Step 2: Run the Rust test-target compile substitute and confirm the tests do not compile/pass yet**

  Run the scoped `just check --tests` command and record the expected failure caused by missing helpers or current `experimental_api: false` behavior.

- [ ] **Step 3: Implement the minimal runtime changes**

  Set the Desktop client capability to experimental, centralize error-kind mapping, and preserve the existing daemon bootstrap fallback. Do not add a Tauri-specific PID manager or stop hook.

- [ ] **Step 4: Verify the Rust targets compile and commit**

  Run scoped `just check` and `just check --tests`, then commit with `feat: enable desktop experimental app-server runtime` and push `main`.

### Task 3: Replace the event list with a reducer-backed live execution store

**Files:**
- Create: `apps/desktop/src/app-server/event-reducer.ts`
- Modify: `apps/desktop/src/app-server/event-store.ts`
- Modify: `apps/desktop/src/app-server/events.ts`
- Modify: `apps/desktop/src/app-server/hooks/useAppServerEvents.ts`
- Create: `apps/desktop/src/app-server/event-reducer.test.ts`
- Create: `apps/desktop/src/app-server/event-store.test.ts`

**Interfaces:**
- `reduceAppServerEvent(state, event) -> EventState` preserves ordered notification identity, streaming item accumulation, pending server requests, lagged/disconnected state, and per-thread active turn state.
- `useAppServerEvents()` exposes a stable snapshot suitable for conversation and approval components.
- Existing Tauri event payloads remain compatible with typed protocol unions.

- [ ] **Step 1: Write failing reducer tests**

  Cover an agent-message delta sequence, turn completion, approval request/resolve, duplicate notification identity, lagged events, and disconnect state. Assert state, not implementation calls.

- [ ] **Step 2: Run focused Vitest tests and confirm RED**

  Run `cd apps/desktop && pnpm vitest run src/app-server/event-reducer.test.ts src/app-server/event-store.test.ts`. Confirm failure because the reducer/state model does not exist.

- [ ] **Step 3: Implement the reducer and store integration**

  Normalize server notifications and server requests into keyed live state. Keep a bounded event history for diagnostics, but make conversation state derive from per-thread ordered items and snapshots. Mark the store resynchronizing after lagged/disconnected events.

- [ ] **Step 4: Verify focused and frontend checks**

  Re-run the focused tests, then `pnpm check` and `pnpm build`. Commit with `feat: add desktop app-server event store` and push `main`.

### Task 4: Add project-aware queries and the desktop shell

**Files:**
- Modify: `apps/desktop/src/app-server/query-keys.ts`
- Modify: `apps/desktop/src/app-server/queries.ts`
- Modify: `apps/desktop/src/app-server/hooks/useThreads.ts`
- Create: `apps/desktop/src/app-server/hooks/useProjects.ts`
- Modify: `apps/desktop/src/app-server/hooks/index.ts`
- Modify: `apps/desktop/src/app/router.tsx`
- Modify: `apps/desktop/src/components/layout/AppLayout.tsx`
- Create: `apps/desktop/src/components/sidebar/ProjectSidebar.tsx`
- Create: `apps/desktop/src/components/sidebar/ThreadList.tsx`
- Create: `apps/desktop/src/pages/ProjectHomePage.tsx`
- Modify: `apps/desktop/src/pages/NewChatPage.tsx`
- Modify: `apps/desktop/src/pages/ThreadsPage.tsx`
- Modify: `apps/desktop/src/pages/AccountPage.tsx`
- Test: `apps/desktop/src/app-server/hooks/useProjects.test.tsx`

**Interfaces:**
- Project hooks use `project/list`, `project/read`, and existing project mutation methods.
- Thread hooks support project filtering, search, archive/unarchive, rename, resume, and fork through generated experimental types.
- The shell provides selected project/thread navigation without duplicating server state in local storage.

- [ ] **Step 1: Write failing project-query and routing tests**

  Test that project queries use the selected project key, project filtering is passed to `thread/list`, and the shell renders a project with its recent chats plus a new-chat action.

- [ ] **Step 2: Run focused frontend tests and confirm RED**

  Run the new Vitest test and confirm failure because project hooks/sidebar components are absent.

- [ ] **Step 3: Implement project/thread hooks and query invalidation**

  Add typed project queries and mutations, update thread query helpers for the experimental project/search/archive fields, and invalidate affected snapshots after mutations or notifications.

- [ ] **Step 4: Implement the persistent shell and routes**

  Replace the top-only navigation with the project/chat sidebar, add `/chat/:threadId`, move account controls into settings-oriented UI, and preserve usable empty/loading/error states.

- [ ] **Step 5: Implement project-aware new chat**

  Submit `thread/start` with `projectId`, `cwd`, policy, model, and reasoning settings, then send the initial turn and navigate to `/chat/:threadId`. Do not use a second project-assignment request when the experimental field is available.

- [ ] **Step 6: Verify and commit**

  Run focused tests, `pnpm check`, and `pnpm build`. Commit with `feat: add project and chat desktop shell` and push `main`.

### Task 5: Implement the conversation workspace and composer

**Files:**
- Create: `apps/desktop/src/components/conversation/ConversationWorkspace.tsx`
- Create: `apps/desktop/src/components/conversation/ConversationItem.tsx`
- Create: `apps/desktop/src/components/conversation/Composer.tsx`
- Create: `apps/desktop/src/components/conversation/ActivityItem.tsx`
- Create: `apps/desktop/src/components/conversation/ConversationWorkspace.test.tsx`
- Modify: `apps/desktop/src/app-server/hooks/useTurns.ts`
- Modify: `apps/desktop/src/app-server/hooks/useThreads.ts`
- Modify: `apps/desktop/src/app-server/query-keys.ts`
- Modify: `apps/desktop/src/app/router.tsx`

**Interfaces:**
- The workspace consumes `useThreadConversation`, `useAppServerEvents`, `useSendMessage`, `useInterruptTurn`, and `turn/steer`.
- The composer supports normal send, Plan mode, Queue/Steer selection, attachments already supported by the protocol, and Stop.
- Completed snapshots and live event state render together without duplicate assistant content.

- [ ] **Step 1: Write failing conversation tests**

  Cover initial transcript rendering, streaming delta replacement, active-turn Stop visibility, normal send, Steer precondition handling, and empty/error states.

- [ ] **Step 2: Run focused Vitest tests and confirm RED**

  Run the workspace test and confirm failure because the conversation components and steer mutation are absent.

- [ ] **Step 3: Add turn controls and event-aware hooks**

  Add `useSteerTurn`, query invalidation after turn completion, and a small selector that merges authoritative thread items with live reducer items by item/turn identity.

- [ ] **Step 4: Implement the conversation workspace**

  Render ordered messages and activity items with collapsible details, live status, Plan/Goal context entry points, and a composer anchored to the bottom of the workspace.

- [ ] **Step 5: Verify and commit**

  Run focused tests, `pnpm check`, and `pnpm build`. Commit with `feat: add desktop live conversation workspace` and push `main`.

### Task 6: Add approval center, runtime settings, and recovery UX

**Files:**
- Create: `apps/desktop/src/components/approvals/ApprovalCenter.tsx`
- Create: `apps/desktop/src/components/approvals/ServerRequestCard.tsx`
- Create: `apps/desktop/src/pages/SettingsPage.tsx`
- Create: `apps/desktop/src/pages/ArchivePage.tsx`
- Modify: `apps/desktop/src/app-server/api.ts`
- Modify: `apps/desktop/src/app-server/event-store.ts`
- Modify: `apps/desktop/src/app-server/hooks/useAccount.ts`
- Modify: `apps/desktop/src/app-server/queries.ts`
- Modify: `apps/desktop/src/app/router.tsx`
- Modify: `apps/desktop/src/components/layout/AppLayout.tsx`
- Create: `apps/desktop/src/components/approvals/ApprovalCenter.test.tsx`

**Interfaces:**
- Approval components consume keyed server requests and call resolve/reject with exact request IDs.
- Settings reads provider, capabilities, models, auth, permissions, rate limits, and connection status through existing methods.
- Archive uses existing thread archive/unarchive methods; it does not delete or rewrite rollout data.

- [ ] **Step 1: Write failing approval and recovery tests**

  Cover approve, deny, user-input response, malformed/unknown request, disconnected pending request, and archive restoration state.

- [ ] **Step 2: Run focused Vitest tests and confirm RED**

  Confirm the new tests fail before adding the UI and structured request helpers.

- [ ] **Step 3: Implement approval resolution and safe error handling**

  Add typed helpers for resolving/rejecting server requests, keep pending entries until a resolved notification or explicit server error, and never auto-approve on disconnect.

- [ ] **Step 4: Implement settings/archive/recovery surfaces**

  Add runtime/provider/auth/capability presentation, login/logout refresh behavior, rate-limit display, retry/reconnect actions, archived thread browsing, and unarchive navigation.

- [ ] **Step 5: Verify and commit**

  Run focused tests, `pnpm check`, and `pnpm build`. Commit with `feat: add desktop approvals and runtime settings` and push `main`.

### Task 7: Full verification and completion review

**Files:**
- Modify only files required by verification findings.
- Update: `docs/superpowers/plans/2026-10-13-genex-desktop-codex-workflow.md` checkboxes and progress ledger.

- [ ] **Step 1: Run required repository verification**

  Run scoped `just check`; run scoped `just check --tests`; run `just generate` if generated schema artifacts changed; run `cd apps/desktop && pnpm check`, `pnpm test`, and `pnpm build`.

- [ ] **Step 2: Inspect the complete diff and test the acceptance checklist**

  Verify Linux/macOS scope, daemon reuse/bootstrap, experimental capability opt-in, project-aware chat creation, live event recovery, approval identity, and absence of Git/worktree/line-comment controls.

- [ ] **Step 3: Fix only verified findings with RED→GREEN tests**

  Use the systematic-debugging workflow for failures, add a regression test first, rerun the affected checks, and commit each meaningful fix.

- [ ] **Step 4: Push the final verified `main` history**

  Confirm `git status` is clean, `git log` contains the milestone commits, and `git push origin main` succeeds.

