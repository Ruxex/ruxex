# Common configuration flow

## Normalized order

```text
root CLI flags
  -> CliConfigOverrides / SharedCliOptions
  -> CODEX_HOME and current directory
  -> profile and config.toml layers
  -> loader overrides and harness overrides
  -> ConfigBuilder
  -> immutable runtime Config
```

The CLI helper [`cloud_config::config_builder`](../../../../crates/cli/src/cloud_config.rs#L20) parses `-c` overrides, resolves `CODEX_HOME`, resolves the effective CWD, and constructs [`codex_core::config::ConfigBuilder`](../../../../crates/core/src/config/mod.rs#L1290). The builder combines persisted/user/project layers with command-specific overrides before runtime consumers receive `Config`.

`codex-utils-cli` owns shared argument shapes such as [`SharedCliOptions`](../../../../crates/utils/cli/src/shared_options.rs#L9), while `codex-cli`, `codex-exec`, and `codex-tui` decide which shared flags are exposed/global for their command.

## Precedence

Command-specific values and explicit CLI overrides have higher intent than loaded defaults. Profile selection changes the config layer being loaded; it is not the same as arbitrary `-c` key/value override. Strict-config changes unknown-field behavior and is rejected for commands where config loading is not meaningful.

## Trait boundary

[`ThreadConfigLoader`](../reference/trait-index.md#threadconfigloader) is the main capability-relevant Trait in the config crate. It lets app-server/core load thread-scoped configuration without making the loader depend on the whole CLI.
