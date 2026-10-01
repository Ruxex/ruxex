#!/usr/bin/env bash

set -euo pipefail

output_file="${1:-chatgpt-openai-line-counts.tsv}"
output_dir="$(dirname -- "$output_file")"

if [[ ! -d "$output_dir" ]]; then
  mkdir -p -- "$output_dir"
fi

needs_review_file="${output_file%.tsv}-needs-review.tsv"
tmp_details="$(mktemp)"
trap 'rm -f -- "$tmp_details"' EXIT

# `rg -l` finds files containing either term. The per-file awk pass classifies
# each matching line as documented by a retention rationale or needing review.
{
  rg -i -l -0 \
    --glob '!docs/**' \
    --glob '!.git/**' \
    --glob '!target/**' \
    --glob '!scripts/.venv/**' \
    --glob '!scripts/count-provider-chatgpt.sh' \
    'chatgpt|openai' . || {
      status=$?
      [[ "$status" -eq 1 ]]
    }
} |
  while IFS= read -r -d '' file; do
    if [[ "$file" != "./$output_file" &&
      "$file" != "$output_file" &&
      "$file" != "./$needs_review_file" &&
      "$file" != "$needs_review_file" ]]; then
      awk -v file="${file#./}" '
        function trim(value) {
          sub(/^[[:space:]]+/, "", value)
          sub(/[[:space:]]+$/, "", value)
          return value
        }

        function is_comment_only(value, trimmed) {
          trimmed = trim(value)
          return trimmed ~ /^\/\// ||
            trimmed ~ /^#/ ||
            trimmed ~ /^\/\*/ ||
            trimmed ~ /^\*/ ||
            trimmed ~ /^<!--/ ||
            trimmed ~ /^--/
        }

        function inline_comment(value, comment_start) {
          if (match(value, /[[:space:]]\/\/|[[:space:]]#|<!--|[[:space:]]\/\*|[[:space:]]--/)) {
            return substr(value, RSTART)
          }
          return ""
        }

        function has_retention_rationale(value) {
          value = tolower(value)
          return value ~ /(external|wire contract|protocol|compatib|keep|retain|remain|remaining|required|necessary|needed|must|preserv|unchanged|intentional|deliberate|not shared|not a codex naming choice|do not remove|should stay|exact path|first-party contract)/
        }

        {
          line = $0
          if (trim(line) == "") {
            previous_comments = ""
            rationale_active = 0
          }

          if (tolower(line) ~ /chatgpt|openai/) {
            context = previous_comments
            if (is_comment_only(line)) {
              context = context "\n" line
            } else {
              context = context "\n" inline_comment(line)
            }
            status = (rationale_active || has_retention_rationale(context)) ? "commented" : "needs-review"
            printf "%s\t%d\t%s\n", file, NR, status
          }

          if (is_comment_only(line)) {
            if (previous_comments == "") {
              previous_comments = line
            } else {
              previous_comments = previous_comments "\n" line
            }
            rationale_active = has_retention_rationale(previous_comments)
          } else {
            previous_comments = ""
          }
        }
      ' "$file"
    fi
  done |
  sort -t $'\t' -k1,1 -k2,2n >"$tmp_details"

{
  printf 'file\tmatching_lines\tcommented_lines\tneeds_review_lines\n'
  awk -F '\t' '
    {
      matching[$1]++
      if ($3 == "commented") {
        commented[$1]++
      } else {
        needs_review[$1]++
      }
    }
    END {
      for (file in matching) {
        printf "%s\t%d\t%d\t%d\n", file, matching[file], commented[file] + 0, needs_review[file] + 0
      }
    }
  ' "$tmp_details" | sort
  total_matching="$(awk -F '\t' 'END { print NR + 0 }' "$tmp_details")"
  total_commented="$(awk -F '\t' '$3 == "commented" { count++ } END { print count + 0 }' "$tmp_details")"
  total_needs_review="$(awk -F '\t' '$3 == "needs-review" { count++ } END { print count + 0 }' "$tmp_details")"
  printf 'TOTAL\t%s\t%s\t%s\n' "$total_matching" "$total_commented" "$total_needs_review"
} >"$output_file"

{
  printf 'file\tline\n'
  awk -F '\t' '$3 == "needs-review" { print $1 "\t" $2 }' "$tmp_details"
} >"$needs_review_file"

printf '結果を %s に記録しました。\n' "$output_file"
printf '要精査一覧を %s に記録しました。\n' "$needs_review_file"
