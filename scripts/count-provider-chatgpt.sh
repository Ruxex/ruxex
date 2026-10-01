#!/usr/bin/env bash

set -euo pipefail

output_file="${1:-provider-chatgpt-line-counts.tsv}"
output_dir="$(dirname -- "$output_file")"

if [[ ! -d "$output_dir" ]]; then
  mkdir -p -- "$output_dir"
fi

tmp_file="$(mktemp)"
trap 'rm -f -- "$tmp_file"' EXIT

# `rg -l` finds files containing either term. Count matching lines per file
# separately so the output remains unambiguous even when a filename contains `:`.
{
  rg -i -l -0 \
    --glob '*.rs' \
    --glob '!.git/**' \
    --glob '!target/**' \
    --glob '!scripts/.venv/**' \
    'provider|chatgpt' . || {
      status=$?
      [[ "$status" -eq 1 ]]
    }
} |
  while IFS= read -r -d '' file; do
    if [[ "$file" != "./$output_file" && "$file" != "$output_file" ]]; then
      count="$(rg -i -c 'provider|chatgpt' -- "$file")"
      printf '%s\t%s\n' "${file#./}" "$count"
    fi
  done |
  sort >"$tmp_file"

{
  printf 'file\tmatching_lines\n'
  cat -- "$tmp_file"
} >"$output_file"

printf '結果を %s に記録しました。\n' "$output_file"
