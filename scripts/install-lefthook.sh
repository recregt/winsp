#!/usr/bin/env bash
set -euo pipefail

version="2.1.11"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="$HOME/.local/bin"

if command -v lefthook >/dev/null 2>&1; then
  installed="$(lefthook version 2>/dev/null | head -n1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || true)"
  if [ "$installed" = "$version" ]; then
    echo "lefthook $version already installed"
    cd "$repo_root" && lefthook install
    exit 0
  fi
fi

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux) plat="Linux" ;;
  Darwin) plat="MacOS" ;;
  MINGW*|MSYS*|CYGWIN*) plat="Windows" ;;
  *) echo "unsupported OS: $os" >&2; exit 1 ;;
esac

case "$arch" in
  x86_64|amd64) plat_arch="x86_64" ;;
  arm64|aarch64) plat_arch="arm64" ;;
  *) echo "unsupported arch: $arch" >&2; exit 1 ;;
esac

asset="lefthook_${version}_${plat}_${plat_arch}"
[ "$plat" = "Windows" ] && asset="${asset}.exe"

base_url="https://github.com/evilmartians/lefthook/releases/download/v${version}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

curl -fsSL "${base_url}/${asset}" -o "${tmp_dir}/${asset}"
curl -fsSL "${base_url}/lefthook_checksums.txt" -o "${tmp_dir}/checksums.txt"

expected="$(grep " ${asset}\$" "${tmp_dir}/checksums.txt" | cut -d' ' -f1)"
[ -n "$expected" ] || { echo "no checksum entry for ${asset}" >&2; exit 1; }

actual="$(sha256sum "${tmp_dir}/${asset}" | cut -d' ' -f1)"
[ "$expected" = "$actual" ] || { echo "checksum mismatch for ${asset}" >&2; exit 1; }

mkdir -p "$bin_dir"
install_name="lefthook"
[ "$plat" = "Windows" ] && install_name="lefthook.exe"
install -m 0755 "${tmp_dir}/${asset}" "${bin_dir}/${install_name}"

case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) echo "warning: $bin_dir is not on PATH" >&2 ;;
esac

echo "installed lefthook $version to $bin_dir"
cd "$repo_root" && "${bin_dir}/${install_name}" install
