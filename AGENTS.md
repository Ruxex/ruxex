# Project direction

This repository turns Codex into a general-purpose AI agent foundation by removing OpenAI-specific features and assumptions.

Keep the shared architecture provider-neutral and suitable for building a wide range of AI agents.

Work directly on the `main` branch in this repository. Do not create Git worktrees for task work.

Current work is Level 1 Phase 3 of [`docs/openai-decoupling-plan.md`](docs/openai-decoupling-plan.md). The hosted config integration, `codex-git-attribution`, and the remote-memory startup rate-limit guard with the memory writer's direct `codex-backend-client` dependency have been removed. Before continuing the remaining OpenAI-specific integrations, execute [`docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md`](docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md) and maintain [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md): this is a pre-release fork with zero external users, so remove obsolete upstream, persisted-data, protocol, CLI/config, import, UI, and build-system compatibility layers first. Do not remove current platform fallbacks, active in-repository clients, security behavior, or provider-neutral capabilities solely because they use `legacy` or `fallback` names. Do not mark Phase 3 complete until the compatibility cleanup and the phase's remaining integrations have been handled.

For the current Phase 3 work, treat [`docs/openai-decoupling-candidates.md`](docs/openai-decoupling-candidates.md) as the primary work queue until further notice. Follow its stated inspection order, starting with unreferenced crates, source files, and generated artifacts. The entries are investigation candidates, not deletion decisions: audit producers and consumers first, then record an explicit `remove`, `retain`, or `migrate-in-place` decision in [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md) before changing code.

Update [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md) in this order: persisted rollout/thread-history readers and migrations, protocol/app-server aliases and schemas, CLI/config/import compatibility, UI/observability/runtime shims, build-system/experimental scaffolding, then the final dependency/documentation audit. Each production match must have a producer/consumer audit and an explicit `remove`, `retain`, or `migrate-in-place` decision before code is changed.

Repository-wide verification policy for this `AGENTS.md`: standalone test execution is not the default verification for this cleanup work. Use the following commands as its test substitutes:

- Always run `just check` for the production workspace type-check.
- Run `just check --tests` whenever Rust code is added, removed, moved, or refactored, so test targets are compiled without executing them. This is required when test-only code or `#[cfg(test)]` imports may be affected.
- Run `just generate` whenever checked-in generated artifacts may be affected, and inspect the resulting diff.

For crate-removal work, test-first development is not required. Do not run standalone test commands for this cleanup unless explicitly requested.
