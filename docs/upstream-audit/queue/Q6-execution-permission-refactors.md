# Q6 — Execution and permission refactors

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Exec-server boundaries, capability roots, sandbox policy plumbing, and retained tool execution factoring |
| Source | Bundle B07; planning type: Refactor / structural alignment |
| Grouping | One permission boundary at a time; security-semantic changes stay separate |

Run the relevant E2E coverage before and after adaptation. A security or
permission semantic that is not a retained bug fix is Q9 review-gated.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| To be selected from `import-plan.md` | — | execution/permission unit + E2E | Waiting for reconciliation | Verify current Ruxex permission model and retained behavior |
