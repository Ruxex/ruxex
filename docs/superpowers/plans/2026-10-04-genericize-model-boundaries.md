# Generic Model Boundary Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove OpenAI/Codex built-in model assumptions from the Genex runtime while preserving model metadata supplied by the active catalog and the external provider integrations that require their own identifiers.

**Architecture:** Model selection and rate-limit presentation will consume catalog and server metadata rather than matching provider-specific model slugs. The TUI will expose all picker-visible catalog models through one generic path, use a generic advanced-reasoning warning, and use the server-advertised normal model for rate-limit nudges. Repository-owned default rate-limit state will use a provider-neutral `default` bucket identifier.

**Tech Stack:** Rust workspace, serde/json-schema configuration types, TUI model catalog, protocol rate-limit snapshots, checked-in generated schemas.

**Spec:** `AGENTS.md`, `docs/provider-decoupling-plan.md`, `docs/provider-decoupling-candidates.md`, `docs/pre-release-compatibility-inventory.md`.

**Constraints:** Work directly on `main`. Do not create a worktree. Do not run standalone tests; use `just check` and `just check --tests` as required by the repository instructions. Run `just generate` after configuration/schema changes and inspect its diff.

## Task 1: Record the audit and migration decisions

- [x] Add a dated entry to the provider-decoupling candidate queue and compatibility inventory describing the producer/consumer audit for model picker branches, model migration notice keys, rate-limit bucket identity, warnings, and status previews.
- [x] Mark OpenAI built-in model fixtures and stale migration notice keys as `remove` or `migrate-in-place`.
- [x] Explicitly retain external provider model IDs and security/product/GitHub identifiers that are not repository-owned model assumptions.

## Task 2: Remove provider-specific model picker branches

- [x] Update picker tests/fixtures first so the expected behavior is catalog-driven and no longer recognizes `codex-auto-*` or `gpt-*` model prefixes.
- [x] Remove the quick auto-model partition/order and route the model popup directly to the catalog-backed all-model picker.
- [x] Remove auto-model-specific reasoning-label and shortcut branches.
- [x] Remove the Luna/reserve built-in model display and selection special cases from the TUI.
- [x] Replace OpenAI model IDs in Rust test fixtures with neutral model IDs.

## Task 3: Genericize warnings, rate limits, and status previews

- [x] Make the advanced reasoning warning apply from advertised reasoning capability, with provider-neutral wording and no OpenAI/plan-specific model matching.
- [x] Replace the repository-owned default rate-limit bucket identifier `codex` with `default` across parser, core state merge, TUI state, status preview, and fixtures.
- [x] Use a server-advertised `normal_model_slug` when offering a lower-usage model switch; do not hardcode a built-in model slug or reserve model.
- [x] Replace hardcoded OpenAI model strings in status previews and protocol fixtures with neutral examples.

## Task 4: Remove stale OpenAI migration notice compatibility

- [x] Remove the old GPT/Codex migration notice constants, configuration fields, imports, and lookup branches.
- [x] Preserve the generic catalog-driven `ModelUpgrade` migration flow and its per-model acknowledgement map.
- [x] Regenerate checked-in schemas and inspect the generated diff.

## Task 5: Verify and review residuals

- [x] Run `just generate` and inspect generated changes.
- [x] Run `just check --tests` and `just check`.
- [x] Run the Genex Rust-name audit and focused residual searches for provider-specific model assumptions.
- [x] Run `git diff --check` and review the final diff for unrelated changes.
