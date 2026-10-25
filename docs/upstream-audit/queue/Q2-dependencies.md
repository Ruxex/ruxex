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
