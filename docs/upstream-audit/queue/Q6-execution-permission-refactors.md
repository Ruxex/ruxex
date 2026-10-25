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
| `3b16b5a5b03aba9a960b5dedde48fb687863c6e9` — Rename exec-server file handle management identifiers (#49702) | `b3c889695`, `57ba26347` | `ruxex-exec-server`: 258/258 passed; `just fmt-check` passed | Implemented | Followed the retained exec-server structural refactor manually: `file_read` became `file_handle`, and read-handle identifiers now use the broader file-handle terminology without changing protocol behavior or error messages. |
