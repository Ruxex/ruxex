# Q4 — App Server and thread-store refactors

| Field | Value |
| --- | --- |
| Status | In progress |
| Scope | Response decoding/serialization, thread preparation, persistence, history projection, and lifecycle factoring |
| Source | Bundles B02 and B08; planning type: Refactor / structural alignment |
| Grouping | Only group behavior-preserving prerequisites needed by a later retained bundle |

Reconcile each candidate with the existing Ruxex architecture first. Ruxex
already has typed App Server request handling and a separated `LiveThread` /
`ThreadStore` persistence model, so textual upstream alignment is not a goal.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| `a83ba6124944...`, `b4b30dd5eb99...` | — | App Server unit/E2E | Waiting for reconciliation | Typed response decoding/JSON-RPC serialization; compare with existing Ruxex types |
| `429afbcaa97a9397e59976afdc0c38a15db64956` — Extract per-thread enrichment into a dedicated async helper (#47083) | `a7c6427a2` | `ruxex-app-server`: 102/102 passed; `just fmt-check` | Implemented | Extracted per-thread status/settings/direct-input enrichment into a dedicated async helper while preserving concurrent `join_all` behavior. No cherry-pick. |
| `b781733f958e...` | — | thread-store unit/E2E | Waiting for reconciliation | Thread preparation/persistence split; compare with `LiveThread` and `ThreadStore` |
| `9ef9cb1d9fc6013f6c1994346e0ee93ad9e6f986` — Complete turn abort callbacks before emitting terminal events (#49475) | `504c52db1` | Focused abort lifecycle integration test; `just check --package ruxex-core --package ruxex-extension-api`; `just fmt-check` | Implemented | Moved abort lifecycle delivery into task-abort handling immediately before `TurnAborted`, covered already-cancelled tasks, removed duplicate outer delivery, and documented the completion-before-terminal-event contract. |
| `eefe0ce1a8e39788a4de7ef5bafd47e51339610b` — Bind the app-server time provider to a local variable (#49411) | `0fc7efc10` | `just check --package ruxex-app-server`; `just fmt-check` | Implemented | Followed the retained App Server ownership/lifetime structure by binding the existing time provider before injecting it into the thread manager; behavior and provider boundaries are unchanged. |
| `98a8d4ea9cab33510c1577619a3c5fcf27baac98` — Advertise the control socket's WebSocket message size limit (#46548) | `5c9b40219` | `ruxex-app-server-transport`: 29/29 passed; `just check --package ruxex-app-server-transport`; `just app-server-e2e`: 99/99 passed, 2 skipped; `just fmt` | Implemented | Added the retained Unix control-socket handshake header using Ruxex naming and the same effective `WebSocketConfig` for both the advertised cap and connection enforcement. |
