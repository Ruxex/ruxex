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
| `5e96aabd68dc6194f5ad928cb7f5d0bc71b72df9` — UTF-8 token truncation boundaries | Pending | `ruxex-utils-string` focused nextest | In progress | Ruxex already had the behavior; retain the upstream regression coverage as a Ruxex utility test |
