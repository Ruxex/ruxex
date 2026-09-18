#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="${repo_root}/third_party/voice/sources.json"
target="$(rustc -vV | sed -n 's/^host: //p')"

case "${target}" in
  aarch64-unknown-linux-gnu | x86_64-unknown-linux-gnu)
    platform="linux"
    cc="$(command -v cc)"
    cxx="$(command -v c++)"
    linker="$(command -v gcc)"
    build_qualifier=""
    ;;
  aarch64-apple-darwin | x86_64-apple-darwin)
    platform="macos"
    cc="$(command -v clang)"
    cxx="$(command -v clang++)"
    linker="${cc}"
    deployment_target="${CODEX_MACOSX_DEPLOYMENT_TARGET:-11.0}"
    build_qualifier="macos-${deployment_target}"
    ;;
  *)
    echo "Local GStreamer setup is not supported for ${target}." >&2
    exit 1
    ;;
esac

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "sha256sum or shasum is required." >&2
    exit 1
  fi
}

manifest_hash="$(sha256 "${manifest}")"
cache_root="${repo_root}/target/gstreamer"
archives="${cache_root}/archives"
if [[ -n "${build_qualifier}" ]]; then
  build_root="${cache_root}/${target}-${build_qualifier}-${manifest_hash}"
else
  build_root="${cache_root}/${target}-${manifest_hash}"
fi
prefix="${build_root}/prefix"

download_sources() {
  mkdir -p "${archives}"
  while IFS=$'\t' read -r archive url expected; do
    local destination="${archives}/${archive}"
    local temporary="${destination}.tmp.$$"

    if [[ -f "${destination}" ]] && [[ "$(sha256 "${destination}")" == "${expected}" ]]; then
      continue
    fi
    rm -f "${temporary}"
    curl --proto '=https' --tlsv1.2 --fail --location --retry 3 \
      --output "${temporary}" "${url}"
    if [[ "$(sha256 "${temporary}")" != "${expected}" ]]; then
      rm -f "${temporary}"
      echo "Source verification failed: ${archive}" >&2
      exit 1
    fi
    mv "${temporary}" "${destination}"
  done < <(
    python3 - "${manifest}" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    manifest = json.load(source)
for item in manifest["sources"]:
    print(item["archive"], item["url"], item["sha256"], sep="\t")
PY
  )
}

build_gstreamer() {
  if [[ -f "${build_root}/built.json" ]] && [[ -f "${prefix}/lib/pkgconfig/gstreamer-1.0.pc" ]]; then
    return
  fi

  local temporary="${build_root}.tmp.$$"
  local jobs
  local build_args=()
  if [[ "${platform}" == "macos" ]]; then
    build_args+=(--deployment-target "${deployment_target}")
    jobs="$(sysctl -n hw.ncpu 2>/dev/null || echo 4)"
  else
    jobs="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)"
  fi
  rm -rf "${temporary}"
  python3 "${repo_root}/third_party/voice/build_native.py" \
    --archives "${archives}" \
    --output "${temporary}" \
    --cc "${cc}" \
    --cxx "${cxx}" \
    --cmake "$(command -v cmake)" \
    --make "$(command -v make)" \
    --pkg-config "$(command -v pkg-config)" \
    --shell "$(command -v bash)" \
    --target "${target}" \
    --jobs "${jobs}" \
    "${build_args[@]}"
  mv "${temporary}" "${build_root}"
}

download_sources
build_gstreamer

pkg_config_path="${prefix}/lib/pkgconfig:${prefix}/share/pkgconfig"
library_path="${prefix}/lib"
plugin_path="${prefix}/lib/gstreamer-1.0"
linker_env="CARGO_TARGET_${target^^}_LINKER"
linker_env="${linker_env//-/_}"

if [[ "${1:-}" == "--" ]]; then
  shift
  if [[ "$#" -eq 0 ]]; then
    echo "Expected a command after --." >&2
    exit 2
  fi
  command_env=(
    "PKG_CONFIG_PATH=${pkg_config_path}${PKG_CONFIG_PATH:+:${PKG_CONFIG_PATH}}"
    "GST_PLUGIN_PATH_1_0=${plugin_path}"
    "GST_PLUGIN_SYSTEM_PATH_1_0=${plugin_path}"
    "CODEX_GSTREAMER_LIB_DIR=${library_path}"
    "CODEX_GSTREAMER_LINKER=${linker}"
    "${linker_env}=${repo_root}/scripts/gstreamer-linker.sh"
  )
  if [[ "${platform}" == "macos" ]]; then
    command_env+=(
      "DYLD_LIBRARY_PATH=${library_path}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
      "MACOSX_DEPLOYMENT_TARGET=${deployment_target}"
    )
  else
    command_env+=("LD_LIBRARY_PATH=${library_path}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}")
  fi
  exec env "${command_env[@]}" "$@"
fi

printf 'GSTREAMER_PREFIX=%s\n' "${prefix}"
printf 'PKG_CONFIG_PATH=%s\n' "${pkg_config_path}"
if [[ "${platform}" == "macos" ]]; then
  printf 'DYLD_LIBRARY_PATH=%s\n' "${library_path}"
  printf 'MACOSX_DEPLOYMENT_TARGET=%s\n' "${deployment_target}"
else
  printf 'LD_LIBRARY_PATH=%s\n' "${library_path}"
fi
