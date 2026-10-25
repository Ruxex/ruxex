# Q5 — Agent and runtime refactors

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Agent control, resume/fork flow, subagent spawning, and message-board storage factoring |
| Source | Bundle B03; planning type: Refactor / structural alignment |
| Grouping | One ownership/lifecycle boundary at a time |

Upstream `LocalAgentControl` is not present in Ruxex. Do not port that type
verbatim; adapt only retained behavior when a Ruxex-native boundary is needed.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| `e416eadf7f99...`, `c46aa58a7baa...` | — | agent/runtime unit + E2E | Waiting for reconciliation | Rollout budget and resume ownership; inspect Ruxex control flow first |
| `73bbb654b3b9...` | — | multi-agent E2E | Waiting for reconciliation | Shared spawning; adapt only if retained by Ruxex |
| `c026e7a622ecae45f0af1939729e7baab9d1637b` — Expose a backend-independent agent control contract (#46547) | — | audit only | Deferred | Ruxex has no alternate agent backend or retained App Server contract requiring this public abstraction; re-audit when a concrete backend boundary or consumer exists. Do not port upstream `AgentControl` verbatim. |
