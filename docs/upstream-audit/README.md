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
5. After completing the audit for a UTC date, share a user-facing summary
   before starting any adaptation. Group the results by decision label, include
   counts and commit references, and summarize the planned imports. For mixed
   commits, summarize each retained portion separately from the excluded
   portion.
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
   - Yes: `Import proactively` is the default, including compatible behavior
     improvements and localized feature extensions. The fact that the upstream
     change is user-visible does not by itself require a review.
   - Manually adapt the change to Ruxex names, structure, provider boundary,
     and existing defaults. Preserve unsupported or ambiguous inputs through a
     safe fallback where applicable.
   - Cover the retained behavior with focused unit tests and the relevant E2E
     tests before committing the adaptation.
7. Does the change require a material Ruxex design decision?
   - Yes: `User review required`. This is limited to changes that alter a
     public API, persistent or wire format, configuration contract, default
     user-visible behavior, security or permission semantics, provider/runtime
     ownership, or an irreversible/destructive operation.
   - A behavior change that is compatible with the retained Ruxex contract and
     does not cross one of these gates remains `Import proactively`.
8. Does it introduce functionality that does not exist in Ruxex?
   - Yes: `User review required`, unless it is strictly internal test,
     diagnostics, CI, or maintenance support with no new public capability.
   - Upstream presence alone is not a reason to import a new public feature.
9. Is the Ruxex impact or compatibility unclear?
   - Yes: `User review required`. Record the specific uncertainty instead of
     silently importing the change.

### User review required

- The update requires a material Ruxex design decision: a public API,
  persistent or wire format, configuration contract, default user-visible
  behavior, security or permission semantic, provider/runtime ownership, or an
  irreversible/destructive operation changes.
- The update introduces a completely new public capability for Ruxex.
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
- A compatible behavior improvement or localized feature extension to retained
  functionality stays within Ruxex's architecture and is covered by focused
  unit and E2E tests.

Apply these changes manually, adapting them to Ruxex names, structure, and
boundaries.

### Do not import

- The update targets only functionality that Ruxex has already removed.
- The update targets a dependency or infrastructure that Ruxex does not use.
- The update is specific to a Codex provider, setting, execution path, or
  other capability with no Ruxex equivalent.

Record which removed capability was identified and which part of the diff
belongs to that capability.

### Decision labels

Use exactly one of these labels for each audited change or independently
evaluated part of a mixed commit:

- `Import proactively`
- `User review required`
- `Do not import`
- `Not applicable` — the issue cannot occur in Ruxex or is already solved by a
  different Ruxex implementation.

When the impact or compatibility is unclear, use `User review required` and
record the specific uncertainty rather than using an unqualified `Hold` label.

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
- Provisional decision: `User review required` / `Import proactively` / `Do not import`
- Reason: <reason based on Ruxex capabilities and policy>
- Reflection status: <not imported / manually reflected>
- Ruxex-side commit: `<manual Ruxex commit hash>`
- Notes: <optional>
```

For `Do not import` and `User review required` entries, use `N/A` for the
Ruxex-side commit. When a change is manually reflected, record the resulting
Ruxex commit hash.
