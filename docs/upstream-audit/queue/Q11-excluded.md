# Q11 — Excluded scope

| Field | Value |
| --- | --- |
| Status | Do not import |
| Scope | Removed functionality and Codex-only infrastructure |
| Source | Audit decision: Do not import (350) |
| Grouping | Never add to an implementation bundle |

These entries remain in the date ledgers and import plan for auditability, but
must not produce Ruxex production or test changes.

| Upstream commit(s) | Ruxex commit | Status | Reason |
| --- | --- | --- | --- |
| To be selected from `import-plan.md` | — | Do not import | Removed or Codex-only functionality |
| `17e5a4dbcb4106955dad3e6a661459c77193ea1d` — Upgrade the argument comment lint toolchain and Dylint (#49706) | `N/A` | Do not import | Codex-specific lint/toolchain and CI infrastructure, including configuration for a non-retained `chatgpt` crate; no retained Ruxex runtime, dependency, or test behavior. |
| `d8f69ea8bc998a33c19a08176cfc22fd4352c592` — Preserve bootstrap discovery across authentication changes (#49432) | `N/A` | Do not import | Codex-only embedded bootstrap discovery and account-owned network policy; Ruxex has no equivalent retained path. |
| `fc81a7154bc6b53f0f2e2b9ecdb33504e0b38f5d` — Add local audio device selection to TUI voice settings (#49437) | `N/A` | Do not import | Codex-only WebRTC voice helper, local audio-device capture, and desktop TUI settings; Ruxex has no equivalent retained runtime path. |
| `bcd6d9ab6b9f26f85d76d0c680b3f88b367bffa0` — Add regression coverage for account switches between analytics batches (#49489) | `N/A` | Do not import | Codex account analytics batching is not a retained Ruxex runtime or credential path; the test and unused-import cleanup have no Ruxex equivalent. |
| `a5cce8895a1400f94eb0a71771275284027fff17` — Skip host skill discovery for Guardian reviews (#49584) | `N/A` | Do not import | Guardian review sessions and their host-skill warmup path are Codex-only; Ruxex has no retained equivalent. |
| `1fc8d548077fc72c4e3d048a78173af07385217f` — Make the strict network approval test independent of request order (#49595) | `N/A` | Do not import | The corresponding Ruxex suite source is not wired into an active Cargo test target, so importing the test-only adaptation would not execute. Revisit after intentional suite enablement. |
| `67727e7cf114cf3e1b71db368d74b24e32f6cb12` — Allow managed requirements to disable the Windows MXC sandbox (#49642) | `N/A` | Do not import | Ruxex does not retain the upstream Windows MXC sandbox backend or its managed requirement/configuration surface. Importing this would reintroduce removed platform-specific infrastructure. |
| `fcbed044c8c6994c97c7387d21ff3077cf3a5812` — Escape command drafts when recovering question answers (#49678) | `N/A` | Do not import | Ruxex answers retained async questions through the dedicated question Composer and `QuestionSubmission` path; it does not recover answers into the main Composer, so the upstream command-prefix bug has no retained equivalent. |
| `6996cde697b228fbf5695ce855b2498c5c668795` — Add a managed feature gate for in-app voice (#49683) | `N/A` | Do not import | Ruxex does not retain the upstream desktop in-app voice capability or managed feature-gate contract. |
| `49be2c7ab029434dc9545cf85a7a41fd5d3beb1a` — Deliver remote message board notifications to active turns (#49686) | `N/A` | Do not import | Ruxex does not retain the upstream remote agent-message-board client, extension, or turn notification receiver. |
| `40eeb6e8a89ef421c25d4c40e06fa1d40ce66b4f` — Recover corrupt message-board storage during thread deletion (#47042) | `N/A` | Do not import | Ruxex does not retain the upstream agent message-board extension or its SQLite storage, so there is no corresponding thread-deletion cleanup path. |
| `f484f7228d0f41df061ce246f379ad0c82dd0472` — Share SQLite pools across local agent message-board handles (#47049) | `N/A` | Do not import | Ruxex does not retain the agent message-board extension or its SQLite board database, so there are no message-board handles or pools to share. |
| `0e69d3bd9b1263dbe7a0c564e67fd4a7656dc1a5` — Batch message-board subscriber reads into one SQLite result (#47050) | `N/A` | Do not import | Ruxex does not retain message-board posting, subscriber storage, or notification queries. |
| `d583e73c4d1204f1e9f654ef87065e9f0dd07ac7` — Fetch local message board thread summaries in one query (#47055) | `N/A` | Do not import | Ruxex has no retained local agent message-board thread-summary API or backing schema. |
| `fe6455a4384d928f009e4e40459701604ea20b7e` — Add a partial index for agent message board root posts (#47063) | `N/A` | Do not import | Ruxex does not retain the agent message-board SQLite schema, so the upstream root-post index has no compatible database. |
| `51d221a9b77ad4810608f148e90231f0e48b5c82` — Stop redundant message-board reads when limits reach one (#47064) | `N/A` | Do not import | Ruxex does not retain message-board result metadata or its paginated read path. |
| `7e8878f60560f8ff964fe38712e53def06402db7` — Preserve PowerShell relative paths in the elevated Windows sandbox (#49690) | `N/A` | Do not import | Ruxex does not retain the upstream Windows sandbox service or ACL runtime path. |
