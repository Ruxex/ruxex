# General-purpose AI Agent Foundation (Rust)

This repository is a general-purpose AI agent foundation. Provider-specific features
and assumptions are removed so the
runtime can provide a provider-neutral base for building a wide range of AI
agents.

## Repository snapshot

- Snapshot commit: `fe8228b3d945a857502e1981e9e7d57f17fe3ae6`
- Approximate size: 1,958,791 lines across 7,207 Git-tracked files

Approximate line count by file type:

| Category | Lines |
| --- | ---: |
| Rust | 1,678,472 |
| TypeScript | 7,502 |
| Python | 25,438 |
| SQL | 806 |
| Shell/PowerShell | 2,746 |
| C/C headers | 6,265 |
| Protocol Buffers | 383 |
| Markdown | 14,274 |
| Config/data (JSON, TOML, YAML, XML, lockfiles) | 178,104 |
| Other tracked files | 44,801 |

The line count includes source, tests, documentation, and generated files, and
excludes untracked and ignored files. Categories are grouped by file extension.

## Current repository snapshot

- Snapshot commit: `f42f440d72a7471fc36264cc71de168970e13b50` (`Remove Guardian provider integration`)
- Approximate size at this commit: 1,755,036 lines across 6,651 Git-tracked files

Approximate line count by file type at this commit:

| Category | Lines |
| --- | ---: |
| Rust | 1,500,917 |
| TypeScript | 6,608 |
| Python | 25,438 |
| SQL | 794 |
| Shell/PowerShell | 2,500 |
| C/C headers | 6,265 |
| Protocol Buffers | 338 |
| Markdown | 15,556 |
| Config/data (JSON, TOML, YAML, XML, lockfiles) | 161,081 |
| Other tracked files | 35,539 |

This snapshot uses the same scope as the preceding one: source, tests,
documentation, and generated files are included, while untracked and ignored
files are excluded.

## Current repository snapshot

- Snapshot commit: `1b846427c7a092b96acb97ced6148d43cc1e1e23` (`Remove residual ChatGPT auth artifacts`)
- Approximate size at this commit: 1,632,277 lines across 6,359 Git-tracked files

Approximate line count by file type at this commit:

| Category | Lines |
| --- | ---: |
| Rust | 1,267,670 |
| TypeScript | 6,455 |
| Python | 25,346 |
| SQL | 753 |
| Shell/PowerShell | 2,503 |
| C/C headers | 6,265 |
| Protocol Buffers | 269 |
| Markdown | 18,690 |
| Config/data (JSON, TOML, YAML, XML, lockfiles) | 269,814 |
| Other tracked files | 34,512 |

This snapshot uses the same scope as the preceding one: source, tests,
documentation, and generated files are included, while untracked and ignored
files are excluded.

The Rust implementation remains in a root Cargo workspace. Member crates live
under `crates/`; shared utility and extension crates retain their existing
subdirectories. Vendored bubblewrap sources live in `crates/vendor/` to preserve
the crate's build-time relative paths.

## Build locally

Install Rust with rustup and the native build dependencies for your platform.
The pinned toolchain is selected by `rust-toolchain.toml`.

```sh
cargo check --locked -p genex-cli
cargo build --locked --release -p genex-cli --bin genex
./target/release/genex
```

On Windows, the executable is `target/release/genex.exe`.
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
