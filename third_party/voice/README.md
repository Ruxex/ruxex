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
