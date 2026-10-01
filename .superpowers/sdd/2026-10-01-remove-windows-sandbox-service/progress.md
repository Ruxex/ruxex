# SDD ledger — plan: docs/superpowers/plans/2026-10-01-remove-windows-sandbox-service.md

Workspace: main checkout, per repository instructions; no worktree.

Pre-flight: Tasks 1–5 share the workspace/build/config interfaces in sequence. Task 1 removes the service crate and feature surface; Task 2 removes the library APIs consumed by Task 3; Task 3 removes app-server/sandboxing callers; Task 4 documents the resulting active surface; Task 5 verifies the complete branch.

Task 1: Ruling: the environment did not provide the plan skill's documented `task-start` helper, so the available `task-brief` helper was used instead; the task scope and BASE were unchanged.

Task 1: TDD exception: this task only removes a crate and configuration/generated surfaces; repository instructions explicitly state that test-first development is not required for crate-removal work.

Task 1: complete — `just generate` succeeded; `cargo metadata --no-deps` succeeded; the service crate/feature/config search is clear except for the Windows sandbox library's service APIs intentionally deferred to Task 2; commit `e9b7bf858`.
