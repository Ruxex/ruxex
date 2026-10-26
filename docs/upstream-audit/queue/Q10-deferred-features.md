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
| `0a76a2520f45af68b7a5386688a9714af0a3ccd2` — Export skill invocation events through OpenTelemetry (#49689) | `N/A` | audit only | Deferred | Re-audit if Ruxex adopts the event contract or approves a new default-on observability requirement. |
| `2cc65cdd4c7c167f0c7252fcb5165d649c16aca0` — Enable analytics by default for daemon-launched app servers (#49426) | `N/A` | No implementation; audit comparison completed | Deferred | Re-audit if Ruxex intentionally enables daemon metrics by default or the behavior becomes an App Server wire-contract change |
| `15fd656ddb55bd82a208fb9f00681880523f5260` — Discover and validate MCP authorization servers before ID-JAG exchange (#49478) | `N/A` | No implementation; retained trusted-metadata exchange remains unchanged | Deferred | Re-audit when Ruxex enables `ema_auth` as an active path, adopts the discovery contract, or records an explicit security exception to the adoption gate |
| `90abcfac02665ad882853a04155591cd863b2ca7` — Add experimental thread prediction protocol types (#49480) | `N/A` | No implementation; request handling remains unimplemented upstream | Deferred | Re-audit if thread prediction becomes a default-on Ruxex capability or a required App Server protocol contract |
| `d42056091aded7feb1d88ac7e83972108b2aa478` — Add a fork shortcut to the TUI command center (#49517) | `N/A` | No implementation; TUI feature bundle remains outside the adoption gate | Deferred | Re-audit if fork UX/keymap behavior becomes an explicit Ruxex product target or an App Server-backed default behavior |
| `2e5fea64eefcaa19f48458b2386011b619f69c70` — Add an opt-in model catalog to multi-agent context (#49560) | `N/A` | No implementation; feature is disabled by default | Deferred | Re-audit if model catalog context becomes default-on or an App Server protocol contract |
| `0b43721d8d1f734658e41bffe12a6ba6c9240abd` — Copy selected file paths as plain text in the TUI (#49564) | `N/A` | No implementation; TUI-only interaction change | Deferred | Re-audit if this becomes an explicit Ruxex TUI adoption target |
| `60947e234156ac12bdb7fba2477d3965f166bd34` — Add account security setup reminders to the TUI (#49715) | `N/A` | No implementation; audit comparison completed | Deferred | Re-audit if Ruxex adopts the account-security notice surface or it becomes a default-on App Server protocol contract |
| `3fd5160cd6c78f2051bb54359d53f09207171733` — Honor the configured product SKU in remote plugin requests (#47116) | `N/A` | No implementation; Ruxex has no remote plugin request path | Deferred | Re-audit if Ruxex retains remote marketplace/plugin backend requests or adopts a product-SKU App Server contract |
| `f07aaf920b14d7a746e435add34b8bbd37da5da6` — Highlight selected newlines in the transcript view (#47068) | `N/A` | No implementation; TUI-only adoption gate not met | Deferred | Re-audit if Ruxex explicitly prioritizes newline selection highlighting or the behavior becomes part of an App Server-backed contract |
| `f3037cafd32b2097dae718eb86bcc9182dc0a5d1` — Refresh host-supplied cloud skills at turn startup (#47074) | `N/A` | No implementation; Ruxex has no retained cloud skill provider boundary | Deferred | Re-audit if Ruxex adopts host-supplied cloud skills or an associated App Server contract |
