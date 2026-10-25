# Codex Upstream Audit Ledger

This directory records whether changes from the OpenAI Codex upstream repository
should be manually adapted to Ruxex.

## Audit scope

- Upstream repository: <https://github.com/openai/codex>
- Upstream branch: `main`
- Fork point: [`e269f2164cbb9f499e4f22301c393500e2a831f3`](https://github.com/openai/codex/commit/e269f2164cbb9f499e4f22301c393500e2a831f3)
- Scope: upstream commits after the fork point that have not yet been audited
  in Ruxex

Audit commits in chronological order, one commit at a time. Copy the upstream
commit hash and commit message verbatim into the ledger.

## Ledger files

Create one file per UTC date based on the upstream Git committer date. Use the
`YYYY-MM-DD.md` filename format.

Do not create the ledger file for the current UTC date while that day is still
in progress. Even if its commits have already been audited, record them only
after the UTC day has ended. Multiple commits on one date belong in the same
file, in chronological order.

### Mechanical skeleton generation

The upstream commit hash, exact committer timestamp, complete commit message,
and empty audit fields can be generated for one completed UTC date with:

```sh
python3 scripts/generate_upstream_audit.py --date YYYY-MM-DD
```

The default input is the local clone at `../codex-upstream`. Use
`--upstream-repo PATH` to select another clone. The generator reads `main`
without pulling or checking out branches, refuses the current or a future UTC
date, avoids duplicate entries, and preserves existing audit notes.

## Audit procedure

1. Verify the upstream commit hash, committer timestamp, and complete commit
   message.
2. Audit the diff from the fork point against the current Ruxex tree and its
   architecture.
3. Write a Ruxex-oriented summary of the change.
4. Make a provisional import decision and record the reason.
5. After completing the audit for a UTC date or audit batch, share a user-facing
   summary before starting any adaptation. Group the results by decision label,
   include counts and commit references, summarize the planned imports, and
   list the trigger for each `Deferred` group. For mixed commits, summarize
   each retained portion separately from the excluded portion.
6. If needed, ask the user to decide before importing.
7. After approval, prepare the verification before adapting production code.
   First manually adapt the upstream tests that cover the retained behavior,
   rather than importing the upstream implementation wholesale. If the
   upstream tests do not cover Ruxex-specific boundaries or behavior, add new
   Ruxex-focused tests before changing the implementation. Keep provider,
   protocol, App Server, and TUI matrices aligned where the behavior is
   exposed through more than one path.
8. Run the test-first checks to confirm that the tests exercise the intended
   behavior. Tests may fail before the implementation is adapted; do not mark
   the change complete or commit it in that state.
9. Manually adapt only the required production changes to Ruxex, guided by the
   tests. Do not cherry-pick the upstream commit; compare the diff and reflect
   it by hand while preserving Ruxex's architecture and naming.
10. Run the affected unit-test suites, including the relevant E2E suites. For
   AppServer changes, run `just app-server-e2e`; for TUI changes, run
   `just tui-e2e`; run both when both layers are affected.
11. Only after the required tests pass, commit the manual adaptation and record
    the resulting Ruxex commit hash in the ledger.

Keep the order as: audit, user-facing classification and import summary, user
decision when needed, tests adapted or added, test-first check, manual
production adaptation, verification (including E2E), commit, then ledger
update. Do not mark an entry as manually reflected or fill in its Ruxex-side
commit until the required tests pass.

## Decision criteria

Apply the following decision flow to the diff, not merely to the upstream
commit title or message. If one commit contains multiple independent changes,
split the diff conceptually and evaluate each change separately. Only the
Ruxex-relevant parts may be manually adapted.

1. Does the change concern functionality that Ruxex has removed?
   - Yes: `Do not import` that part of the change.
   - If only part of the commit is removed functionality, exclude that part and
     continue evaluating the remainder.
2. Is it a bug fix for functionality that Ruxex still retains?
   - Yes: `Import proactively`, unless the defect cannot occur in Ruxex because
     its architecture already prevents it. Security fixes, data-loss fixes,
     crashes, and authentication or authorization fixes take priority.
3. Is it a dependency update?
   - For a non-member dependency that Ruxex uses, `Import proactively` when
     compatibility is confirmed. Security updates take priority.
   - A major-version update, API-breaking update, build-impacting update, or
     member-crate update requires `User review required`.
   - An update for a dependency that Ruxex does not use is `Do not import`.
4. Is it a test, CI, diagnostics, or maintenance improvement for retained Ruxex
   functionality?
   - Yes: `Import proactively` when it can be adapted to Ruxex's architecture
     and protects behavior that Ruxex actually supports.
   - Codex-specific providers, settings, execution paths, or infrastructure are
     `Do not import` unless an equivalent Ruxex use is identified.
5. Is it an internal refactoring of retained functionality?
   - A behavior-preserving refactoring that improves correctness or maintenance
     is `Import proactively` after compatibility is confirmed.
   - Trait extraction, interface factoring, and internal ownership or module
     refactoring should generally be followed when they preserve behavior and
     support later upstream changes. If a later upstream commit depends on the
     extracted trait or interface, audit the structural changes as one related
     sequence instead of leaving the base refactor behind and recreating it in
     each follow-up.
   - A refactoring that changes Ruxex's provider boundary, naming, runtime
     model, permission/security semantics, wire format, or public behavior is
     evaluated by the review gate below.
6. Does the change concern retained Ruxex functionality and remain within
   Ruxex's architecture?
   - A bug fix is not enough by itself to establish that the code path is
     retained; first apply rule 1.
   - New functionality is not imported merely because it is compatible or
     user-visible. If it is not eligible for the review gate below, mark it
     `Deferred` and record the trigger that would cause a future re-audit.
   - Manually adapt eligible changes to Ruxex names, structure, provider
     boundary, and existing defaults. Preserve unsupported or ambiguous inputs
     through a safe fallback where applicable.
   - Cover every manually adapted behavior with focused unit tests and the
     relevant E2E tests before committing the adaptation.
7. Is this a new feature that is both enabled by default upstream and changes
   the App Server protocol?
   - Yes: `User review required`.
   - For this rule, an App Server protocol change means a change to a public
     App Server schema, JSON-RPC method, parameter, result, notification,
     capability advertisement, authentication handshake, or other wire-level
     contract. Internal runtime traits and TUI-only behavior do not qualify.
   - Both conditions are required. A default-only TUI or configuration change,
     or an opt-in protocol extension, is `Deferred` unless an exception below
     applies.
8. Does the change require a review outside the new-feature gate?
   - Yes: `User review required` for an irreversible/destructive operation,
     a security or permission semantic that cannot be treated as a bug fix, or
     a compatibility-sensitive persistent/wire migration that requires a
     Ruxex design choice.
   - A normal configuration, default, public API, or user-visible change that
     is not a default-on App Server feature is `Deferred`, not a review item.
9. Does it introduce functionality that does not exist in Ruxex and fail the
   review gate?
   - Yes: `Deferred`. Record the capability and the exact trigger for
     re-audit; do not ask the user to review it now.
10. Is the Ruxex impact or compatibility unclear?
   - Yes: `User review required`. Record the specific uncertainty instead of
     silently importing the change.

This policy applies to new and not-yet-decided audit entries. Completed
historical audits retain their recorded decision unless they are explicitly
re-audited under the updated policy.

### User review required

- The update introduces a new feature that is enabled by default and changes
  the App Server protocol, schema, capability advertisement, authentication
  handshake, or other App Server wire contract.
- The update requires a review outside that gate because it changes a security
  or permission semantic, performs an irreversible/destructive operation, or
  requires a compatibility-sensitive persistent/wire migration.
- The update is a major or otherwise compatibility-sensitive dependency or
  member-crate change.
- The Ruxex impact or compatibility cannot be established.

Do not reflect either category until the user has decided.

### Import proactively

- The upstream change fixes a bug in functionality that Ruxex still retains.
- Security, data-loss, crash, authentication, and authorization fixes in
  retained functionality are prioritized.
- The change updates a non-member dependency crate used by Ruxex, when the
  compatible update can be confirmed.
- The change improves tests, CI, diagnostics, or maintenance for retained Ruxex
  functionality and can be adapted to Ruxex's architecture.
- A behavior-preserving internal refactoring improves correctness or
  maintainability without crossing Ruxex's architectural boundaries.
- A behavior-preserving trait extraction, interface factoring, or internal
  module/ownership refactoring keeps Ruxex structurally aligned with retained
  upstream follow-ups, especially when later commits depend on that shape.
- A compatible behavior improvement that is not a new feature stays within
  Ruxex's architecture and is covered by focused unit and E2E tests.

Apply these changes manually, adapting them to Ruxex names, structure, and
boundaries.

### Do not import

- The update targets only functionality that Ruxex has already removed.
- The update targets a dependency or infrastructure that Ruxex does not use.
- The update is specific to a Codex provider, setting, execution path, or
  other capability with no Ruxex equivalent.

Record which removed capability was identified and which part of the diff
belongs to that capability.

### Deferred

- The update is a new feature but is not both default-on upstream and an App
  Server protocol change.
- The update is a default-only, opt-in, TUI-only, provider-specific, or other
  user-visible feature that does not meet the review gate.

Do not implement deferred changes. Record the reason and the trigger for
re-audit, such as the feature becoming default-on or gaining an App Server
protocol change. A deferred entry may later be reclassified without treating
the original decision as a permanent rejection.

### Decision labels

Use exactly one of these labels for each audited change or independently
evaluated part of a mixed commit:

- `Import proactively`
- `User review required`
- `Do not import`
- `Deferred` — a new feature that does not yet meet the review gate
- `Not applicable` — the issue cannot occur in Ruxex or is already solved by a
  different Ruxex implementation.

When the impact or compatibility is unclear, use `User review required` and
record the specific uncertainty. Use `Deferred` only when the feature is
understood but intentionally outside the current adoption gate.

## Entry format

Each entry must contain at least the following fields:

```markdown
## `<upstream commit hash>` — <upstream commit subject>

- Upstream commit: `<upstream commit hash>`
- Upstream committer date (UTC): `<YYYY-MM-DDTHH:MM:SSZ>`
- Upstream commit message:

  ```text
<verbatim upstream commit message>
  ```

- Diff audit:
  - <audited change scope>
- Ruxex summary: <summary of the change>
- Provisional decision: `User review required` / `Import proactively` /
  `Do not import` / `Deferred`
- Reason: <reason based on Ruxex capabilities and policy>
- Reflection status: <not imported / manually reflected>
- Ruxex-side commit: `<manual Ruxex commit hash>`
- Notes: <optional>
```

For `Do not import`, `Deferred`, and `User review required` entries, use `N/A`
for the Ruxex-side commit. When a change is manually reflected, record the
resulting Ruxex commit hash.
