# Codex CLI Architecture Documentation Design

## Purpose

Document the current implementation of every user-facing and operational entry point reachable from `codex`, with the documentation anchored to an immutable full commit hash. The documentation is for engineers who want to understand how a CLI capability enters the system, which crates own the behavior, and which abstractions are shared across capabilities.

The snapshot commit for this document is:

```text
c902263c127cddfea4a072f2c183040083426697
```

## Goals

- Make every `codex` top-level command discoverable from a command-oriented index.
- Explain each capability from CLI parse/dispatch through its main runtime path.
- Explicitly record the defining crate and source location for every relevant Trait.
- Normalize cross-cutting behavior into reusable common documents instead of repeating it in command pages.
- Preserve enough source paths and line references for the documentation to be audited against this snapshot.
- Make the snapshot understandable without requiring the reader to start from the crate dependency graph.

## Non-goals

- This is not a user manual or complete API reference.
- It does not document every private UI rendering Trait or every test helper when those do not form part of a CLI capability path.
- It does not change Rust code, generated schemas, protocol behavior, or compatibility policy.
- It does not claim that a source line remains stable after the snapshot commit; later code changes require a new snapshot directory.

## Documentation layout

Create the following directory:

```text
docs/cli-architecture/c902263c127cddfea4a072f2c183040083426697/
```

The directory contains:

```text
README.md
00-system-map.md
01-cli-entrypoint.md
commands/
  interactive.md
  agents.md
  exec.md
  review.md
  sessions.md
  mcp.md
  plugin.md
  app-server.md
  auth.md
  diagnostics.md
  sandbox.md
  debug.md
  policy.md
  utility.md
common/
  configuration.md
  authentication.md
  provider-and-model.md
  session-and-thread.md
  tool-execution.md
  sandbox-and-permissions.md
  transport-and-server.md
  persistence-and-rollout.md
  output-and-errors.md
  feature-flags.md
  telemetry.md
reference/
  trait-index.md
  crate-index.md
  source-map.md
```

## Command coverage

The command pages group related entry points without hiding the actual command names:

- `interactive.md`: `codex`, including the default TUI path and `codex agents`.
- `exec.md`: `codex exec` and its non-interactive execution modes.
- `review.md`: `codex review`.
- `sessions.md`: `resume`, `queue`, `archive`, `delete`, `unarchive`, and `fork`.
- `mcp.md`: `codex mcp` and MCP login/configuration paths.
- `plugin.md`: `codex plugin` and marketplace operations.
- `app-server.md`: app-server foreground mode, daemon lifecycle, proxy, and schema generation.
- `auth.md`: `login`, `login status`, and `logout`.
- `diagnostics.md`: `doctor`.
- `sandbox.md`: host-specific `sandbox` execution and platform selection.
- `debug.md`: `debug models`, `debug app-server`, `debug prompt-input`, trace reduction, and memory reset.
- `policy.md`: `execpolicy check` and the relationship to tool approval/sandboxing.
- `utility.md`: `completion`, `features`, `exec-server`, and internal transport utilities (`stdio-to-uds`, `tcp-tunnel`).

Hidden or internal commands are included where they explain a production execution path, but are labeled as internal rather than presented as supported end-user features.

## Standard shape of a command page

Each command page uses the same sections:

1. Capability and supported command spellings.
2. CLI definition and dispatch entry point.
3. Main call graph, with links to shared documents.
4. Inputs, outputs, and state transitions.
5. Relevant Traits and types, each linked to `reference/trait-index.md`.
6. Platform, authentication, remote-mode, or feature-flag constraints.
7. Source map for the snapshot.

The command pages describe behavior in terms of the capability first. Crate names and module paths are included at the point where they explain ownership or an interface boundary.

## Normalized common model

The common documents describe the following normalized pipeline:

```text
argv
  -> root CLI parsing and override folding
  -> config/home/profile resolution
  -> auth/provider/model selection
  -> local or remote transport selection
  -> thread/session admission
  -> turn/command/tool execution
  -> approval and sandbox enforcement
  -> persistence/rollout and telemetry
  -> TUI, JSON, protocol, or process exit output
```

The normalized model must distinguish three boundaries that are easy to conflate:

- CLI command parsing in `codex-cli` versus runtime orchestration in `codex-core`/`codex-tui`.
- App-server protocol types in `codex-app-server-protocol` versus internal application-server request processing in `codex-app-server`.
- Provider/API transport abstractions in `codex-api`/`codex-client` versus provider-neutral agent behavior in core crates.

## Trait documentation policy

`reference/trait-index.md` lists Traits that are relevant to a CLI capability or a shared boundary. For every listed Trait, record:

- exact Trait name;
- defining crate and Rust module path;
- visibility (`pub`, `pub(crate)`, or private);
- source file and snapshot line;
- the capability or common concern it supports;
- important implementations or consumers;
- whether it is an extension point, an internal lifecycle contract, or a rendering/test-only detail.

The index includes relevant internal Traits such as tool runtime and session task contracts when they explain behavior, while excluding unrelated presentation-only Traits. A command page links to the index rather than duplicating ownership facts.

## Source and evidence rules

- All paths and line numbers are resolved against the snapshot commit above.
- Source links use repository-relative paths and line anchors where practical.
- A statement about a Trait's owner must be supported by its actual definition, not only by a re-export.
- Re-exports are recorded separately when they affect the public API seen by a caller.
- Generated protocol/schema artifacts are described as generated outputs and linked to their generator entry point.
- The documents explicitly label inferred relationships when no single source file states the complete flow.

## Verification

Documentation work must be checked by:

- verifying every referenced path exists at the snapshot commit;
- checking the command list against the `Subcommand` enum and dispatch match in `crates/cli/src/main.rs`;
- checking each Trait entry with a repository search for its defining declaration;
- running `just check` as required by the repository instructions;
- inspecting `git diff --check` and the final diff.

No generated artifact is changed by this documentation snapshot, so `just generate` is not required unless the implementation unexpectedly touches a checked-in generated file.

## Future snapshots

When CLI behavior or the documented Trait boundaries change, create a new full-hash directory and keep the earlier snapshot intact. The root index of the new snapshot should identify the source commit and summarize meaningful differences from the previous documented snapshot.
