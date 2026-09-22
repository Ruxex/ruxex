# Codex CLI Architecture Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task.

**Goal:** Add a commit-pinned documentation snapshot that explains every `codex` CLI capability, its runtime path, the defining crate of each relevant Trait, and the normalized cross-cutting architecture.

**Architecture:** Create one immutable snapshot directory under `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/`. Command pages describe user-visible entry points and link into common pages; common pages describe configuration, transport, sessions, execution, persistence, and output once; reference pages provide the authoritative Trait, crate, and source maps.

**Tech Stack:** Markdown, repository-relative source references, `rg`, `git`, and the repository's `just check` verification command.

**Spec:** `docs/superpowers/specs/2026-09-22-cli-architecture-documentation-design.md`

## Global Constraints

- Work directly on the `main` branch; do not create a Git worktree.
- Keep the shared architecture provider-neutral and suitable for general-purpose AI agents.
- Follow the repository verification policy: always run `just check` for the production workspace type-check.
- Run `just generate` only if checked-in generated artifacts are affected; this plan changes Markdown only.
- Do not run standalone test commands for this cleanup/documentation task unless explicitly requested.
- Treat the snapshot commit as `c902263c127cddfea4a072f2c183040083426697`; source references must describe that code state.

## Review Focus

- Command coverage: every `Subcommand` variant and its dispatch arm is represented, including hidden/internal commands with an explicit visibility label.
- Boundary ownership: `codex-cli`, `codex-tui`, `codex-exec`, `codex-core`, app-server crates, protocol crates, and API/client crates are not conflated.
- Trait provenance: every listed Trait points to its defining declaration rather than only a re-export or implementation.
- Common-flow normalization: command pages link to common behavior instead of copying conflicting descriptions of config, auth, session, tool, sandbox, persistence, or output handling.
- Snapshot integrity: all referenced files, line anchors, and links resolve against the repository state represented by the snapshot.

### Task 1: Build the snapshot index and CLI entrypoint map

**Files:**
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/README.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/00-system-map.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/01-cli-entrypoint.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/reference/source-map.md`

**Interfaces:**
- Consumes: `Subcommand`, nested subcommand enums, and the dispatch match in `crates/cli/src/main.rs`.
- Produces: the root navigation and source-of-truth command inventory used by every later page.

- [ ] **Step 1: Extract the command inventory from the snapshot source**

  Inspect `crates/cli/src/main.rs` with `nl -ba`, record the exact top-level and nested command names, and verify each has a dispatch arm. Include `None` as the default interactive path and mark `tcp-tunnel`, `execpolicy`, `stdio-to-uds`, and hidden debug/app-server variants as internal where appropriate.

- [ ] **Step 2: Write the snapshot README**

  Create `README.md` with the full commit hash, scope, reading order, command-page links, common-page links, reference-page links, and a warning that this is a source snapshot rather than a stable API promise.

- [ ] **Step 3: Write the system map**

  Create `00-system-map.md` with the normalized pipeline `argv -> parse/dispatch -> config -> auth/provider -> transport -> session -> turn/tool -> policy/sandbox -> persistence/telemetry -> output`, and map each stage to its owning crate without implying that one crate owns all behavior.

- [ ] **Step 4: Write the CLI entrypoint page**

  Create `01-cli-entrypoint.md` describing `crates/cli/src/main.rs`, `MultitoolCli`, root option folding, remote endpoint selection, validation/rejection helpers, `arg0` dispatch, and the final match that routes into TUI, exec, app-server, or utility paths. Link every command group to its command page.

- [ ] **Step 5: Write the source map**

  Create `reference/source-map.md` with a table containing command group, defining file/module, dispatch symbol, major downstream crate, and snapshot line. Resolve line numbers using `nl -ba` against the current code, whose non-documentation content matches the pinned source commit.

- [ ] **Step 6: Review navigation and command completeness**

  Run `rg -n '^\s*[A-Z][A-Za-z]+\(' crates/cli/src/main.rs` and targeted searches for `Some(Subcommand::` to compare the source inventory with the README links. Fix missing or duplicate command entries before continuing.

### Task 2: Document each CLI capability entry point

**Files:**
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/interactive.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/agents.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/exec.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/review.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/sessions.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/mcp.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/plugin.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/app-server.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/auth.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/diagnostics.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/sandbox.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/debug.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/policy.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/commands/utility.md`

**Interfaces:**
- Consumes: Task 1 navigation, `reference/source-map.md`, and the command-page template from the approved design.
- Produces: capability-first pages that name the CLI entry, dispatch symbol, runtime path, state/output behavior, constraints, and links to common abstractions.

- [ ] **Step 1: Document interactive entry points**

  Write `interactive.md` for `codex` with no subcommand, including `TuiCli`, prompt/image input, startup orchestration, and TUI exit handling. Write `agents.md` separately for the shared-daemon overview and its remote/no-daemon constraints.

- [ ] **Step 2: Document model execution entry points**

  Write `exec.md` for `codex exec`, including streaming/non-streaming output modes, command variants, `codex-exec` ownership, and the handoff into core turn execution. Write `review.md` for `codex review` and identify the review-specific argument path.

- [ ] **Step 3: Document session lifecycle entry points**

  Write `sessions.md` for `resume`, `queue`, `archive`, `delete`, `unarchive`, and `fork`. Distinguish interactive resume/fork from daemon-mediated queue/archive/delete operations and link to the normalized session/persistence pages.

- [ ] **Step 4: Document extensibility and server entry points**

  Write `mcp.md`, `plugin.md`, and `app-server.md`, covering their nested commands, owning modules/crates, transport boundaries, protocol/schema generation, daemon lifecycle, and whether an operation mutates local configuration or state.

- [ ] **Step 5: Document auth, diagnostics, and sandbox entry points**

  Write `auth.md`, `diagnostics.md`, and `sandbox.md`. Include login status/logout behavior, doctor’s read-mostly checks, host-specific sandbox dispatch, remote-mode rejection, and the relevant common pages.

- [ ] **Step 6: Document debug, policy, and utility entry points**

  Write `debug.md`, `policy.md`, and `utility.md` for debug subcommands, execpolicy checks, completion, features, exec-server, stdio-to-UDS, TCP tunnel, and schema/trace utilities. Mark hidden/internal operations explicitly.

- [ ] **Step 7: Check cross-page consistency**

  Search all command pages for repeated definitions of configuration, auth, session, tool, sandbox, and output behavior. Replace duplicated explanations with links to Task 3 common pages while retaining command-specific constraints.

### Task 3: Normalize common architecture and index Traits/crates

**Files:**
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/configuration.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/authentication.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/provider-and-model.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/session-and-thread.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/tool-execution.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/sandbox-and-permissions.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/transport-and-server.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/persistence-and-rollout.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/output-and-errors.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/feature-flags.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/common/telemetry.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/reference/trait-index.md`
- Create: `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/reference/crate-index.md`

**Interfaces:**
- Consumes: command pages, source searches for `pub trait`, `pub(crate) trait`, and capability-relevant private Traits, plus Cargo package names and module paths.
- Produces: one normalized explanation for each shared concern and an authoritative Trait/crate ownership index linked by all command pages.

- [ ] **Step 1: Document configuration and identity selection**

  Write `configuration.md`, `authentication.md`, and `provider-and-model.md` covering CLI override folding, `CODEX_HOME`/profiles, `ConfigBuilder`, auth providers, model-provider selection, API/client transport, and provider-neutral core boundaries.

- [ ] **Step 2: Document sessions, tools, and policy**

  Write `session-and-thread.md`, `tool-execution.md`, and `sandbox-and-permissions.md` covering thread lifecycle, turns, queued work, tool registry/runtime, approval, execpolicy, platform sandbox selection, and user verification.

- [ ] **Step 3: Document transport, persistence, and output**

  Write `transport-and-server.md`, `persistence-and-rollout.md`, and `output-and-errors.md` covering local/remote/app-server transport, protocol serialization, rollout/history/thread state, TUI/JSON/process output, retries, and error propagation.

- [ ] **Step 4: Document flags and telemetry**

  Write `feature-flags.md` and `telemetry.md` covering feature resolution, command gating, metrics/tracing initialization, consent/config behavior, and the distinction between foreground CLI and detached services.

- [ ] **Step 5: Build the Trait index from definitions**

  Use `rg -n '(^|\s)(pub\s+)?(crate\s+)?trait\s+'` over capability crates, then inspect each declaration with `nl -ba`. Include relevant Traits such as `AuthProvider` (`codex-api`), `SseTelemetry`/`WebsocketTelemetry` (`codex-api`), tool runtime contracts (`codex-core`), session task contracts (`codex-core`), event sources (`codex-tui`), and protocol/client boundary Traits when they are on a documented path. For each entry record exact visibility, defining crate, module/file/line, purpose, implementations/consumers, and classification.

- [ ] **Step 6: Build the crate index**

  Write `crate-index.md` grouping the CLI-facing crates into entrypoint, UI, orchestration, execution/policy, protocol/server, API/client, state/persistence, and platform support. For each crate include package name, Rust crate name, responsibility, and links to the command/common pages that use it.

- [ ] **Step 7: Link command pages to common ownership**

  Add links from every command page to the exact common page and Trait index entries it depends on. Ensure common pages do not introduce a second definition of a command-specific behavior.

### Task 4: Verify the snapshot and deliver the documentation

**Files:**
- Modify: all files under `docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/` as needed for corrections.

**Interfaces:**
- Consumes: all documentation from Tasks 1–3 and the pinned source commit.
- Produces: a checked, navigable documentation snapshot with no unresolved source references or command omissions.

- [ ] **Step 1: Validate referenced paths and snapshot identity**

  Run a small shell loop over source paths mentioned in the snapshot and confirm each exists. Confirm `git rev-parse c902263c127cddfea4a072f2c183040083426697^{commit}` resolves and that source references point to files present in that commit.

- [ ] **Step 2: Validate command and Trait coverage**

  Compare the command inventory in `README.md` and `source-map.md` with `Subcommand`/nested enum declarations and dispatch arms. For every Trait index row, run a targeted `rg` search for its defining declaration and correct any row whose definition is only a re-export.

- [ ] **Step 3: Validate Markdown links and formatting**

  Check every relative Markdown link using a shell/Python-free path-resolution loop or the repository’s available Markdown tooling, then run `git diff --check`. Fix broken links, missing blank lines, malformed tables, and inconsistent command names.

- [ ] **Step 4: Run repository verification**

  Run `just check`. Do not run standalone tests. Do not run `just generate` because this change does not modify generated artifacts; if verification reveals generated files are affected, run `just generate` and inspect the diff.

- [ ] **Step 5: Perform final review and commit**

  Review the complete diff for provider-specific assumptions, duplicated common explanations, missing Trait provenance, accidental code changes, and unsupported claims. Commit the snapshot with a message such as `docs: add commit-pinned cli architecture map`.
