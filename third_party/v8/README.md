# rusty_v8 artifacts

Local Cargo builds use the `v8` version pinned in the root `Cargo.toml` and
upstream prebuilt archives by default. `RUSTY_V8_ARCHIVE` and
`RUSTY_V8_SRC_BINDING_PATH` can select explicitly supplied native artifacts.

The checksum manifests in this directory are retained as reference metadata.
This fork does not build or publish V8 release artifacts automatically.
