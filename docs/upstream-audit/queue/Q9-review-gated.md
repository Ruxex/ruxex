# Q9 — Review-gated protocol, persistence, and security features

| Field | Value |
| --- | --- |
| Status | Waiting for review |
| Scope | Default-on App Server protocol changes, persistent-format changes, auth/permission semantics, and network-boundary choices |
| Source | Audit decision: User review required (30) |
| Grouping | Keep each decision-sensitive feature series separate |

No implementation starts until the user decision and focused Ruxex E2E plan
are recorded. This queue must not block unrelated Q1–Q8 work.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| To be selected from `import-plan.md` | — | Focused protocol/persistence/security E2E | Waiting for user decision | Record the exact design uncertainty |
| `de020167989fde4937204a064ea2fedd54f3c931` — Persist explicit user goal edits in model history (#49598) | `N/A` | No implementation; protocol/persistence review required | Waiting for user decision | Decide whether goal origin fields and explicit goal history are retained Ruxex semantics; define focused AppServer and resume/compaction E2E before adaptation. |
