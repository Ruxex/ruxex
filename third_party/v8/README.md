# rusty_v8 artifacts

Local Cargo builds use the `v8` version pinned in the root `Cargo.toml` and
upstream prebuilt archives by default. `RUSTY_V8_ARCHIVE` and
`RUSTY_V8_SRC_BINDING_PATH` can select explicitly supplied native artifacts.
`just test` uses `scripts/download-rusty-v8.sh` to cache the matching release
and sandbox-enabled archives from the `openai/codex` GitHub release, verify
their checksums under `.build-cache/rusty-v8`, and expose the cache through
`RUSTY_V8_MIRROR`.

The checksum manifests in this directory are retained as reference metadata.
This fork does not build or publish V8 release artifacts automatically.
