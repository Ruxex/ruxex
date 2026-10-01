# SDD ledger — plan: docs/superpowers/plans/2026-10-01-remove-windows-sandbox-service.md

Workspace: main checkout, per repository instructions; no worktree.

Pre-flight: Tasks 1–5 share the workspace/build/config interfaces in sequence. Task 1 removes the service crate and feature surface; Task 2 removes the library APIs consumed by Task 3; Task 3 removes app-server/sandboxing callers; Task 4 documents the resulting active surface; Task 5 verifies the complete branch.

Task 1: Ruling: the environment did not provide the plan skill's documented `task-start` helper, so the available `task-brief` helper was used instead; the task scope and BASE were unchanged.

Task 1: TDD exception: this task only removes a crate and configuration/generated surfaces; repository instructions explicitly state that test-first development is not required for crate-removal work.

Task 1: complete — `just generate` succeeded; `cargo metadata --no-deps` succeeded; the service crate/feature/config search is clear except for the Windows sandbox library's service APIs intentionally deferred to Task 2; commit `6b9ffcae2`.

Task 2: TDD exception: this is service/Registered Core crate-removal and API simplification; repository instructions explicitly allow omitting test-first development for crate-removal work. Existing ordinary sandbox tests were updated to cover the single native helper path.

Task 2: complete — removed service IPC, package identity, runtime ownership, registration, alias verification, and Registered Core-only tests; retained the native setup helper, command runner, ACL/token/Job Object/Private Desktop/deny-read paths. `cargo check -p codex-windows-sandbox` passed on Linux; Windows-target compilation was attempted but the environment lacks `x86_64-w64-mingw32-gcc`.

Task 3: complete — removed app-server service refresh/provisioning and desktop registration, proxy listener contract, and Registered Core environment injection while preserving native Windows setup and wrapper environment sanitization.

Task 4: complete — removed active service entry points from Hawk documentation and stale service dependency wording; compatibility inventory records the audited remove/retain decision.

Task 5: complete — `just check --tests` and `just check` both passed with existing warnings; `cargo metadata --no-deps` passed; active production and workspace metadata searches have no Windows service/Registered Core references. The Windows-target check was blocked before compiling repository code because the environment has no `x86_64-w64-mingw32-gcc` linker toolchain. Commits: `6b9ffcae2`, `081a33632`.
