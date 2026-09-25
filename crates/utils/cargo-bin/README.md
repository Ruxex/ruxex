# codex-utils-cargo-bin

The helpers in this crate use Cargo's test environment and manifest-relative
paths.

Function behavior:
- `cargo_bin`: reads `CARGO_BIN_EXE_*` environment variables and otherwise
  falls back to `assert_cmd`'s Cargo lookup.
- `find_resource!`: locates fixtures relative to `CARGO_MANIFEST_DIR`.
