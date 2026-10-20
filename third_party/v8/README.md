# rusty_v8 artifacts

Local Cargo builds use the `v8` version pinned in the root `Cargo.toml` and the
sandbox-enabled configuration required by `code-mode-runtime`.
`RUSTY_V8_ARCHIVE` and `RUSTY_V8_SRC_BINDING_PATH` can select explicitly
supplied native artifacts. `just` recipes use
`scripts/download-rusty-v8.sh` to cache the matching sandbox archive and
bindings, verify their checksums under `.build-cache/rusty-v8`, and expose the
cache through `RUSTY_V8_MIRROR`.

The supported Ruxex build path currently uses the historical `openai/codex`
release for the sandbox-enabled variant. The official `rusty_v8` release does
not publish a prebuilt `v8_enable_sandbox` archive for this pin, and Ruxex does
not support silently falling back to an ordinary shared-cage archive.

The Cargo dependency does not itself distribute a V8 binary. The `v8` crate's
build script selects an archive by version, target, profile, and enabled
features. A library consumer therefore either uses the official prebuilt
path for supported features, supplies `RUSTY_V8_ARCHIVE`/
`RUSTY_V8_MIRROR`, or builds the sandbox-enabled variant with
`V8_FROM_SOURCE=1`. Ruxex's supported recipes require the matching sandbox
variant; a consumer who builds the crate directly must provide the same
sandbox archive or a reproducible source-build equivalent. Ruxex does not
redistribute a V8 binary.

The checksum manifests in this directory are retained as reference metadata.
This fork does not build or publish V8 release artifacts automatically.
