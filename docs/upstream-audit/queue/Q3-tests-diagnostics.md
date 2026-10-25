# Q3 — Shared tests and diagnostics

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Regression tests, E2E coverage, diagnostics, and retained test-fixture maintenance |
| Source | `import-overview.md` planning type: Test / diagnostics / maintenance (74) |
| Grouping | Keep coverage with the behavior it protects; group shared harness changes |

Provider matrices must cover both Chat Completions and Responses where the
behavior is exposed through both paths. App Server and TUI coverage use their
Ruxex targets.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| `5e96aabd68dc6194f5ad928cb7f5d0bc71b72df9` — UTF-8 token truncation boundaries | `f52d01b44` | `ruxex-utils-string` focused nextest; 19/19 passed | Implemented | Ruxex already had the behavior; retained the upstream regression coverage as a Ruxex utility test |
| `5e96aabd68dc6194f5ad928cb7f5d0bc71b72df9` — Avoid full-string scans in token-budget truncation (#49712) | `8466c8452` | `ruxex-utils-string`: 19/19 passed; `just check --package ruxex-utils-string`; `just fmt-check` | Implemented | Avoided discarded-middle character scans on the token path using UTF-8 byte boundaries; preserved character-budget behavior and existing boundary tests. |
| `8ea2c0e0d4bc1261258eb532b5a9d616147b6571` — Prune diagnostic logs periodically by age and database size (#49425) | `0a127e205` | `ruxex-state`: 137/137 passed (2 flaky retries); `just check --package ruxex-state`; `just fmt-check` | Implemented | Moved retained log cleanup to an immediate/periodic maintenance worker, added occupied-page budget pruning and passive checkpointing, and covered age boundaries plus idle cleanup. |
| `ab84d71f5767e4a565ce81c2c426287cb48c7918` — Omit payloads from multiline ANSI warnings (#49416) | `6ba35a551` | `ruxex-ansi-escape`: 2/2 passed; `just check --package ruxex-ansi-escape`; `just fmt-check` | Implemented | Enabled the crate's existing tests, added a tracing capture regression test covering small and 128 KiB tails, and retained only structured input-byte and line-count fields in multiline warnings while preserving first-line styling. |
| `d940fb21f35ef32a736016a4c196150c6380bdb3` — Truncate input text in protocol debug output (#49415) | `55e097f5e` | `ruxex-protocol`: 317/317 passed; `just check --package ruxex-protocol`; `just fmt-check` | Implemented | Bound `Debug` previews for `ContentItem::InputText` and `UserInput::Text` to 512 UTF-8-safe bytes and included the full byte length, with regression coverage for both protocol types. Wire serialization is unchanged. |
