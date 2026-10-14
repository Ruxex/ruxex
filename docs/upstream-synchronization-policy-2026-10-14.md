# Ruxex upstream synchronization policy

This is the active synchronization policy for the Ruxex fork. The older
[`upstream-commit-audit-2026-10-07.md`](upstream-commit-audit-2026-10-07.md)
and [`upstream-commits-after-fork-2026-10-07.md`](upstream-commits-after-fork-2026-10-07.md)
remain historical research records; their OpenAI repository snapshot is not
the active synchronization source.

## Current roles and checkpoint

| Ref | Value | Role |
|---|---|---|
| `origin` | `https://github.com/Ruxex/ruxex.git` | Ruxex publication remote |
| `origin/develop` | `923a1d50c01e` | Current Ruxex working branch |
| `upstream` | `https://github.com/edge-m/codex-fork.git` | Source fork remote for future review |
| `upstream/main` | `0071ec8184cb` | Current source branch to inspect |
| shared checkpoint | `0071ec8184cb` | Common ancestor of `main`, `upstream/main`, and `develop` |
| target branch | `develop` | Integration branch for reviewed Ruxex changes |

At the 2026-10-14 checkpoint, `develop..upstream/main` contains zero commits
and `upstream/main..develop` contains the Ruxex publication commits. There is
therefore no upstream candidate to import in this cycle. Ruxex's own commits
must not be treated as upstream imports.

## Review cadence

Before each planned synchronization cycle:

1. Fetch `upstream` and record the fetched `upstream/main` SHA.
2. Start at the last reviewed upstream SHA, not necessarily at the current
   branch merge base.
3. Enumerate candidates in dependency order using the upstream commit graph.
4. Create one ledger row per candidate commit before changing Ruxex code.
5. Implement accepted changes as Ruxex-compatible patches. Do not blindly
   cherry-pick commits that depend on removed hosted surfaces, old names, or
   provider-specific runtime assumptions.
6. Update the row with the final decision, affected producer/consumer boundary,
   and verification result.
7. Run the repository verification policy for every accepted batch and push the
   ledger together with the code it explains.

The default cadence is before a Ruxex release or when a security, sandbox,
protocol, persistence, or dependency change in the source fork is relevant.
Routine upstream changes are not imported automatically.

## Decision ledger

Every candidate uses one of these decisions:

- `import`: the commit can be applied with no material semantic change after
  the Ruxex boundary audit;
- `partial`: only the provider-neutral, supported, or security-relevant part is
  implemented and the excluded paths are recorded; or
- `skip`: the commit is intentionally not applicable, obsolete, unsafe, or
  outside Ruxex scope.

Each row must contain the commit SHA, original subject, decision, reason,
affected Ruxex boundary, dependency notes, and verification result. Keep
provider-specific, hosted, licensing, and security-sensitive decisions
explicit even when the result is `skip`.

| Candidate SHA | Subject | Decision | Reason / affected boundary | Dependencies | Verification |
|---|---|---|---|---|---|
| None at the 2026-10-14 checkpoint | — | — | `upstream/main` has no commits beyond the shared checkpoint | — | `git fetch --prune upstream`; `git rev-list --count develop..upstream/main` = `0` |

## Verification contract

For accepted Rust changes, use the repository's memory-limited `just` recipes:

```text
just check
just check --tests
just generate
git diff --check
```

Do not claim an upstream batch is complete until the relevant commands pass.
Hosted Linux/macOS CI is additional evidence and does not replace the local
producer/consumer audit. If a candidate is security-sensitive, record the
security boundary and the focused test or review result in the same row.
