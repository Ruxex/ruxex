# Codex CLI architecture snapshot

Source snapshot: `c902263c127cddfea4a072f2c183040083426697`

This directory explains how capabilities reachable from `codex` enter the Rust workspace, which crates own each stage, and where shared Traits are defined. It is an implementation map for engineers, not a user manual and not a promise that source line numbers remain stable after this commit.

## Reading order

1. [System map](00-system-map.md) — the normalized end-to-end pipeline.
2. [CLI entrypoint](01-cli-entrypoint.md) — root parsing, validation, and dispatch.
3. [Command pages](commands/interactive.md) — one page per capability group.
4. [Common architecture](common/configuration.md) — shared behavior described once.
5. [Trait index](reference/trait-index.md) and [crate index](reference/crate-index.md) — ownership and extension points.
6. [Source map](reference/source-map.md) — snapshot paths and line anchors.

## Command groups

| CLI capability | Page |
| --- | --- |
| `codex` interactive TUI | [interactive](commands/interactive.md) |
| `codex agents` | [agents](commands/agents.md) |
| `codex exec` | [exec](commands/exec.md) |
| `codex review` | [review](commands/review.md) |
| `resume`, `queue`, `archive`, `delete`, `unarchive`, `fork` | [sessions](commands/sessions.md) |
| `mcp` | [MCP](commands/mcp.md) |
| `plugin` | [plugins](commands/plugin.md) |
| `app-server` | [app-server](commands/app-server.md) |
| `login`, `logout` | [authentication](commands/auth.md) |
| `doctor` | [diagnostics](commands/diagnostics.md) |
| `sandbox` | [sandbox](commands/sandbox.md) |
| `debug` | [debug](commands/debug.md) |
| `execpolicy check` | [policy](commands/policy.md) |
| `completion`, `features`, `exec-server`, internal transports | [utilities](commands/utility.md) |

## Common architecture

- [Configuration](common/configuration.md)
- [Authentication](common/authentication.md)
- [Provider and model](common/provider-and-model.md)
- [Session and thread](common/session-and-thread.md)
- [Tool execution](common/tool-execution.md)
- [Sandbox and permissions](common/sandbox-and-permissions.md)
- [Transport and server](common/transport-and-server.md)
- [Persistence and rollout](common/persistence-and-rollout.md)
- [Output and errors](common/output-and-errors.md)
- [Feature flags](common/feature-flags.md)
- [Telemetry](common/telemetry.md)

## References

- [Trait index](reference/trait-index.md)
- [Crate index](reference/crate-index.md)
- [Source map](reference/source-map.md)

## Snapshot rule

The source commit is part of the directory name. When CLI behavior or a documented Trait boundary changes, create another full-hash directory and leave this one intact. A source reference is authoritative for the pinned commit only.
