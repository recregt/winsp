#!/usr/bin/env bash
set -euo pipefail
shopt -s dotglob nullglob

msg_file="$1"
header="$(head -n1 "$msg_file")"

case "$header" in
  "Merge "*|"Revert "*|"fixup! "*|"squash! "*)
    exit 0
    ;;
esac

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

description="${header#*: }"
case "$description" in
  added\ *|adds\ *|fixed\ *|fixes\ *|updated\ *|updates\ *|removed\ *|removes\ *|changed\ *|changes\ *|created\ *|creates\ *)
    echo "Invalid commit message:" >&2
    echo "  $header" >&2
    echo >&2
    echo "Use imperative mood: 'add' not 'added'/'adds', 'fix' not 'fixed'/'fixes', etc." >&2
    exit 1
    ;;
esac
