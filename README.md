# Codex (Rust)

This fork keeps the Rust implementation in a root Cargo workspace. Member crates
live under `crates/`; shared utility and extension crates retain their existing
subdirectories. Vendored bubblewrap sources live in `crates/vendor/` to preserve
the crate's build-time relative paths.

## Build locally

Install Rust with rustup and the native build dependencies for your platform.
The pinned toolchain is selected by `rust-toolchain.toml`.

```sh
cargo check --locked -p codex-cli
cargo build --locked --release -p codex-cli --bin codex
./target/release/codex
```

On Windows, the executable is `target/release/codex.exe`.
Release artifacts are built locally and attached to releases manually. Features
that launch helper executables or use native resources still need those resources
beside the CLI; building the CLI alone does not assemble a complete runtime package.

## Repository layout

- `Cargo.toml`, `Cargo.lock`: workspace definition and locked dependencies.
- `crates/`: Rust crates and their unchanged source, tests, and generated schemas.
- `scripts/`, `tools/`: local development helpers.
- `third_party/`: native dependency sources, metadata, and development helpers.

Standalone TypeScript/Python SDKs, package publishing, GitHub Actions, and Bazel
build infrastructure are not maintained in this fork. The app-server protocol's
generated TypeScript and JSON schemas remain in
`crates/app-server-protocol/schema/`.

Use `just write-app-server-schema` to regenerate only the app-server schemas.
`just fmt` formats repository sources; `just test -p <crate>` runs selected tests
with cargo-nextest. Python and uv remain development dependencies for the helpers.

The Rust implementation's install detection and update behavior are unchanged.
