# Q10 — Deferred feature backlog

| Field | Value |
| --- | --- |
| Status | Deferred |
| Scope | Features that are not default-on App Server protocol changes and are not otherwise eligible for proactive import |
| Source | Audit decision: Deferred (338) |
| Grouping | Keep grouped by future adoption trigger; re-audit when the trigger occurs |

Deferred does not mean rejected. It means no implementation is planned under
the current adoption gate.

| Upstream commit(s) | Ruxex commit | Tests/checks | Status | Trigger |
| --- | --- | --- | --- | --- |
| To be selected from `import-plan.md` | — | — | Deferred | Record the exact future default/protocol or product trigger |
| `2cc65cdd4c7c167f0c7252fcb5165d649c16aca0` — Enable analytics by default for daemon-launched app servers (#49426) | `N/A` | No implementation; audit comparison completed | Deferred | Re-audit if Ruxex intentionally enables daemon metrics by default or the behavior becomes an App Server wire-contract change |
| `15fd656ddb55bd82a208fb9f00681880523f5260` — Discover and validate MCP authorization servers before ID-JAG exchange (#49478) | `N/A` | No implementation; retained trusted-metadata exchange remains unchanged | Deferred | Re-audit when Ruxex enables `ema_auth` as an active path, adopts the discovery contract, or records an explicit security exception to the adoption gate |
| `90abcfac02665ad882853a04155591cd863b2ca7` — Add experimental thread prediction protocol types (#49480) | `N/A` | No implementation; request handling remains unimplemented upstream | Deferred | Re-audit if thread prediction becomes a default-on Ruxex capability or a required App Server protocol contract |
