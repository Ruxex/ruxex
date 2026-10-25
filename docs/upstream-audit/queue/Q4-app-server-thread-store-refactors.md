# Q4 — App Server and thread-store refactors

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Response decoding/serialization, thread preparation, persistence, history projection, and lifecycle factoring |
| Source | Bundles B02 and B08; planning type: Refactor / structural alignment |
| Grouping | Only group behavior-preserving prerequisites needed by a later retained bundle |

Reconcile each candidate with the existing Ruxex architecture first. Ruxex
already has typed App Server request handling and a separated `LiveThread` /
`ThreadStore` persistence model, so textual upstream alignment is not a goal.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| `a83ba6124944...`, `b4b30dd5eb99...` | — | App Server unit/E2E | Waiting for reconciliation | Typed response decoding/JSON-RPC serialization; compare with existing Ruxex types |
| `b781733f958e...` | — | thread-store unit/E2E | Waiting for reconciliation | Thread preparation/persistence split; compare with `LiveThread` and `ThreadStore` |
