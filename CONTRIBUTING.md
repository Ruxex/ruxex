# Contributing to Ruxex

Ruxex welcomes bug reports, design discussions, documentation improvements,
and pull requests that improve the library and its supported host surfaces.

## Before starting

For a substantial change, open an issue or draft pull request first. Explain
the problem, the intended boundary, and how the change will be verified. Keep
provider-specific behavior behind the provider boundary and preserve external
endpoint, authentication, and model contracts unless the change explicitly
updates that contract.

During the pre-release migration, use `develop` unless a pull request says
otherwise. The `main` branch is the publication target.

## Local verification

The repository uses the Rust toolchain declared by `rust-toolchain.toml` and
the `just` recipes. Before submitting a change, run the checks relevant to the
files you touched:

```text
just check
just check --tests
just generate
```

Rust commands should be run through the repository recipes and the documented
memory-limited user systemd scope. Do not commit build caches, credentials,
generated files that are not checked in, or local machine paths.

Changes to persisted data, protocol schemas, provider contracts, build inputs,
or public names must include the corresponding producer/consumer audit and
compatibility decision in the repository's migration records while those
records are still maintained.

## Pull requests

Keep pull requests focused and describe:

- the user-visible or host-visible behavior;
- the compatibility and security impact;
- the verification commands and results; and
- any follow-up work that is intentionally deferred.

Do not include secrets or private service data in source, tests, logs, or issue
reports. The CI matrix currently covers Linux and macOS. Windows and standalone
SDK/package publishing are not supported by this repository at this time.

## Security issues

Do not report vulnerabilities in a public issue. Follow the [security
policy](SECURITY.md) for private reporting.

## Licensing and provenance

By contributing, you agree that your contribution is provided under the
repository's Apache-2.0 license unless a different applicable notice is stated
for the affected file. Preserve existing copyright, third-party license, and
upstream provenance notices.
