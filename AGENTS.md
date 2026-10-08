# Project direction

This repository turns Codex into a general-purpose AI agent foundation by removing provider-specific features and assumptions.

The current main task is the phased implementation of the single-provider Genex library described in [`docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`](docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md). The Genex identity migration remains a prerequisite and continues under the naming-migration section in [`docs/provider-decoupling-plan.md`](docs/provider-decoupling-plan.md), but new implementation work should prioritize the provider contract, runtime injection, provider-scoped data root, and removal of provider selection from thread execution.

The target architecture has one `ModelProvider` per Genex runtime/AppServer. Provider implementation type is selected by the host binary or library consumer and is not switched through runtime protocol. Multiple providers require separate runtimes/AppServers. A provider-specific data root (default `.genex`, with build-time and startup overrides) owns a non-secret marker containing the provider identity and protocol compatibility version; mounting a root with a mismatched marker must fail before opening its database or credentials. Threads and rollouts must not persist provider selection or credentials. Same-provider credential changes affect the shared runtime, while in-flight turns keep their credential snapshot and new turns after logout are rejected.

Implement this migration in the following phases. Do not combine all phases into one broad rewrite; each phase must leave a buildable, reviewable boundary and record any producer/consumer decision before changing existing contracts.

1. **Contract and boundary preparation**: audit the existing model-provider, auth, usage, model-catalog, app-server, thread, rollout, configuration, and data-root producers/consumers. Define provider-neutral request/response, stream event, error, capability, descriptor, auth-state, model-catalog, and usage types. Add the smallest fake provider and contract tests where practical. Record `remove`, `retain`, or `migrate-in-place` decisions in [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md) and candidate findings in [`docs/provider-decoupling-candidates.md`](docs/provider-decoupling-candidates.md).
2. **Provider trait and adapter**: introduce the broad `ModelProvider` contract for chat/streaming, model listing, capabilities, authentication lifecycle, and usage. Keep provider-specific HTTP, wire payloads, credentials, and error details behind an adapter. The core must consume provider-neutral types and must not require every provider to implement optional capabilities; unsupported operations return an explicit unsupported result.
3. **Single-provider runtime injection**: construct Genex runtime/AppServer with exactly one provider instance. Remove or bypass core responsibilities for provider registries, active-provider selection, provider switching, and provider fallback. Route chat, model listing, auth, usage, and capability requests through the injected provider while preserving existing provider-neutral agent, MCP, sandbox, and security behavior.
4. **Shared credential and account lifecycle**: connect login, credential update, auth state, usage, and logout to the injected provider. State is shared by all clients of the same AppServer. Credential updates apply to new turns; an in-flight turn uses its start-time credential snapshot. Logout rejects new turns and lets in-flight turns finish or end with an authentication error. Never persist credential values in threads or rollouts.
5. **Provider-scoped data root and mount marker**: make the data-root name configurable, with `.genex` as the default and an explicit precedence between builder, startup configuration, build-time default, and fallback. Add and validate `genex-home.json` before opening SQLite, sessions, caches, or credentials. New roots receive the current provider descriptor; existing roots with a missing or incompatible marker require explicit initialization or migration and must not be opened implicitly.
6. **Thread, rollout, and protocol persistence migration**: audit every reader and writer of persisted `model_provider` or equivalent fields, including resume, history, rollout, audit, protocol, and schema paths. Remove provider routing from thread execution and migrate or delete obsolete fields only after the audit. Replace provider list/select/switch APIs with current-provider status, capabilities, model catalog, auth, and usage APIs. Run generation and inspect all checked-in schema changes when applicable.
7. **Host surfaces and cleanup**: migrate CLI, TUI, app-server clients, configuration, fixtures, documentation, and UI from provider pickers to current-provider status and runtime configuration. Document separate runtime/AppServer startup for using another provider. Remove dead registry/switching code only after all producers and consumers are gone, then perform the final dependency and external-contract audit.

The completion gate for this main task is: a host can inject one provider; all clients sharing an AppServer observe the same provider and auth state; another provider requires another runtime/data root; mismatched data roots fail before sensitive state is opened; threads do not select providers; optional provider capabilities are explicit; and the existing OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers remain unchanged where they are external contracts.

Keep the shared architecture provider-neutral and suitable for building a wide range of AI agents.

Work directly on the `main` branch in this repository. Do not create Git worktrees for task work.

The earlier provider-decoupling and pre-release compatibility work remains the basis for this migration. The hosted config integration, `codex-git-attribution`, and the remote-memory startup rate-limit guard with the memory writer's direct `codex-backend-client` dependency have been removed. Maintain [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md): this is a pre-release fork with zero external users, so remove obsolete upstream, persisted-data, protocol, CLI/config, import, UI, and build-system compatibility layers only after auditing their producers and consumers. Do not remove current platform fallbacks, active in-repository clients, security behavior, or provider-neutral capabilities solely because they use `legacy` or `fallback` names.

For the Genex migration, assume this tool is unreleased and that its server and client surfaces are almost entirely contained within this repository. Repository-owned API, protocol, schema, persisted-data, CLI/configuration, package, and UI names may therefore be migrated together after their in-repository producers and consumers are audited; compatibility with existing external users is not a requirement. The sole substantive runtime boundary outside this repository is the LLM endpoint. Preserve names that are required by the OpenAI endpoint itself, including its URLs, request/response fields, authentication variables or headers, and model identifiers. Also preserve identifiers belonging to the external OpenAI fork or upstream repository, third-party protocols/packages, and historical attribution or legal records.

For provider-decoupling cleanup that is outside the main single-provider implementation, use [`docs/provider-decoupling-candidates.md`](docs/provider-decoupling-candidates.md) as the work queue and follow its stated inspection order, starting with unreferenced crates, source files, and generated artifacts. These entries are investigation candidates, not deletion decisions: audit producers and consumers first, then record an explicit `remove`, `retain`, or `migrate-in-place` decision in [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md) before changing code. The phased single-provider implementation above takes priority over this cleanup queue unless a dependency or safety audit makes the cleanup a prerequisite.

The immediate follow-up work is tracked in [`docs/superpowers/plans/2026-09-30-remove-chatgpt-hosted-surfaces.md`](docs/superpowers/plans/2026-09-30-remove-chatgpt-hosted-surfaces.md). It covers the planned removal or provider-neutralization of ChatGPT-specific update/install paths, desktop/security detection, and usage/billing error presentation.

While performing the Genex rename, if a remaining OpenAI-dependent capability, endpoint, authentication path, telemetry sink, hosted assumption, or provider-specific behavior is discovered, do not silently fold its removal into the naming change. Record the exact path and producer/consumer evidence in [`docs/provider-decoupling-candidates.md`](docs/provider-decoupling-candidates.md), add an explicit `remove`, `retain`, or `migrate-in-place` decision to [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md), and leave it as a separately traceable follow-up removal task.

Update [`docs/pre-release-compatibility-inventory.md`](docs/pre-release-compatibility-inventory.md) in this order: persisted rollout/thread-history readers and migrations, protocol/app-server aliases and schemas, CLI/config/import compatibility, UI/observability/runtime shims, build-system/experimental scaffolding, then the final dependency/documentation audit. Each production match must have a producer/consumer audit and an explicit `remove`, `retain`, or `migrate-in-place` decision before code is changed.

Repository-wide verification policy for this `AGENTS.md`: standalone test execution is not the default verification for this cleanup work. Use the following commands as its test substitutes:

- Always run `just check` for the production workspace type-check.
- Run `just check --tests` whenever Rust code is added, removed, moved, or refactored, so test targets are compiled without executing them. This is required when test-only code or `#[cfg(test)]` imports may be affected.
- Run `just generate` whenever checked-in generated artifacts may be affected, and inspect the resulting diff.

For crate-removal work, test-first development is not required. Do not run standalone test commands for this cleanup unless explicitly requested.

## Rust/Cargo memory guardrails for this repository

Rust commands that may compile or link must be run through the repository's `just` recipes and a user systemd scope. This applies to `just test`, `just check`, `just check --tests`, and any recipe or command that starts Cargo, `rustc`, a linker, or a build script.

Use `CARGO_BUILD_JOBS=4` for ordinary crate-local work. For the full workspace or large targets such as `genex-cli`, use `CARGO_BUILD_JOBS=2`. Keep nextest execution concurrency separate from Cargo build concurrency; use `--test-threads=4` when test-process memory is the limiting factor.

Use these limits for this 32 GiB Linux desktop:

```sh
systemd-run --user --scope --collect --quiet \
  -p MemoryHigh=16G \
  -p MemoryMax=18G \
  -p MemorySwapMax=4G \
  --setenv=CARGO_BUILD_JOBS=2 \
  just test -p <crate>
```

For ordinary crate-local checks or tests, the same scope may use `CARGO_BUILD_JOBS=4`. Do not run the repository's Rust build, check, or test commands directly without this memory-limited scope. If the scope reaches its limit, inspect `memory.peak`, `memory.events`, and the user journal before lowering the job count further or changing compiler profiles.
