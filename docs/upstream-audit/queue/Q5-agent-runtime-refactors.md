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
| `7a15548c6a5a54c5af9dcb4b5e9cd89387cd89f0` — Move child completion routing into the agent controller (#46552) | `da77ed5b6` | Agent-related `ruxex-core` lib tests: 51/51; `just check --package ruxex-core`; `just app-server-e2e`: 99/99 passed, 2 skipped; `just fmt` | Implemented | Moved retained terminal child-result routing into the existing Ruxex-local controller without introducing the upstream backend-independent public contract. |
| `c6f5d9e9b5d9ab8085ad5d3a7e28717d3a6a19dc` — Return `LiveAgent` records from agent listing (#46553) | `8a7dceaf9` | `ruxex-core` list-agents schema tests: 2/2; `just check --package ruxex-core`; `just app-server-e2e`: 99/99 passed, 2 skipped; `just fmt-check` | Implemented | Returned the existing Ruxex `LiveAgent` records from the controller and kept conversion to the unchanged tool response at the handler boundary, including the path-less thread ID fallback. |
