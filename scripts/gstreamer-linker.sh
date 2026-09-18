#!/usr/bin/env bash

set -euo pipefail

: "${CODEX_GSTREAMER_LIB_DIR:?CODEX_GSTREAMER_LIB_DIR is required}"
: "${CODEX_GSTREAMER_LINKER:?CODEX_GSTREAMER_LINKER is required}"

# Keep the project linker configuration, but put the local GStreamer prefix
# before system library directories emitted by other native dependencies.
exec "${CODEX_GSTREAMER_LINKER}" "-L${CODEX_GSTREAMER_LIB_DIR}" "$@"
