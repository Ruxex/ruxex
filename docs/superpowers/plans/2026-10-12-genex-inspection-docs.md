# Genex Inspection Documentation Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a React-based pnpm + Docusaurus site under `docs/inspection` that teaches the current Genex architecture and runtime behavior through source-grounded diagrams, tables, and guides.

**Architecture:** Keep the site self-contained in `docs/inspection` with Docusaurus 3.10.2, React, TypeScript configuration, Markdown documentation, and Mermaid support. Organize content by learning path—system map, runtime, chat loop, context assembly, persistence, provider, capabilities, protocol, and source reference—while linking each behavior to the existing Rust implementation.

**Tech Stack:** pnpm, Docusaurus `3.10.2`, React, TypeScript, `@docusaurus/theme-mermaid`, Markdown, Mermaid.

**Spec:** `docs/superpowers/specs/2026-10-12-genex-inspection-docs-design.md`

## Global Constraints

- Work directly on the existing `main` branch; do not create a Git worktree.
- Keep the site independent from `apps/desktop` and the Rust workspace build.
- Use Japanese as the primary documentation language.
- Describe current implementation behavior, and distinguish current behavior from migration plans or historical compatibility layers.
- Use source links for major claims; avoid pretending to document every function.
- Use `onBrokenLinks: 'throw'` so broken internal documentation links fail the build.
- Do not run standalone Rust tests; this task changes only documentation-site files.

## Review Focus

- Source links must resolve to checked-in files and useful line anchors; verify representative links after content edits.
- Database documentation must match the five SQLite filenames, lazy thread-history opening behavior, and the current migration table names.
- The chat-loop explanation must distinguish a user-visible Turn from repeated model Sampling/Step iterations.
- Context-assembly documentation must distinguish data captured before a request from data persisted to rollout/SQLite.
- The Docusaurus site must build from `docs/inspection` with pnpm and must fail on broken internal links.

---

### Task 1: Scaffold the React documentation site

**Files:**
- Create: `docs/inspection/package.json`
- Create: `docs/inspection/pnpm-lock.yaml`
- Create: `docs/inspection/tsconfig.json`
- Create: `docs/inspection/docusaurus.config.ts`
- Create: `docs/inspection/sidebars.ts`
- Create: `docs/inspection/src/css/custom.css`
- Create: `docs/inspection/static/img/.gitkeep`

**Interfaces:**
- Produces: pnpm scripts `start`, `build`, `serve`, and `typecheck`; Docusaurus config with Japanese locale, Mermaid, broken-link failures, and sidebar categories used by later content tasks.

- [ ] **Step 1: Create the minimal site package and configuration**

  Use Docusaurus `3.10.2`, React 18, TypeScript, the classic preset, and `@docusaurus/theme-mermaid` `3.10.2`. Configure `docs/` as the content root and enable `markdown.mermaid`.

- [ ] **Step 2: Install dependencies and generate the lockfile**

  Run: `pnpm install`

  Expected: `docs/inspection/pnpm-lock.yaml` is generated and dependencies resolve without modifying unrelated workspace files.

- [ ] **Step 3: Add the base theme and navigation placeholders**

  Configure the sidebar IDs that later pages will use, add restrained typography/colors for long technical reading, and keep the theme implementation React-native without introducing a separate UI framework.

- [ ] **Step 4: Verify the empty site builds**

  Run: `pnpm build`

  Expected: Docusaurus produces a static build with exit code 0 and no broken-link errors.

- [ ] **Step 5: Commit**

  ```bash
  git add docs/inspection
  git commit -m "docs: scaffold Genex inspection site"
  ```

### Task 2: Add the learning path and system map

**Files:**
- Create: `docs/inspection/docs/intro/reading-order.md`
- Create: `docs/inspection/docs/intro/glossary.md`
- Create: `docs/inspection/docs/intro/system-map.md`
- Create: `docs/inspection/docs/runtime/execution-model.md`
- Create: `docs/inspection/docs/runtime/startup.md`
- Create: `docs/inspection/docs/runtime/app-server-runtime.md`
- Create: `docs/inspection/docs/index.md`
- Modify: `docs/inspection/sidebars.ts`

**Interfaces:**
- Consumes: Existing `docs/cli-architecture/*`, `crates/cli/src/main.rs`, `crates/core/src/lib.rs`, `crates/app-server/src/runtime.rs`, and `crates/app-server/src/lib.rs`.
- Produces: The recommended entry path and the shared vocabulary used by all later pages.

- [ ] **Step 1: Write the landing page and reading order**

  Explain that the site is an implementation-inspection guide, recommend a five-stage reading sequence, and link to the three user-priority topics.

- [ ] **Step 2: Write the glossary**

  Define AppServer, Runtime, Provider, Thread, Turn, Step, Rollout, Response Item, Tool, MCP, Plugin, Skill, StepContext, State DB, and Projection using current code semantics.

- [ ] **Step 3: Write the system map**

  Include a Mermaid flow from CLI/TUI/App Server through Core, Provider, Tools, Persistence, and UI. Link responsibilities to the existing crate entry points.

- [ ] **Step 4: Write runtime and startup pages**

  Describe command dispatch, Runtime construction, injected single Provider, Provider Home marker validation, and the difference between process, server, client, thread, and turn.

- [ ] **Step 5: Verify content links and build**

  Run: `pnpm build`

  Expected: exit code 0 with no broken-link errors.

- [ ] **Step 6: Commit**

  ```bash
  git add docs/inspection
  git commit -m "docs: explain Genex system and runtime model"
  ```

### Task 3: Document the chat loop and context assembly

**Files:**
- Create: `docs/inspection/docs/chat/chat-loop.md`
- Create: `docs/inspection/docs/chat/context-assembly.md`
- Create: `docs/inspection/docs/chat/streaming.md`
- Create: `docs/inspection/docs/chat/tool-loop.md`
- Create: `docs/inspection/docs/chat/compaction.md`
- Create: `docs/inspection/docs/chat/errors-and-cancellation.md`
- Modify: `docs/inspection/sidebars.ts`

**Interfaces:**
- Consumes: `crates/core/src/session/turn.rs`, `crates/core/src/session/step_context.rs`, `crates/core/src/session/session.rs`, `crates/core/src/context_manager/*`, `crates/core/src/stream_events_utils.rs`, and `crates/core/src/tools/*`.
- Produces: The central explanation of one user-visible Turn and its repeated Sampling/Tool iterations.

- [ ] **Step 1: Write the chat-loop page**

  Document `run_turn` in phases: admission, pre-sampling compact, required MCP/plugin discovery, StepContext capture, history preparation, provider sampling, post-sampling state, follow-up decision, auto-compact, hooks, completion, and error paths. Include a sequence diagram.

- [ ] **Step 2: Write the context-assembly page**

  Add a table with source, capture moment, model visibility, and persistence for config, model settings, history, AGENTS.md, environment, Git/worktree, permissions, sandbox, MCP, plugins, skills, tools, attachments, auth snapshot, time, and telemetry.

- [ ] **Step 3: Write streaming and tool-loop pages**

  Explain provider stream events, response-item reconstruction, assistant text/reasoning deltas, tool-call execution, tool output persistence, and why tool results cause another sampling request.

- [ ] **Step 4: Write compaction and failure pages**

  Explain pre-turn and mid-turn compaction, token-limit rollover, cancellation, invalid input/image, provider/auth errors, Stop Hook continuation, and pending/steered input.

- [ ] **Step 5: Verify representative source anchors and build**

  Run: `pnpm build`

  Expected: exit code 0; the build output contains no broken-link errors.

- [ ] **Step 6: Commit**

  ```bash
  git add docs/inspection
  git commit -m "docs: explain Genex chat loop and context assembly"
  ```

### Task 4: Document persistence, databases, and rollouts

**Files:**
- Create: `docs/inspection/docs/storage/storage-map.md`
- Create: `docs/inspection/docs/storage/sqlite.md`
- Create: `docs/inspection/docs/storage/tables.md`
- Create: `docs/inspection/docs/storage/rollouts.md`
- Create: `docs/inspection/docs/storage/migrations.md`
- Modify: `docs/inspection/sidebars.ts`

**Interfaces:**
- Consumes: `crates/state/src/sqlite.rs`, `crates/state/src/runtime.rs`, `crates/state/*_migrations/*.sql`, `crates/rollout/src/*`, and `crates/thread-store/src/local/*`.
- Produces: A source-grounded answer to how many DBs exist, what they store, and how JSONL and SQLite cooperate.

- [ ] **Step 1: Write the storage map**

  Show Provider Home, five SQLite files, rollout JSONL, credentials, attachments, and caches, explicitly distinguishing durable conversation records from projections and operational state.

- [ ] **Step 2: Write SQLite and table pages**

  List the five DB files, open timing, ownership, tables, key fields, indexes, and the fact that thread history is opened separately from the primary StateRuntime initialization.

- [ ] **Step 3: Write rollouts and migrations pages**

  Explain append-oriented JSONL rollouts, live writing, projection/backfill, migration application, and the removal of persisted `model_provider` from the current state schema.

- [ ] **Step 4: Verify schemas against checked-in SQL and build**

  Run: `pnpm build`

  Expected: exit code 0 and no broken links.

- [ ] **Step 5: Commit**

  ```bash
  git add docs/inspection
  git commit -m "docs: map Genex persistence and databases"
  ```

### Task 5: Document Provider, capabilities, protocol, and security

**Files:**
- Create: `docs/inspection/docs/provider/provider-contract.md`
- Create: `docs/inspection/docs/provider/authentication.md`
- Create: `docs/inspection/docs/provider/model-catalog.md`
- Create: `docs/inspection/docs/provider/provider-home.md`
- Create: `docs/inspection/docs/capabilities/tools.md`
- Create: `docs/inspection/docs/capabilities/mcp.md`
- Create: `docs/inspection/docs/capabilities/plugins.md`
- Create: `docs/inspection/docs/capabilities/skills.md`
- Create: `docs/inspection/docs/capabilities/sandbox.md`
- Create: `docs/inspection/docs/protocol/app-server.md`
- Create: `docs/inspection/docs/protocol/tui.md`
- Create: `docs/inspection/docs/protocol/desktop.md`
- Modify: `docs/inspection/sidebars.ts`

**Interfaces:**
- Consumes: `crates/model-provider/src/contract.rs`, `crates/model-provider/src/provider.rs`, `crates/login/src/auth/manager.rs`, `crates/genex-api/src/*`, `crates/protocol/src/*`, `crates/app-server-protocol/src/*`, `crates/tools/src/*`, `crates/execpolicy/src/*`, and `crates/sandboxing/src/*`.
- Produces: The boundary explanation for model access, auth lifecycle, optional capabilities, tool execution, and client-facing protocol.

- [ ] **Step 1: Write Provider pages**

  Explain provider-neutral contracts, stream events, capabilities, model catalog, auth state, credential snapshots, login/logout, and Provider Home marker validation without exposing secret values.

- [ ] **Step 2: Write capability pages**

  Explain how built-in tools, MCP, plugins, skills, approvals, sandbox, exec policy, and network policy become a per-step tool plan.

- [ ] **Step 3: Write protocol pages**

  Explain app-server requests/notifications, thread/turn lifecycle, TUI event consumption, Desktop’s client boundary, and reconnect/replay at a conceptual level.

- [ ] **Step 4: Verify links and build**

  Run: `pnpm build`

  Expected: exit code 0 with no broken-link errors.

- [ ] **Step 5: Commit**

  ```bash
  git add docs/inspection
  git commit -m "docs: explain Genex provider capabilities and protocol"
  ```

### Task 6: Add source reference, diagnostics, and final verification

**Files:**
- Create: `docs/inspection/docs/reference/crate-map.md`
- Create: `docs/inspection/docs/reference/source-map.md`
- Create: `docs/inspection/docs/reference/change-guide.md`
- Create: `docs/inspection/docs/reference/observability.md`
- Modify: `docs/inspection/docusaurus.config.ts`
- Modify: `docs/inspection/sidebars.ts`
- Modify: `docs/inspection/docs/intro/reading-order.md`

**Interfaces:**
- Consumes: All earlier documentation pages and the current crate/file layout.
- Produces: A practical map for future code reading and change work.

- [ ] **Step 1: Write the crate and source maps**

  Map major crates to responsibilities and provide stable file-level links for startup, chat, context, provider, protocol, state, rollout, and thread-store paths.

- [ ] **Step 2: Write the change guide and observability page**

  Give “if you change X, read Y” routes and explain logs, metrics, tracing, DB inspection, rollout inspection, and common failure boundaries.

- [ ] **Step 3: Add site metadata and navigation polish**

  Set the Japanese title, description, repository link, footer, and sidebar ordering. Ensure the landing page links to every major section.

- [ ] **Step 4: Run the final site checks**

  Run from `docs/inspection`:

  ```bash
  pnpm typecheck
  pnpm build
  pnpm serve --host 127.0.0.1 --port 4173
  ```

  Expected: TypeScript exits 0, Docusaurus build exits 0 with no broken-link errors, and the static server starts successfully.

- [ ] **Step 5: Inspect the final diff and commit**

  Run: `git diff --check` and `git status --short`

  Expected: no whitespace errors; only the intended documentation-site and plan/spec files are changed.

  ```bash
  git add docs/inspection docs/superpowers/plans/2026-10-12-genex-inspection-docs.md
  git commit -m "docs: complete Genex inspection guide"
  ```
