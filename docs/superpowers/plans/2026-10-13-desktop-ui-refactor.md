# Desktop UI Structure Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize the desktop UI's hooks, page composition, and reusable components without changing user-visible behavior or app-server protocol calls.

**Architecture:** Keep existing route and barrel exports stable while splitting app-server thread queries from mutations and grouping runtime/provider queries by responsibility. Extract layout/page presentation into focused components and reuse thread/card state components where the current screens share the same behavior.

**Tech Stack:** React 19, React Router, TanStack Query, TypeScript, Biome, ESLint, Vitest, Vite.

**Spec:** Existing desktop workflow implementation and the preceding component-boundary review in this conversation.

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Preserve app-server protocol method names, query keys, route paths, and visible behavior.
- Keep `useEffect` usage lifecycle-only; do not introduce dependency-driven effects.
- Preserve the public exports from `apps/desktop/src/app-server/hooks/index.ts`.
- Run `pnpm check`, `pnpm test`, `pnpm build`, and `git diff --check` from `apps/desktop` before completion.

## Review Focus

- Existing pages continue to render loading, error, empty, and success states after extraction.
- Query invalidation after thread mutations remains unchanged.
- A missing project id does not trigger a project thread request.
- Layout logout and navigation behavior remain unchanged.
- The refactor does not add a new `useEffect` or create unstable render loops.

---

### Task 1: Split app-server query and mutation hooks

**Files:**
- Create: `apps/desktop/src/app-server/hooks/useThreadQueries.ts`
- Create: `apps/desktop/src/app-server/hooks/useThreadMutations.ts`
- Create: `apps/desktop/src/app-server/runtime-queries.ts`
- Create: `apps/desktop/src/app-server/provider-queries.ts`
- Modify: `apps/desktop/src/app-server/hooks/useThreads.ts`
- Modify: `apps/desktop/src/app-server/queries.ts`
- Modify: `apps/desktop/src/app-server/hooks/index.ts`
- Modify: `apps/desktop/src/app-server/hooks/useProjects.ts`
- Modify: `apps/desktop/src/app-server/hooks/useTurns.ts`

**Interfaces:**
- Preserve all current hook names and signatures exported by `hooks/index.ts`.
- `useThreadQueries.ts` owns read-only thread list/search/read/turn/item/conversation hooks and list parameter helpers.
- `useThreadMutations.ts` owns create/archive/unarchive/rename/resume/fork mutations.
- `runtime-queries.ts` owns `useAppServerStatus`.
- `provider-queries.ts` owns model/provider/capability/permission queries.

- [x] **Step 1: Move read-only thread hooks without changing their signatures**
- [x] **Step 2: Move thread mutations and preserve invalidation behavior**
- [x] **Step 3: Split runtime/provider query implementations and keep compatibility re-exports**
- [x] **Step 4: Consolidate React Query imports and guard empty project ids**
- [x] **Step 5: Run `pnpm check` and `pnpm test` from `apps/desktop`**
- [x] **Step 6: Commit the hook refactor**

---

### Task 2: Extract shared desktop UI components

**Files:**
- Create: `apps/desktop/src/components/common/ThreadCard.tsx`
- Create: `apps/desktop/src/components/common/QueryState.tsx`
- Create: `apps/desktop/src/components/layout/TopBar.tsx`
- Create: `apps/desktop/src/components/settings/SettingsCards.tsx`
- Modify: `apps/desktop/src/components/layout/AppLayout.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Modify: `apps/desktop/src/pages/ThreadsPage.tsx`
- Modify: `apps/desktop/src/pages/ArchivePage.tsx`
- Modify: `apps/desktop/src/pages/ProjectHomePage.tsx`
- Modify: `apps/desktop/src/components/sidebar/ThreadList.tsx`

**Interfaces:**
- Shared components receive data and callbacks as props; they must not call Tauri APIs directly.
- `TopBar` owns navigation, connection display, and logout controls while `AppLayout` owns shell composition.
- Settings cards own presentation only; `SettingsPage` remains the query composition boundary.

- [x] **Step 1: Add focused shared thread/card and query-state components**
- [x] **Step 2: Extract top-bar responsibilities from `AppLayout`**
- [x] **Step 3: Split settings sections into presentation components**
- [x] **Step 4: Replace repeated thread markup in list/archive/project pages**
- [x] **Step 5: Keep selected thread state explicit in `ThreadList`**
- [x] **Step 6: Run the desktop test suite and lint checks**
- [x] **Step 7: Commit the shared UI refactor**

---

### Task 3: Final verification and review

**Files:**
- Modify: only files required by verification fixes.

- [x] **Step 1: Review the complete diff for behavior changes and new effects**
- [x] **Step 2: Run `pnpm check`**
- [x] **Step 3: Run `pnpm test`**
- [x] **Step 4: Run `pnpm build`**
- [x] **Step 5: Run `git diff --check` and confirm a clean intended diff**
- [x] **Step 6: Commit any final fixes and push `main`**
