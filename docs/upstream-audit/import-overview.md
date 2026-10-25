# Ruxex Upstream Import Overview Ledger

This overview is a generated and manually curated planning ledger for the audited upstream range. It does not change any per-commit audit decision and does not indicate that implementation has occurred.

## Scope

- Fork point: `e269f2164cbb9f499e4f22301c393500e2a831f3`
- Audited upstream range: `2026-09-19`–`2026-10-23` UTC
- Audited commits: 1626
- Detailed commit membership and ledger links: [`import-plan.md`](./import-plan.md)
- Per-date source of truth: the `YYYY-MM-DD.md` ledgers in this directory.

## Decision overview

| Audit decision | Count | Planning meaning |
| --- | ---: | --- |
| `Import proactively` | 909 | Candidate for manual adaptation; not necessarily immediate. |
| `Deferred` | 338 | Known feature outside the current adoption gate. |
| `User review required` | 30 | Blocked until the explicit Ruxex decision is made. |
| `Do not import` | 349 | Excluded from Ruxex implementation. |

## Planning classification

The following classification is for implementation planning only. It is derived from the audit decision and commit subject; it may be refined when a bundle is prepared, without rewriting the original audit record.

| Planning type | Count | Handling |
| --- | ---: | --- |
| `Bug / security / robustness` | 397 | Normally one commit at a time; group only when compilation or a shared prerequisite requires it. |
| `Dependency / maintenance` | 23 | Take independently when compatible; verify the affected workspace and lockfile. |
| `Refactor / structural alignment` | 93 | Group by prerequisite shape and preserve behavior with tests before dependent changes. |
| `Test / diagnostics / maintenance` | 74 | Take with the behavior it protects, or independently when it improves shared coverage. |
| `Behavioral alignment` | 322 | Reconstruct as a feature or protocol-compatible bundle across upstream commits. |
| `Feature candidate (deferred)` | 338 | Hold until the adoption trigger is met and re-audit if necessary. |
| `Review-gated change` | 30 | Hold for user decision, then require the relevant protocol, persistence, or security E2E coverage. |
| `Excluded` | 349 | Do not implement. |

## Executable import queue

This is the order in which the audited work can actually be handled. Queue
items are implementation units, not upstream commit dates. A queue item may
contain commits from several audit batches, while an independent bug fix stays
an individual item.

| Queue | Work unit | Includes | Grouping rule | Prerequisite | Status |
| --- | --- | --- | --- | --- | --- |
| Q0 | Audit baseline | README, date ledgers, `import-plan.md`, and this overview | Commit the audit result as one documentation checkpoint | None | Ready to checkpoint |
| Q1 | Independent bug/security fixes | Retained correctness, crash, data-loss, auth, authorization, and robustness fixes | One upstream fix per Ruxex commit; combine only when one cannot compile or verify without the other | Check for an existing Ruxex equivalent | Ready per item |
| Q2 | Compatible dependencies | Non-member dependency upgrades and related lockfile maintenance | Group updates that share one dependency family or lockfile transition | Compatibility and workspace check | Ready per family |
| Q3 | Shared tests and diagnostics | Regression tests, E2E coverage, diagnostics, and test-fixture maintenance | Keep with the behavior they protect; group shared harness improvements | Relevant retained behavior | Ready per subsystem |
| Q4 | App Server and thread-store refactors | Response decoding/serialization, thread preparation, persistence, history projection, and lifecycle factoring | Group only behavior-preserving prerequisites needed by a later Bundle; skip structures already present in Ruxex | Current Ruxex counterpart check | Ready after reconciliation |
| Q5 | Agent/runtime refactors | Agent control, resume/fork flow, subagent spawning, and message-board storage factoring | Group by one ownership/lifecycle boundary | Q4 only when the implementation shares thread persistence | Candidate |
| Q6 | Execution and permission refactors | Exec-server boundaries, capability roots, sandbox policy plumbing, and tool execution factoring | Group by one permission boundary; keep security review items separate | Q3 and current permission-model check | Candidate; some review-gated |
| Q7 | Retained history and lifecycle behavior | Resume, rollout, pagination internals, compaction, and thread listing behavior | Reconstruct related commits across dates; split public protocol changes into Q9 | Q4 and Q3 | Candidate Bundle |
| Q8 | Retained MCP/tool behavior | Tool metadata, MCP state, plugin/skill identity, and catalog behavior that Ruxex retains | Group by one retained protocol or catalog behavior; exclude backend-only portions | Q5/Q6 as applicable | Candidate Bundle |
| Q9 | Review-gated protocol, persistence, and security features | App Server schema changes, persistent-format changes, auth/permission semantics, and network-boundary changes | Keep each decision-sensitive feature series separate until the user decision is recorded | User decision plus focused E2E | Waiting for review |
| Q10 | Deferred feature backlog | Features that are understood but not yet default-on App Server protocol changes | Keep grouped by future adoption trigger; do not implement now | Trigger and re-audit | Deferred |
| Q11 | Excluded scope | Removed functionality and Codex-only infrastructure | Never add to an implementation Bundle | None | Do not import |

### Queue execution rules

1. Start with Q0, then process Q1 and Q2 independently. They do not wait for
   all refactors or feature Bundles.
2. Use Q3 to add missing regression coverage before a Q4–Q8 Bundle when the
   behavior is not already protected.
3. Reconcile each Q4/Q5 refactor with the current Ruxex code first. An
   upstream refactor that Ruxex already has, or whose structure conflicts with
   Ruxex's architecture, is not imported merely for textual alignment.
4. For Q7/Q8, adapt the upstream tests first, add Ruxex-specific tests, then
   manually adapt the production changes as one Bundle.
5. Split any Q7/Q8 Bundle at a Q9 boundary. A review-gated member must not
   block unrelated bug fixes or retained internal changes.
6. After each queue item passes unit tests and relevant E2E tests, commit the
   Ruxex adaptation and update both the detailed plan and date ledgers.

## Bundle index

The bundle index is the execution map. A bundle may span multiple UTC ledgers and seven-day audit batches. Review-gated members keep the whole affected feature cut waiting until the boundary decision is resolved; unrelated bug fixes remain independently eligible.

| Bundle | Members | Proactive | Deferred | Review | Excluded | Current handling |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `B02` — App Server, thread history, and lifecycle | 465 | 248 | 80 | 14 | 123 | Split out review-gated members; retain bundle order |
| `B03` — Agents, forks, and message-board runtime | 99 | 44 | 39 | 2 | 14 | Split out review-gated members; retain bundle order |
| `B04` — MCP, plugins, skills, and tool metadata | 146 | 103 | 26 | 1 | 16 | Split out review-gated members; retain bundle order |
| `B05` — Provider, Responses, and model behavior | 74 | 44 | 16 | 1 | 13 | Split out review-gated members; retain bundle order |
| `B06` — TUI and CLI behavior | 284 | 149 | 95 | 0 | 40 | Separate retained/refactor work; hold feature members |
| `B07` — Execution, sandbox, and permission boundaries | 224 | 142 | 42 | 10 | 30 | Split out review-gated members; retain bundle order |
| `B08` — Dependencies, diagnostics, tests, and maintenance | 334 | 179 | 40 | 2 | 113 | Split out review-gated members; retain bundle order |

## Recommended execution order

1. Q0 — establish the audit baseline.
2. Q1/Q2 — process independent bug fixes and compatible dependency families.
3. Q3 — close the relevant test and E2E gaps.
4. Q4/Q5/Q6 — adapt only the structural prerequisites that Ruxex actually lacks.
5. Q7/Q8 — implement retained history, lifecycle, MCP, and tool behavior as
   cross-commit Bundles.
6. Q9 — handle protocol, persistence, and security decisions after user review.
7. Q10 — revisit deferred features only when their adoption trigger occurs.
8. Keep Q11 out of all implementation work.

## Accounting rules

- This file is an overview, not a replacement for the per-date audit ledgers.
- A planning classification never overrides the recorded audit decision.
- `Import proactively` means eligible for manual adaptation, not that the change should be cherry-picked or immediately implemented.
- Every implementation must follow tests first, manual Ruxex adaptation, unit/E2E verification, commit, and ledger update.
