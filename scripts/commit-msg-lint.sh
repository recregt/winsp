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

normalize() {
  local n="$1"
  n="${n#.}"
  n="${n%%.*}"
  n="$(printf '%s' "$n" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')"
  printf '%s' "$n"
}

scopes=()
for entry in "$repo_root"/*; do
  base="$(basename "$entry")"
  case "$base" in
    .git|target) continue ;;
  esac
  norm="$(normalize "$base")"
  [ -n "$norm" ] && scopes+=("$norm")
done

if [ -d "$repo_root/crates" ]; then
  for entry in "$repo_root"/crates/*/; do
    norm="$(normalize "$(basename "$entry")")"
    [ -n "$norm" ] && scopes+=("$norm")
  done
fi

scope_list="$(printf '%s\n' "${scopes[@]}" | sort -u | paste -sd ' ' -)"

fail() {
  echo "Invalid commit message:" >&2
  echo "  $header" >&2
  echo >&2
  echo "Expected: <type>(<scope>): <imperative description>" >&2
  echo "  scope is optional: <type>: <description> is also valid" >&2
  echo "  type:  feat fix refactor ci perf test chore docs" >&2
  echo "  scope: $scope_list" >&2
  echo "  example: fix(indexer): handle null pointer dereference in shell enumeration" >&2
  exit 1
}

struct_pattern='^([a-z]+)(\(([^()]+)\))?: (.+)$'
[[ "$header" =~ $struct_pattern ]] || fail

type="${BASH_REMATCH[1]}"
scope_group="${BASH_REMATCH[2]}"
raw_scope="${BASH_REMATCH[3]}"
description="${BASH_REMATCH[4]}"

[[ "$type" =~ ^(feat|fix|refactor|ci|perf|test|chore|docs)$ ]] || fail

if [ -n "$scope_group" ]; then
  norm_scope="$(normalize "$raw_scope")"
  match=0
  for s in "${scopes[@]}"; do
    [ "$s" = "$norm_scope" ] && { match=1; break; }
  done
  [ "$match" = 1 ] || fail
fi

[[ "$description" =~ ^[a-z0-9] ]] || fail
[[ "$description" != *. ]] || fail

case "$description" in
  added\ *|adds\ *|fixed\ *|fixes\ *|updated\ *|updates\ *|removed\ *|removes\ *|changed\ *|changes\ *|created\ *|creates\ *)
    echo "Invalid commit message:" >&2
    echo "  $header" >&2
    echo >&2
    echo "Use imperative mood: 'add' not 'added'/'adds', 'fix' not 'fixed'/'fixes', etc." >&2
    exit 1
    ;;
esac
