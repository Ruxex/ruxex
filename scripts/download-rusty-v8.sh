#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
v8_version="$(sed -n 's/^v8 = "=\([^"]*\)"$/\1/p' "${repo_root}/Cargo.toml")"
target="$(rustc -vV | sed -n 's/^host: //p')"

if [[ -z "${v8_version}" ]]; then
  echo "Could not resolve the pinned v8 version from Cargo.toml." >&2
  exit 1
fi

case "${target}" in
  aarch64-apple-darwin | aarch64-unknown-linux-gnu | aarch64-unknown-linux-musl | x86_64-apple-darwin | x86_64-unknown-linux-gnu | x86_64-unknown-linux-musl)
    ;;
  *)
    echo "No prebuilt sandboxed rusty_v8 artifact is available for ${target}." >&2
    exit 1
    ;;
esac

mirror_root="${repo_root}/.build-cache/rusty-v8"
download_dir="${mirror_root}/v${v8_version}"
release_url="https://github.com/codex/codex/releases/download/rusty-v8-v${v8_version}"
release_manifest="${repo_root}/third_party/v8/rusty_v8_${v8_version//./_}_release_manifests.sha256"

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

download() {
  local url="$1"
  local destination="$2"
  local temporary="${destination}.tmp.$$"

  trap 'rm -f "${temporary}"' RETURN
  curl --proto '=https' --tlsv1.2 --fail --location --retry 3 \
    --output "${temporary}" "${url}"
  mv "${temporary}" "${destination}"
  trap - RETURN
}

mkdir -p "${download_dir}"
download_variant() {
  local variant="$1"
  local asset_prefix="rusty_v8_${variant}release_${target}"
  local archive_name="lib${asset_prefix}.a.gz"
  local binding_name="src_binding_${variant}release_${target}.rs"
  local checksum_name="${asset_prefix}.sha256"
  local archive_path="${download_dir}/${archive_name}"
  local binding_path="${download_dir}/${binding_name}"
  local checksum_path="${download_dir}/${checksum_name}"
  local expected_checksum_manifest_hash

  expected_checksum_manifest_hash="$(awk -v name="${checksum_name}" '$2 == name { print $1 }' "${release_manifest}")"
  if [[ -z "${expected_checksum_manifest_hash}" ]]; then
    echo "No checksum is pinned for ${checksum_name}." >&2
    exit 1
  fi
  if [[ ! -f "${checksum_path}" ]] || [[ "$(sha256 "${checksum_path}")" != "${expected_checksum_manifest_hash}" ]]; then
    download "${release_url}/${checksum_name}" "${checksum_path}"
  fi
  if [[ "$(sha256 "${checksum_path}")" != "${expected_checksum_manifest_hash}" ]]; then
    echo "Checksum manifest verification failed: ${checksum_path}" >&2
    exit 1
  fi

  download_and_verify "${archive_name}" "${archive_path}" "${checksum_path}"
  download_and_verify "${binding_name}" "${binding_path}" "${checksum_path}"
}

download_and_verify() {
  local name="$1"
  local destination="$2"
  local checksum_path="$3"
  local expected
  expected="$(awk -v name="${name}" '$2 == name { print $1 }' "${checksum_path}")"

  if [[ -z "${expected}" ]]; then
    echo "No checksum is published for ${name}." >&2
    exit 1
  fi
  if [[ ! -f "${destination}" ]] || [[ "$(sha256 "${destination}")" != "${expected}" ]]; then
    download "${release_url}/${name}" "${destination}"
  fi
  if [[ "$(sha256 "${destination}")" != "${expected}" ]]; then
    echo "Artifact verification failed: ${destination}" >&2
    exit 1
  fi
}

# Cargo may enable the sandbox feature for a full workspace run but omit it for
# a package-scoped run. Populate both names so rusty_v8 can select the matching
# archive and bindings itself.
download_variant ""
download_variant "ptrcomp_sandbox_"

if [[ "${1:-}" == "--" ]]; then
  shift
  if [[ "$#" -eq 0 ]]; then
    echo "Expected a command after --." >&2
    exit 2
  fi
  RUSTY_V8_MIRROR="${mirror_root}" \
    exec "$@"
fi

printf 'RUSTY_V8_MIRROR=%s\n' "${mirror_root}"
