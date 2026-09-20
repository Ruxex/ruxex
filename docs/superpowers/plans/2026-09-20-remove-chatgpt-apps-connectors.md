# ChatGPT Apps / Connectors Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove ChatGPT Apps, connector directory, and host-owned Apps MCP paths while retaining provider-neutral MCP and plugin capabilities.

**Architecture:** Delete the dedicated connector crates and remove their protocol, configuration, core, app-server, and TUI consumers. In `codex-mcp`, remove only Apps/OpenAI-specific adapters and preserve standard MCP runtime, generic OAuth/elicitation, and tool metadata.

**Tech Stack:** Rust workspace, Cargo manifests, app-server Rust protocol and generated JSON/TypeScript schemas, CLI/TUI/core MCP integration.

**Spec:** `docs/superpowers/specs/2026-09-20-remove-chatgpt-apps-connectors-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- This is a pre-release fork with zero external users; obsolete Apps protocol and persisted-data readers may be removed.
- Do not delete existing cache files automatically.
- Preserve standard MCP, generic OAuth/elicitation, provider-neutral tool metadata, generic `app://` parsing, local plugins, platform fallbacks, and security behavior.
- Use `cargo check`, generated-artifact verification, and `just check`; do not run tests for this cleanup.
- Update `docs/pre-release-compatibility-inventory.md` with exact producer/consumer decisions.

## Review Focus

- No active non-Apps consumer remains for `codex-connectors` or its extension.
- App-server protocol schemas no longer publish Apps RPCs while ordinary MCP protocol remains.
- ChatGPT/OpenAI Apps metadata/auth/file handling is removed without removing standard MCP OAuth/elicitation.
- Core and TUI no longer expose Apps-only settings, tool routing, or auth UI while generic mentions remain.
- Workspace manifests and generated artifacts contain no orphaned Apps crate or RPC references.

## Tasks

### Task 1: Remove dedicated connector crates and workspace edges

- [x] Remove `crates/connectors` and `crates/ext/connectors` from workspace manifests and dependency declarations.
- [x] Remove connector-only plugin loading and connector imports from core-plugins, app-server, core, TUI, and MCP manifests/code.
- [x] Remove connector-only tests, fixtures, and documentation references.
- [x] Verify no production dependency or module references remain.

### Task 2: Remove Apps protocol and app-server surface

- [x] Remove Apps request/response types and exports from `app-server-protocol`.
- [x] Remove Apps request processors, dispatch, tests, and app-server client routes.
- [x] Regenerate JSON/TypeScript schemas and inspect the generated diff.

### Task 3: Remove Apps configuration and core/TUI behavior

- [x] Remove Apps/Psp feature flags, Apps config fields, Apps instructions, and Apps-only core MCP/tool routing.
- [x] Remove TUI Apps settings, connector discovery/mentions, auth-link rendering, and Apps snapshots.
- [x] Retain generic MCP lifecycle, tool metadata, standard elicitation, and `app://` parsing used by non-Apps integrations.

### Task 4: Remove OpenAI-specific MCP adapters and finalize inventory

- [x] Remove `codex_apps`, OpenAI `fileParams`, Apps source-attribution, and ChatGPT MCP auth adapters while retaining generic MCP OAuth/elicitation.
- [x] Update compatibility inventory and decoupling plan status for this capability.
- [x] Run broad searches, `git diff --check`, affected `cargo check`, generated-artifact verification, and `just check`.
