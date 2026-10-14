# Ruxex OSS Publication Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish the current Codex-derived Rust agent foundation as Ruxex with a reviewable public `main`, preserved history, explicit provenance, provider-neutral runtime boundaries, and reproducible Linux/macOS builds.

**Architecture:** Continue work on `develop` while retaining `origin` as the historical `edge-m` remote. The public `Ruxex/ruxex` repository receives the working branch first; after the publication gate passes, the cleaned result becomes `main`, the local `origin` is changed to Ruxex, and temporary branches/remotes are removed only after verification.

**Tech Stack:** Rust workspace, Cargo, `just`, GitHub Actions, `rusty_v8`, Git, GitHub, Apache-2.0 and third-party attribution files.

**Spec:** [`docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`](../specs/2026-10-08-single-model-provider-library-design.md), [`docs/provider-decoupling-plan.md`](../../provider-decoupling-plan.md), and the repository `AGENTS.md` publication constraints.

## Global Constraints

- Work directly on local `develop`; do not create a worktree.
- Preserve the complete existing Git history; do not rewrite history or force-push.
- Remove the development-only `.codex`, `.vscode`, `.superpowers`, and unused `scripts` content as the final filesystem cleanup before commits are consolidated. Remove `docs/` except for this active publication plan and the current compatibility/upstream audit ledgers; extract any legally or operationally required text into retained root files first.
- Preserve OpenAI endpoint URLs, request/response fields, authentication headers/variables, and model identifiers when they are external contracts.
- Keep third-party, upstream, historical, and legal attribution identifiers where required; remove repository-owned OpenAI branding and assumptions only after producer/consumer auditing.
- Run `just check`, `just check --tests` after Rust changes, and `just generate` when generated artifacts may be affected, using the repository's memory-limited systemd scope.

## Review Focus

- Public HEAD contains no internal design material, credentials, private endpoints, or generated build cache, except for the explicitly retained publication plan and audit ledgers needed to continue this release work.
- Rename coverage distinguishes repository-owned `genex`/Codex identifiers from external contracts, third-party packages, historical records, and legal notices.
- Every OpenAI-derived dependency and fork retains the attribution and license obligations needed for redistribution.
- A clean consumer can build `rusty_v8` through supported official artifacts on Linux and macOS without receiving a Genex/Ruxex binary.
- The upstream commit ledger records every imported, skipped, and partially imported commit with a reason and verification reference.

### Task 1: Freeze the publication baseline

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`
- Create: `docs/upstream-commit-audit-2026-10-14.md` if the existing ledger needs a new dated section

- [x] Record the current `origin/main`, `ruxex/develop`, local `main`, and local `develop` commit IDs and remote roles.
- [x] Define the public-tree allowlist: runtime/source, root-level user-facing text, legal notices, third-party licenses, reproducibility metadata, and required examples. Treat the development `docs/` tree as a final removal target.
- [x] Classify each internal design document, inspection artifact, audit log, and generated artifact as `retain`, `move-to-private-record`, or `remove` before deleting anything.
- [x] Record each decision in the compatibility inventory before changing the corresponding files.
- [x] Commit the baseline decision separately and push it to `ruxex/develop`.

### Task 2: Confirm Ruxex legal and package identity

**Files:**
- Modify: `README.md`
- Modify: `LICENSE`
- Create or modify: `NOTICE`, `CREDITS.md`, or the repository's selected attribution file
- Modify: root `Cargo.toml` and all affected `crates/*/Cargo.toml`
- Modify: package metadata and user-facing documentation identified by the name audit

- [x] Record preliminary Ruxex trademark/domain/package-name checks outside the code rename; leave formal legal clearance as an explicit publication gate.
- [x] Decide the exact copyright notice for new Ruxex-authored changes and retain required upstream attribution.
- [x] State plainly that Ruxex is derived from/forked from Codex, without presenting Ruxex as an OpenAI product.
- [x] Audit every package name, Rust crate name, binary name, environment variable, data-root marker, protocol/schema identifier, fixture, and documentation link.
- [x] Rename repository-owned identifiers in a controlled wave; preserve external wire identifiers and historical/legal records in the inventory.
- [x] Run the repository name audit and inspect all remaining matches manually.
- [x] Run `just check`, `just check --tests`, `just generate`, and `git diff --check`.

### Task 3: Migrate OpenAI-derived library forks

> User ruling: defer creation of the Ruxex mirrors and Cargo URL migration.
> Keep the current public `openai-oss-forks` pins until a later dependency
> migration pass; preserve this audit and provenance record in the meantime.

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: fork-related documentation and attribution files
- Create or update: the Ruxex-owned forks of `crossterm`, `tokio-tungstenite`, and `tungstenite-rs`

- [ ] Create or confirm Ruxex organization forks with preserved upstream history and licenses.
- [ ] Remove `openai-oss-forks` repository URLs from active dependency declarations while preserving upstream provenance in a dedicated attribution record.
- [ ] Audit patches, commit ranges, licenses, notices, and security-sensitive changes in each fork.
- [ ] Update Cargo sources and regenerate the lockfile without changing unrelated dependency resolution.
- [ ] Verify the workspace does not depend on a private or unavailable fork URL.
- [ ] Run `just check`, `just check --tests`, and inspect the lockfile diff.

### Task 4: Resolve Rusty V8 artifact ownership

> Current ruling: use the OpenAI/Codex sandbox-enabled artifact as the sole
> supported prebuilt V8 input for Ruxex. The pinned official release does not
> publish a prebuilt archive for `v8_enable_sandbox`; do not silently use an
> ordinary shared-cage archive or disable the feature. Ruxex ownership of the
> artifact remains a later release task.

**Files:**
- Modify: `scripts/download-rusty-v8.sh`
- Modify: `third_party/v8/README.md`
- Modify: `third_party/v8/*.sha256` and release metadata if required
- Modify: `Cargo.toml`, `justfile`, and relevant build documentation

- [x] Confirm the pinned `rusty_v8` version and the exact Linux/macOS targets required by Ruxex.
- [x] Use the sandbox-enabled artifact required by Ruxex; the OpenAI/Codex release remains the explicitly documented source until Ruxex takes ownership.
- [x] Keep checksum verification and sandbox/ptrcomp variant selection explicit.
- [x] Document that Ruxex is a library/workspace and does not distribute a V8 binary; consumers build through the crate's supported artifact path.
- [ ] Test a clean artifact fetch/build path on Linux and macOS, including cache miss and cache hit behavior.
- [x] Record unsupported targets and source-build instructions without making them mandatory for ordinary consumers.

### Task 5: Add repository-owned CI and release hygiene

**Files:**
- Create: `.github/workflows/ci.yml`
- Create or modify: `.github/CODEOWNERS`, issue templates, `SECURITY.md`, and contribution documentation
- Modify: `justfile` or CI helper scripts when needed

- [x] Add pull-request and `main`-push jobs for Linux and macOS covering Cargo check, build, and test targets through the supported repository recipes.
- [x] Pin toolchain and action versions, configure Rusty V8 caching, and avoid exposing credentials in logs.
- [x] Ensure CI can build from a clean checkout without the old OpenAI artifact release or private organization access for ordinary Rusty V8 artifacts.
- [x] Replace Codex-specific issue templates, ownership paths, links, and support language that are repository-owned.
- [x] Add a public contribution/security policy and state the supported platforms and provider-host model.
- [ ] Validate workflow syntax and perform at least one successful run for pull-request-equivalent and `main`-equivalent events.

Implementation note (2026-10-14): `.github/workflows/ci.yml` is now present and
the `just` recipes default to direct `cargo`, with `RUXEX_CARGO_RUNNER` retained
as an optional local override. Local recipe checks and a memory-limited
workspace build passed; a hosted GitHub Actions run still requires a pull
request or a push to `main`. Public governance files were migrated to a root
`CONTRIBUTING.md`, a Ruxex security policy, generic issue forms, and an
unconfigured CODEOWNERS placeholder because no maintainer team handle has been
verified yet.

### Task 6: Finalize upstream synchronization policy

**Files:**
- Create: `docs/upstream-synchronization-policy-2026-10-14.md`
- Modify: `docs/upstream-commit-audit-2026-10-07.md` and retain the historical commit list
- Create: a small review checklist or script only if it reduces repeated commit-audit errors

- [x] Define the upstream remote/ref and the synchronization cadence.
- [x] Define the per-commit ledger fields and record the initial zero-candidate checkpoint; future candidates must record commit ID, subject, decision (`import`, `skip`, or `partial`), reason, affected Ruxex boundary, and verification result.
- [x] Require review in dependency order where later commits depend on earlier changes.
- [x] Keep provider-specific, hosted, licensing, and security-sensitive changes as explicit audit entries rather than silently importing them.
- [ ] Verify each accepted batch with the repository check policy and push the audit together with the code it explains.

Implementation note (2026-10-14): `upstream/main` is currently identical to the
shared `main` checkpoint, so this cycle has no upstream code to import. The
active policy is recorded separately from the historical OpenAI-based audit.

### Task 7: Prepare the public tree and publication candidate

**Files:**
- Modify: repository root and `docs/` according to Task 1's allowlist
- Modify: `README.md`, `CHANGELOG.md`, package metadata, and release documentation
- Modify: generated schemas and checked-in artifacts only when their producers require it

- [x] Before removing development materials, extract the minimum required API/setup/legal/provenance text into retained root files such as `README.md`, `LICENSE`, `NOTICE`, `SECURITY.md`, or `CONTRIBUTING.md`.
- [x] As the final filesystem cleanup before commit consolidation, remove `.codex/`, `.vscode/`, `.superpowers/`, and scripts classified as unused. Remove `docs/` except for the active publication plan and compatibility/upstream audit ledgers; keep only scripts required by build, verification, artifact resolution, or release operations.
- [x] Do not delete those development materials during earlier implementation tasks; they remain available for audits and verification until this step.
- [x] Remove caches, local paths, private organization references, stale screenshots, and unused build scaffolding.
- [x] Verify that examples, README commands, crate names, data-root names, and CI URLs all describe Ruxex consistently.
- [x] Run the local repository verification gate: `just check`, `just check --tests`, `just generate`, and `git diff --check`.
- [ ] Confirm the hosted Linux/macOS CI matrix is green.
- [x] Review the complete staged diff and the clean post-commit checkout of the candidate.
- [x] Commit the candidate on `develop` and push it to `Ruxex/ruxex:develop`.

### Task 8: Promote and close the temporary branch

**Files:**
- Git refs: `Ruxex/ruxex` `main` and `develop`
- Modify: local Git remote configuration

- [ ] Fast-forward or push the reviewed candidate to `Ruxex/ruxex:main` without rewriting history.
- [ ] Verify the public repository, branch protection, Actions, license display, README rendering, and source archive.
- [ ] Change the local `origin` from `edge-m/codex-fork` to `Ruxex/ruxex` only after the public `main` verification succeeds.
- [ ] Remove the temporary local `ruxex` remote only if `origin` now points to Ruxex and the remote refs are verified.
- [ ] Delete the temporary public `develop` branch only after the final `main` commit and tag/rollback reference are recorded.
- [ ] Record the final repository URLs, commit IDs, release decision, and remaining follow-up items.

## Verification Gate

The publication is complete only when the Ruxex `main` branch has a clean public tree, the history is preserved, the name/legal/provenance audit is complete, the OpenAI-derived forks are accounted for, official Rusty V8 artifact resolution works on Linux and macOS, CI is green for pull requests and `main`, and the upstream ledger is ready for the next synchronization cycle.
