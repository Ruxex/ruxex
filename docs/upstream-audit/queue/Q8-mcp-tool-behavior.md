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
