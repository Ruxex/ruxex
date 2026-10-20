# ruxex

This is a **crates.io placeholder** for the Ruxex project. It is published as
version `0.0.0` to reserve the package name and to make an accidental registry
dependency easy to recognize.

It is not the Ruxex implementation and provides no Ruxex runtime functionality.

## Use the Git dependency

Use the implementation from the Ruxex Git repository instead:

```toml
[dependencies]
ruxex = {
    git = "https://github.com/Ruxex/ruxex.git",
    tag = "v0.1.0"
}
```

Pin the dependency to a reviewed commit or release tag. The Git source is
required because Ruxex currently depends on Ruxex-maintained revisions of
some upstream crates, including proxy support that is not available from the
corresponding crates.io releases.

If this package was selected by Cargo, check that the dependency does not use
`ruxex = "0.0.0"` or another registry-only specification. The placeholder
prints a build warning, and calling `ruxex::notice()` prints this guidance at
runtime. The actual Ruxex API is intentionally absent; only the `notice()`
helper is provided, so code that expects the implementation will fail to
compile against this package.

## Publishing this placeholder

Publishing is intentionally manual. Do not add a CI publishing workflow for
this repository.

From the repository root, run:

```sh
cargo package --manifest-path placeholder/Cargo.toml --list
cargo publish --manifest-path placeholder/Cargo.toml --dry-run
cargo publish --manifest-path placeholder/Cargo.toml
```

The package version is intentionally `0.0.0`. Do not change it unless the
purpose of this directory changes; the implementation belongs in the Ruxex
Git repository.
