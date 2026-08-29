#!/usr/bin/env bash
set -euo pipefail
shopt -s dotglob nullglob

msg_file="$1"
sha="${2:-}"
header="$(head -n1 "$msg_file")"

case "$header" in
  "Merge "*|"Revert "*|"fixup! "*|"squash! "*)
    exit 0
    ;;
esac

header_len=${#header}
if [ "$header_len" -gt 50 ]; then
  echo "Invalid commit message:" >&2
  echo "  $header" >&2
  echo >&2
  echo "Subject line is $header_len characters, must be 50 or fewer." >&2
  exit 1
fi

mapfile -t body_lines < <(tail -n +3 "$msg_file")
for line in "${body_lines[@]}"; do
  line_len=${#line}
  if [ "$line_len" -gt 72 ]; then
    echo "Invalid commit message body:" >&2
    echo "  $line" >&2
    echo >&2
    echo "Body line is $line_len characters, must be 72 or fewer." >&2
    exit 1
  fi
done

repo_root="$(git rev-parse --show-toplevel)"

escape_re() {
  local s="$1" out="" c i
  for (( i=0; i<${#s}; i++ )); do
    c="${s:$i:1}"
    case "$c" in
      '.'|'^'|'$'|'*'|'+'|'?'|'('|')'|'['|']'|'{'|'}'|'|'|'\')
        out+="\\$c" ;;
      *)
        out+="$c" ;;
    esac
  done
  printf '%s' "$out"
}

scopes=()
for entry in "$repo_root"/*; do
  base="$(basename "$entry")"
  case "$base" in
    .git|target) continue ;;
  esac
  scopes+=("$base")
done

if [ -d "$repo_root/crates" ]; then
  for entry in "$repo_root"/crates/*/; do
    scopes+=("$(basename "$entry")")
  done
fi

mapfile -t scopes < <(printf '%s\n' "${scopes[@]}" | sort -u)
scope_list="$(printf '%s' "${scopes[*]}")"

fail() {
  echo "Invalid commit message:" >&2
  echo "  $header" >&2
  echo >&2
  echo "Expected: <type>(<scope>): <imperative description>" >&2
  echo "  scope is optional: <type>: <description> is also valid" >&2
  echo "  scope must match a file or folder name exactly, as it appears in the repo" >&2
  echo "  type:  feat fix refactor ci perf test chore docs" >&2
  echo "  scope: $scope_list" >&2
  echo "  example: fix(indexer): handle null pointer dereference in shell enumeration" >&2
  exit 1
}

types="feat|fix|refactor|ci|perf|test|chore|docs"
scope_alt=""
for s in "${scopes[@]}"; do
  esc="$(escape_re "$s")"
  scope_alt="${scope_alt:+${scope_alt}|}${esc}"
done
scope_part=""
[ -n "$scope_alt" ] && scope_part="(\\((${scope_alt})\\))?"
pattern="^(${types})${scope_part}: [a-z0-9].*[^.]\$"

[[ "$header" =~ $pattern ]] || fail

full_pattern="^(${types})${scope_part}: (.*)\$"
[[ "$header" =~ $full_pattern ]]
commit_type="${BASH_REMATCH[1]}"
written_scope="${BASH_REMATCH[3]}"
description="${BASH_REMATCH[4]}"

case "$description" in
  added\ *|adds\ *|fixed\ *|fixes\ *|updated\ *|updates\ *|removed\ *|removes\ *|changed\ *|changes\ *|created\ *|creates\ *)
    echo "Invalid commit message:" >&2
    echo "  $header" >&2
    echo >&2
    echo "Use imperative mood: 'add' not 'added'/'adds', 'fix' not 'fixed'/'fixes', etc." >&2
    exit 1
    ;;
esac

if [ -n "$sha" ]; then
  mapfile -t changed_files < <(git diff-tree --no-commit-id --name-only -r "$sha")
else
  mapfile -t changed_files < <(git diff --cached --name-only)
fi

[ "${#changed_files[@]}" -eq 0 ] && exit 0

touched=()
all_crate_scoped=1
for path in "${changed_files[@]}"; do
  first="${path%%/*}"
  if [ "$first" = "crates" ] && [[ "$path" == crates/*/* ]]; then
    rest="${path#crates/}"
    touched+=("${rest%%/*}")
  else
    touched+=("$first")
    all_crate_scoped=0
  fi
done

mapfile -t touched < <(printf '%s\n' "${touched[@]}" | sort -u)

fail_scope() {
  echo "Invalid commit message:" >&2
  echo "  $header" >&2
  echo >&2
  echo "$1" >&2
  echo "  files touched: ${changed_files[*]}" >&2
  exit 1
}

if [ "${#touched[@]}" -eq 1 ]; then
  if [ -n "$written_scope" ] && [ "$written_scope" != "${touched[0]}" ]; then
    fail_scope "This commit only touches '${touched[0]}', but the scope says '$written_scope'."
  fi
  if [ "${touched[0]}" = ".github" ] && [ "$commit_type" != "ci" ]; then
    fail_scope "This commit only touches '.github', use type 'ci' (not '$commit_type') so CHANGELOG.md skips it."
  fi
elif [ "$all_crate_scoped" -eq 1 ]; then
  if [ -n "$written_scope" ] && [ "$written_scope" != "crates" ]; then
    fail_scope "This commit touches multiple crates (${touched[*]}); use scope 'crates' or omit the scope."
  fi
else
  if [ -n "$written_scope" ]; then
    fail_scope "This commit touches multiple unrelated areas (${touched[*]}); omit the scope."
  fi
fi
