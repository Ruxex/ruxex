# ChatGPT Apps / Connectors Removal Design

## Goal

Remove the ChatGPT-owned Apps, connector directory, and host-owned Apps MCP
capability from the pre-release provider-neutral fork while preserving standard
MCP, generic OAuth/elicitation, local plugin metadata, and the generic `app://`
mention syntax.

## Boundary

The removal covers the ChatGPT connector directory and cache, the
`codex-connectors` and dedicated connector extension crates, Apps app-server
protocol/request processors, Apps configuration and feature flags, Apps-only
core/TUI behavior, and provider-specific MCP metadata/auth/file-parameter
adapters. Existing ordinary MCP runtime, standard OAuth/elicitation, generic
tool metadata, plugin loading, and non-Apps `app://` parsing remain.

## Compatibility decision

This is a pre-release fork with no external users. Existing connector/App
cache files are not deleted automatically; their readers and runtime paths are
removed. The compatibility inventory records each production boundary as
`remove` or `retain` with its producer/consumer evidence and verification.

## Verification

Use repository-wide `rg` searches, generated app-server schema verification,
`git diff --check`, `cargo check` for affected crates, and `just check`.
Per repository instructions, do not add or run tests for this crate-removal
cleanup.
