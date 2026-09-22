# Feature flags

## Resolution

The root parser folds `--enable` and `--disable` into config overrides before dispatch. `codex features list` loads effective configuration and reports each feature's key, stage, and enabled state; `enable` and `disable` edit persistent config. Feature definitions are in `codex-features` and are consumed by core/CLI gates.

## Scope

Feature flags are configuration/runtime gates, not a replacement for clap subcommands. A command can be syntactically available while a feature gate rejects or changes its runtime behavior. Conversely, hidden/internal commands may be available for plumbing without being a feature users can enable.

See [`FeaturesSubcommand`](../../../../crates/cli/src/main.rs#L897), [CLI entrypoint](../01-cli-entrypoint.md), and [configuration](configuration.md).
