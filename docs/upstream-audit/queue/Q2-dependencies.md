# Q2 — Compatible dependencies

| Field | Value |
| --- | --- |
| Status | In progress |
| Scope | Non-member dependency upgrades and compatible lockfile maintenance |
| Source | `import-overview.md` planning type: Dependency / maintenance (23) |
| Grouping | Group one dependency family or lockfile transition; keep unrelated updates separate |

Member-crate updates, major/API-breaking changes, and build-impacting changes
remain review-gated. Unused dependencies are excluded.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| To be selected from `import-plan.md` | — | — | Ready per family | Confirm Ruxex uses the dependency and verify the affected workspace |
| `c9b3924a623fbe5ac932ed9b898ad18eeed665dd` — Use the rmcp SDK for enterprise-managed token exchanges (#49473) | `f7d696ee0` | `ruxex-rmcp-client`: 216 passed; 1 ignored | Implemented | Upgraded `rmcp` to the pinned 3.3.0 SDK revision with `auth-enterprise-managed`; manually retained Ruxex routing and error policy, with no cherry-pick. |
| `3627bb91b2f14f47c1a415412ec2f1e132e50647` — Avoid building a host SQLite driver for SQLx macros (#47084) | `a703c9fb6` | `ruxex-state`/`ruxex-thread-store`: check passed; 322 tests, 1 unrelated flaky retry | Implemented | Fixed SQLx at `=0.9.0`, separated `sqlx-macros` and `sqlx-sqlite` from runtime SQLx features, and adapted retained derives/migration embedding. No cherry-pick. |
| `09bb3d694223ef8a11b1c3ff6e87042dc0c5dbbb` — Disable unused default dependency features (#47088) | `73999712e` | `ruxex-tui`: check passed; 1082 tests passed, 2 skipped; `just fmt-check` | Implemented | Applied only the retained `syntect`/`two-face` feature selection. The upstream `sentry`, feedback, Bazel, and unrelated lockfile cleanup is not applicable to Ruxex. |
