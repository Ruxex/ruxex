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
| `8c3612fb638d356579c446de68421288164484dc` — Use `memrchr` to find newlines in reverse JSONL scans (#49444) | `226fb9b9f` | `ruxex-rollout`: 129/129 passed | Implemented | Added the compatible workspace `memchr` dependency and used `memchr::memrchr` for reverse newline search without changing scanner behavior. |
| `d7b0d4aa663172876871b605d53283bc0120882b` — Return authoritative replay history when resuming a thread (#49599) | `9da092934` | `ruxex-thread-store`: 184/184 passed; `just check --package ruxex-core` | Implemented | Changed resume to return the replay history selected after writer ownership is acquired, refreshed local history through the installed writer, preserved explicit headerless overrides, and updated session restoration to consume the returned authoritative history. |
| `92bc601ad60542c92bf0bb1e7a2eb70b84ac49d2` — Reuse unchanged history snapshots when resuming threads (#49600) | `9884dc89e` | Focused regression passed; `ruxex-thread-store`: 185/185; `just check --package ruxex-core --package ruxex-app-server`; `just fmt-check` | Implemented | Added opaque history revisions through stored/resumed contexts, reuses an unchanged snapshot only after writer-owned revision validation, reloads stale or unknown snapshots, preserves explicit headerless overrides, and supports canonical, compressed, external, and archived-path handling within Ruxex boundaries. |
