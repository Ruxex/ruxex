# Project direction

This repository turns Codex into a general-purpose AI agent foundation by removing OpenAI-specific features and assumptions.

Keep the shared architecture provider-neutral and suitable for building a wide range of AI agents.

Current work is Level 2 Phase 1 of `docs/openai-decoupling-plan.md`; update the plan when the phase is complete.

For crate-removal work in this phase, test-first development is not required. Treat `cargo check` (and the repository-wide `just check` when applicable) as the primary verification instead of adding deletion-specific tests first.
