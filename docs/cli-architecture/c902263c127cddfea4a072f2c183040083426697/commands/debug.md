# `codex debug`

## Entry points

[`DebugSubcommand`](../../../../crates/cli/src/main.rs#L210) contains:

- `models [--bundled]`: render the model catalog as JSON;
- `app-server send-message-v2`: send a diagnostic app-server message;
- `prompt-input [PROMPT] --image FILE`: render model-visible prompt input as JSON;
- hidden `trace-reduce TRACE_BUNDLE`: replay a rollout trace and write reduced state JSON;
- hidden `clear-memories`: reset local memory state.

The dispatch begins at [main.rs#L1498](../../../../crates/cli/src/main.rs#L1498). These commands reject remote mode and are diagnostic/maintenance entry points rather than ordinary turn execution.

## Ownership and side effects

`debug models` uses the model manager/catalog path and can either refresh or use the bundled catalog. `prompt-input` exercises input normalization, including image attachments. `trace-reduce` reads a trace bundle and writes reduced state. `clear-memories` mutates local memory state and is intentionally hidden.

See [persistence/rollout](../common/persistence-and-rollout.md), [provider/model](../common/provider-and-model.md), and [output/errors](../common/output-and-errors.md).
