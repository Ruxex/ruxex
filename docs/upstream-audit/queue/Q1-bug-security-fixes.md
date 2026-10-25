# Q1 — Independent bug and security fixes

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Retained correctness, crash, data-loss, auth, authorization, and robustness fixes |
| Source | `import-overview.md` planning type: Bug / security / robustness (397) |
| Grouping | One upstream fix per Ruxex commit; group only for a required shared prerequisite |

Each item is inspected against the current Ruxex tree before implementation.
Production changes are manually adapted after tests are prepared; upstream
commits are never cherry-picked.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| `5e96aabd68dc6194f5ad928cb7f5d0bc71b72df9` — Avoid full-string scans in token-budget truncation (#49712) | N/A | Boundary regression test passes before production changes | No production adaptation required | Ruxex already preserves the tested UTF-8 behavior. The remaining upstream difference is an unobserved performance optimization, so no production change is justified under the test-first rule. |
| `67352b5b4adc3d3e8865ac8bf343b952b8c040e7` — Recover exec-server sessions after environment info timeouts (#49407) | Pending | `ruxex-exec-server` client recovery test | In progress | Ruxex has the same timeout-prone environment-info probe and recovery state machine. The timeout must retire the probed transport and request recovery. |
