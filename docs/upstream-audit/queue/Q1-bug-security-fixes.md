# Q1 — Independent bug and security fixes

| Field | Value |
| --- | --- |
| Status | In progress |
| Scope | Retained correctness, crash, data-loss, auth, authorization, and robustness fixes |
| Source | `import-overview.md` planning type: Bug / security / robustness (397) |
| Grouping | One upstream fix per Ruxex commit; group only for a required shared prerequisite |

Each item is inspected against the current Ruxex tree before implementation.
Production changes are manually adapted after tests are prepared; upstream
commits are never cherry-picked.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| `5e96aabd68dc6194f5ad928cb7f5d0bc71b72df9` — Avoid full-string scans in token-budget truncation (#49712) | N/A | Boundary regression test passes before production changes | No production adaptation required | Ruxex already preserves the tested UTF-8 behavior. The remaining upstream difference is an unobserved performance optimization, so no production change is justified under the test-first rule. |
| `67352b5b4adc3d3e8865ac8bf343b952b8c040e7` — Recover exec-server sessions after environment info timeouts (#49407) | `f04415bc6` | `ruxex-exec-server`: 251/251 passed; `just fmt-check` | Implemented | Ruxex has the same timeout-prone environment-info probe and recovery state machine. The timeout retires the probed transport and requests recovery. |
| `0ad18769ad8dfb7a6cf16b1bdd26b6e5f8773f64` — Avoid exposing config values in `codex doctor` errors (#46962) | `e69b04665` | `ruxex-cli`: 88/88 passed; focused redaction tests passed; `just fmt` passed | Implemented | Manually adapted the retained doctor report to expose only typed configuration location metadata, I/O error kinds, or a generic fallback; raw configuration values are no longer reported. |
| `f2b2e5b2a96d845a16d649f0984c0bd06c15ef37` — Make exec-server file reads cancellable between chunks (#49696) | `2a842f8af` | `ruxex-exec-server`: 258/258 passed; focused read tests passed; `just fmt-check` passed | Implemented | Manually adapted the retained exec-server read path to use blocking bounded chunks with cancellation checks while preserving symlink, regular-file, FIFO, and 512 MiB limit behavior. |
| `76a6e55d5ac69e3ba6b0481f8f3b94256f46dfa7` — Restore executor tool paths after login shell startup (#49467) | N/A | Protocol and runtime comparison completed | Not applicable | Ruxex does not retain the executor-reported tool-directory protocol or `login_shell_package_path` feature; importing this would add a new cross-process feature rather than repair retained behavior. |
| `d1d0e89558578f622b93702686963e67948fd84c` — Infer Windows UNC paths with forward and mixed slashes (#49424) | N/A | Path-URI model comparison completed | Not applicable | Ruxex deliberately supports only POSIX `PathConvention`; importing Windows UNC inference would add unsupported path semantics and change ambiguous `//...` behavior. |
| `3620b2caf8afecfcedd5c5c9520c0d239c880f26` — Detect SQLite corruption during startup and preserve recovery backups (#49701) | `06422447b` | `ruxex-state`: 134/134 passed; `ruxex-app-server`: 204/204 passed, 2 skipped; `just fmt` passed | Implemented | Added budgeted startup quick-check validation, backup collection across automatic recovery, and a user-facing warning that preserves the existing unavailable policy for thread history. |
| `596ae94fb00f8325d6bf835a9ebdc90baab4ad33` — Classify SQLite corruption using typed error codes (#49710) | `d97c359ad` | `ruxex-state`: 135/135 passed; `ruxex-app-server`: 204/204 passed, 2 skipped | Implemented | Replaced message-text corruption detection with typed SQLite primary/extended result-code classification, including directly wrapped SQLite errors; removed the obsolete public detail helper. |
| `6ba4bf9e647cf8bc17eb9005af5ca9498bd7e654` — Honor server retry advice across Responses retries and fallback (#49441) | `868b63a80` | `ruxex-http-client`: 86/86; `ruxex-api`: 96/96; `ruxex-protocol`: 318/318; `ruxex-client` Retry-After integration test; `just app-server-e2e`: 99/99 passed, 2 skipped; `just fmt-check` | Implemented | Manually preserved monotonic `Retry-After` deadlines through HTTP, Responses SSE, WebSocket rejection, API mapping, stream retries, and transport fallback without changing retry budgets or making quota/policy errors retryable. |
