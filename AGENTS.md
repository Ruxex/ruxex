# Ruxex agent instructions

## Project

Ruxex is an unreleased Rust library and agent-runtime foundation derived from
Codex. Preserve its independent Ruxex identity while retaining required
copyright, license, third-party, and upstream provenance notices.

The runtime receives one model provider at startup. Keep provider-specific HTTP,
credentials, wire formats, and errors behind the provider boundary. Threads and
rollouts must not select providers or persist credentials. A different provider
uses another runtime and data root. The default root is `~/.ruxex`, overridden
by `RUXEX_HOME`, and identified by `ruxex-home.json` before sensitive state is
opened.

## OSS boundaries

- Use Ruxex names for repository-owned code and documentation.
- Preserve names required by external protocols, endpoint URLs, fields,
  authentication variables/headers, model IDs, third-party packages, and
  legal or historical attribution.
- Do not add private plans, `.codex`, `.vscode`, `.superpowers`, credentials,
  generated local state, or machine-specific paths to the public tree.

## Development and verification

Use the toolchain in `rust-toolchain.toml` and the repository `just` recipes.
The supported CI platforms are Linux and macOS. Rusty V8 artifacts are prepared
for repository development by `scripts/download-rusty-v8.sh`; Ruxex does not
distribute those native binaries.

Use:

```text
just fmt-check
just check
just check --tests   # when Rust or test code changes
just generate       # when checked-in generated files may change
just clean          # removes target/ and .build-cache/rusty-v8/
```

Run build-producing recipes in the memory-limited user systemd scope; use
`CARGO_BUILD_JOBS=4` for crate-local work and `2` for large builds:

```sh
systemd-run --user --scope --collect --quiet \
  -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
  --setenv=CARGO_BUILD_JOBS=2 just check --tests
```

Do not invoke Cargo, `rustc`, linkers, or build scripts directly for repository
builds. Before reporting completion, run `git diff --check` and inspect the
diff.

## Git

Work in the current checkout; do not create worktrees unless requested.
Preserve unrelated changes. Do not reset, rewrite history, change remotes,
push, or create pull requests unless explicitly requested.

For security reports, follow [`SECURITY.md`](SECURITY.md). Contributor and
licensing guidance is in [`CONTRIBUTING.md`](CONTRIBUTING.md).
