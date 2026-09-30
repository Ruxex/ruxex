# SDD ledger — plan: docs/superpowers/plans/2026-10-01-codex-only-product-filtering.md

Pre-flight: the task interfaces are sequential: Task 1 changes the Product wire model consumed by the existing skill/plugin filters; Task 2 removes obsolete session-list entries; Task 3 verifies the unchanged filtering pipeline against the Task 1 unknown-product representation; Task 4 regenerates and audits the final artifacts.

Task 1: complete — Product is Codex-only at runtime, unsupported serialized values become Unknown, and the compatibility comment is beside Product::Codex.
Task 2: complete — obsolete ChatGPT/Atlas interactive session filters were removed while generic custom-source decoding was retained.
Task 3: complete — skill and plugin filtering tests cover unknown-only rejection, Codex/non-Codex mixed acceptance, and unchanged empty/missing semantics.
Task 4: complete — generated output was refreshed, production references were audited, and repository verification completed.
