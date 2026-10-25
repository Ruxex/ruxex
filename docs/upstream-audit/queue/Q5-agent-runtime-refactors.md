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
| `e416eadf7f990d96bc1b538c02ddcdb062d5542c` — Encapsulate rollout budget accounting in `LocalAgentControl` (#46559) | `5a06c6c17` | `just check --package ruxex-core`; `just app-server-e2e`: 99/99 passed, 2 skipped; `just fmt-check` | Implemented | Encapsulated retained shared rollout-budget usage and reminder acknowledgment behind the Ruxex-local controller. The upstream Guardian-specific input-budget caller was not applicable. |
| `87bc50f9d423a62617addd5721f8bd7c54f26bc5` — Cache OS discovery for user agents and telemetry (#47038) | `8b5a9b6b2` | `just test --package ruxex-login --lib`: 8 passed; `just test --package ruxex-otel --lib`: 23 passed, 1 skipped; `just check --package ruxex-login --package ruxex-otel`; `just fmt-check` | Implemented | Cached retained login and OTEL OS discovery while keeping mutable User-Agent overrides dynamic. The upstream analytics runtime-metadata path has no Ruxex equivalent and was excluded. |
| `16bb1b9d7e21f73d67f20aacc3a7c37d80cde352` — Implement `AgentControl` for `LocalAgentControl` (#47271) | — | Audit only | Deferred | Ruxex has no alternate agent backend or retained App Server contract requiring this backend-neutral abstraction. Revisit when a second backend or concrete consumer exists; do not port the upstream trait verbatim. |
| `2c087a0cc2ef1d731086a1d0cb0081bdd792cc4d` — Fix Escape navigation in read-only agent sessions (#47320) | `0a30e1a7e` | `just check --package ruxex-tui`; `just tui-e2e`: 16 passed, 4 skipped; `just fmt-check` | Implemented | Prevented read-only Esc from entering backtracking, safely closes the transcript overlay, and opens a stale agent overview while offline without refresh. Ruxex has no retained transcript search/selection state from the upstream path. |
