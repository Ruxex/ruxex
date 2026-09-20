# Project direction

This repository turns Codex into a general-purpose AI agent foundation by removing OpenAI-specific features and assumptions.

Keep the shared architecture provider-neutral and suitable for building a wide range of AI agents.

Work directly on the `main` branch in this repository. Do not create Git worktrees for task work.

Current work is Level 1 Phase 3 of `docs/openai-decoupling-plan.md`. The hosted config integration, `codex-git-attribution`, and the remote-memory startup rate-limit guard with the memory writer's direct `codex-backend-client` dependency have been removed. Before continuing the remaining OpenAI-specific integrations, execute `docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md` and maintain `docs/pre-release-compatibility-inventory.md`: this is a pre-release fork with zero external users, so remove obsolete upstream, persisted-data, protocol, CLI/config, import, and UI compatibility layers first. Do not remove current platform fallbacks, active in-repository clients, security behavior, or provider-neutral capabilities solely because they use `legacy` or `fallback` names. Do not mark Phase 3 complete until the compatibility cleanup and the phase's remaining integrations have been handled.

Update `docs/pre-release-compatibility-inventory.md` in this order: persisted rollout/thread-history readers and migrations, protocol/app-server aliases and schemas, CLI/config/import compatibility, then UI/observability/runtime shims. Each production match must have a producer/consumer audit and an explicit `remove`, `retain`, or `migrate-in-place` decision before code is changed.

For crate-removal work in this phase, test-first development is not required. Treat `cargo check` (and the repository-wide `just check` when applicable) as the primary verification instead of adding deletion-specific tests first.

Do not run tests for this cleanup; use `just check` instead.
