# Native voice development helpers

This directory contains pinned source metadata, license notices, and local
build helpers for native voice dependencies. See `sources.json` for pinned
versions and `NOTICE.md` and `licenses/` for attribution.

- `prepare_sources.py` verifies and prepares source archives.
- `build_native.py` builds a native dependency prefix.
- `sdk.py` prepares native SDK metadata.
- `linux_runtime.py`, `macos_runtime.py`, and `windows_runtime.py` handle
  platform runtime resources.
- `windows_build_inputs.py` and `windows_crt.py` describe Windows build inputs.

Use each script's `--help` for its inputs. These helpers are retained for local
native development; there are no automatic builds or publishing workflows.

On GNU Linux and macOS, `just check` and `just test` invoke
`scripts/setup-gstreamer.sh` to download the pinned sources, build a cached
native prefix under `.build-cache/gstreamer`, and expose it to Cargo. The initial
build requires Python 3.12+, CMake, pkg-config, make, and a native C/C++
toolchain. macOS builds default to a deployment target of 11.0; set
`CODEX_MACOSX_DEPLOYMENT_TARGET` to override it explicitly.
