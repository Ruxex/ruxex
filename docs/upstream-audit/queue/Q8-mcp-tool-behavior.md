# Q8 — Retained MCP and tool behavior

| Field | Value |
| --- | --- |
| Status | Queued |
| Scope | Retained tool metadata, MCP state, plugin/skill identity, and catalog behavior |
| Source | Bundle B04; planning type: Behavioral alignment |
| Grouping | One retained protocol or catalog behavior at a time |

Backend-only Codex behavior and removed functionality stay excluded. Tests
must cover the Ruxex provider boundary and relevant App Server/TUI surfaces.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Notes |
| --- | --- | --- | --- | --- |
| To be reconstructed from `import-plan.md` | — | MCP/tool unit + App Server/TUI E2E | Waiting for Q3/Q5/Q6 | Split review-gated protocol changes into Q9 |
| `05de93520542d5d1a3f3af76ad446607a12d98fd` — Expose declared onboarding skills in plugin details (#46544) | `193c714a1` | `ruxex-app-server-protocol` focused compatibility test; `just check --package ruxex-core-plugins --package ruxex-app-server --package ruxex-app-server-protocol`; `just app-server-e2e`: 99/99 passed, 2 skipped; `just generate` | Implemented | Added optional `PluginDetail.onboardingSkill` for enabled matching local skills. Ruxex accepts `com.codex` and compatible `com.openai` declarations, keeps plugin-root containment, and omits the upstream remote-marketplace branch because Ruxex does not retain it. No cherry-pick. |
| `3ead67394691548e5c2f1be4cad649907f075e66` — Defer extension tool history materialization until first access (#47035) | `b5fdd944c` | `ruxex-tools`: 98/98 passed; `ruxex-core` lib: 676/676 passed; `just check --package ruxex-core`; `just fmt-check` | Implemented | Deferred raw history conversion until extension access, retained the invocation snapshot and history envelopes, shared one conversion across clones, and kept debug formatting lazy. |
| `afe4249cfe6bdf218ce8807a670fad122d2b60ed` — Restrict Unix local MCP servers to stdio descriptors (#47094) | `1bbeaec08` | `just check --package ruxex-rmcp-client --package ruxex-utils-pty`; `just test --package ruxex-rmcp-client`: 323 passed, 9 skipped; focused boundary tests: 2 passed | Implemented | Manually applied the retained Unix descriptor boundary to local MCP launches. Non-macOS uses the shared inherited-FD cleanup; macOS native `posix_spawn` uses `CLOEXEC_DEFAULT`; explicit stdio and exec-failure reporting remain intact. No cherry-pick. |
