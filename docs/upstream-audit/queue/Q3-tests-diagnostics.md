# Q3 — Shared tests and diagnostics

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Regression tests, E2E coverage, diagnostics, and retained test-fixture maintenance |
| Source | `import-overview.md` planning type: Test / diagnostics / maintenance (74) |
| Grouping | Keep coverage with the behavior it protects; group shared harness changes |

Provider matrices must cover both Chat Completions and Responses where the
behavior is exposed through both paths. App Server and TUI coverage use their
Ruxex targets.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| To be selected from `import-plan.md` | — | — | Ready per subsystem | Add Ruxex-specific boundaries when upstream tests do not apply directly |
