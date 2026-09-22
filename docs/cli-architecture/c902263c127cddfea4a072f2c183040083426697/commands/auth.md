# Authentication commands

## Entry

`codex login` and `codex logout` are parsed by [`LoginCommand` and `LogoutCommand`](../../../../crates/cli/src/main.rs#L452). `login status` and logout dispatch to helpers from `codex-login` at [main.rs#L1392](../../../../crates/cli/src/main.rs#L1392).

## Behavior

- `codex login status` reports the currently selected local auth state.
- `codex logout` removes stored authentication credentials.
- These commands reject remote mode because they mutate or inspect local credential state.
- The selected auth mode affects later provider/client construction, but the login command does not itself run an agent turn.

The auth manager, credential formats, keyring, and environment selection are documented in [common authentication](../common/authentication.md). Provider selection is a separate step described in [provider and model](../common/provider-and-model.md).
