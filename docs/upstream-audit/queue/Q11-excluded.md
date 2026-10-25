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
