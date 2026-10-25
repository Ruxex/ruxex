# Q7 — Retained history and lifecycle behavior

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Resume, rollout, pagination internals, compaction, and thread listing behavior |
| Source | Bundle B02; planning type: Behavioral alignment |
| Grouping | Reconstruct related commits across dates; split public protocol changes into Q9 |

Tests are adapted first, followed by Ruxex-specific coverage and then the
manually adapted implementation as one behavior bundle.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| To be reconstructed from `import-plan.md` | — | history/thread-store/App Server unit + E2E | Waiting for Q3/Q4 | Keep public schema and persistence migrations in Q9 |
| `4f699cd64227f42ff1094b0d3704ff8c3d05c96d` — Move session index I/O off async runtime threads (#49708) | `0610ddf4b` | `ruxex-rollout`: 127/127 passed | Implemented | Manually moved session-index writes/removals and batch reverse scans to blocking workers, retaining an owned lock through cancellation and preserving malformed lines and write ordering. |
| `5aa92804d255dcaefe169dd7febb4006a2474e32` — Batch rollout listing scans on cancellable blocking workers (#49694) | `eea37ea85` | `ruxex-rollout`: 129/129 passed | Implemented | Manually moved rollout directory and candidate scans to cancellable blocking workers, retained scan limits and ordering, and shared synchronous plain-before-compressed metadata resolution. |
