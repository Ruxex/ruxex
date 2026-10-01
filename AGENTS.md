# Project direction

This repository turns Codex into a general-purpose AI agent foundation by removing provider-specific features and assumptions.

The current main task is the staged migration from the Codex identity to **Genex**. Follow the naming-migration section in [`docs/provider-decoupling-plan.md`](docs/provider-decoupling-plan.md) as the primary plan: begin with low-dependency file, directory, and crate/package boundaries, then move through internal Rust symbols, and handle CLI/config, protocol/schema, persisted-data, and external-contract names only after producer/consumer audits.

Keep the shared architecture provider-neutral and suitable for building a wide range of AI agents.

Work directly on the `main` branch in this repository. Do not create Git worktrees for task work.

The earlier provider-decoupling and pre-release compatibility work remains the basis for this migration. The hosted config integration, `codex-git-attribution`, and the remote-memory startup rate-limit guard with the memory writer's direct `codex-backend-client` dependency have been removed. Maintain [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md): this is a pre-release fork with zero external users, so remove obsolete upstream, persisted-data, protocol, CLI/config, import, UI, and build-system compatibility layers only after auditing their producers and consumers. Do not remove current platform fallbacks, active in-repository clients, security behavior, or provider-neutral capabilities solely because they use `legacy` or `fallback` names.

For the current Phase 3 work, treat [`docs/provider-decoupling-candidates.md`](docs/provider-decoupling-candidates.md) as the primary work queue until further notice. Follow its stated inspection order, starting with unreferenced crates, source files, and generated artifacts. The entries are investigation candidates, not deletion decisions: audit producers and consumers first, then record an explicit `remove`, `retain`, or `migrate-in-place` decision in [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md) before changing code.

The immediate follow-up work is tracked in [`docs/superpowers/plans/2026-09-30-remove-chatgpt-hosted-surfaces.md`](docs/superpowers/plans/2026-09-30-remove-chatgpt-hosted-surfaces.md). It covers the planned removal or provider-neutralization of ChatGPT-specific update/install paths, desktop/security detection, and usage/billing error presentation.

While performing the Genex rename, if a remaining OpenAI-dependent capability, endpoint, authentication path, telemetry sink, hosted assumption, or provider-specific behavior is discovered, do not silently fold its removal into the naming change. Record the exact path and producer/consumer evidence in [`docs/provider-decoupling-candidates.md`](docs/provider-decoupling-candidates.md), add an explicit `remove`, `retain`, or `migrate-in-place` decision to [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md), and leave it as a separately traceable follow-up removal task.

Update [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md) in this order: persisted rollout/thread-history readers and migrations, protocol/app-server aliases and schemas, CLI/config/import compatibility, UI/observability/runtime shims, build-system/experimental scaffolding, then the final dependency/documentation audit. Each production match must have a producer/consumer audit and an explicit `remove`, `retain`, or `migrate-in-place` decision before code is changed.

Repository-wide verification policy for this `AGENTS.md`: standalone test execution is not the default verification for this cleanup work. Use the following commands as its test substitutes:

- Always run `just check` for the production workspace type-check.
- Run `just check --tests` whenever Rust code is added, removed, moved, or refactored, so test targets are compiled without executing them. This is required when test-only code or `#[cfg(test)]` imports may be affected.
- Run `just generate` whenever checked-in generated artifacts may be affected, and inspect the resulting diff.

For crate-removal work, test-first development is not required. Do not run standalone test commands for this cleanup unless explicitly requested.
