# Project direction

This repository turns Codex into a general-purpose AI agent foundation by removing OpenAI-specific features and assumptions.

Keep the shared architecture provider-neutral and suitable for building a wide range of AI agents.

Work directly on the `main` branch in this repository. Do not create Git worktrees for task work.

Current work is Level 1 Phase 3 of `docs/openai-decoupling-plan.md`. The hosted config integration, `codex-git-attribution`, and the remote-memory startup rate-limit guard with the memory writer's direct `codex-backend-client` dependency have been removed. Resume with the remaining account/usage/workspace boundaries; do not mark Phase 3 complete until the phase's remaining integrations have been handled.

For crate-removal work in this phase, test-first development is not required. Treat `cargo check` (and the repository-wide `just check` when applicable) as the primary verification instead of adding deletion-specific tests first.
