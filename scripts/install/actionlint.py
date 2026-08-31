#!/usr/bin/env python3
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

from common import (
    atomic_install,
    checksum_for,
    download,
    extract_binary,
    fail,
    sha256_of,
)

VERSION = "1.7.12"

PLATFORMS = {
    "Linux": "linux",
    "Darwin": "darwin",
    "Windows": "windows",
}

ARCHES = {
    "x86_64": "amd64",
    "amd64": "amd64",
    "AMD64": "amd64",
    "arm64": "arm64",
    "aarch64": "arm64",
}


def installed_version() -> str | None:
    actionlint = shutil.which("actionlint")
    if not actionlint:
        return None
    out = subprocess.run(
        [actionlint, "-version"], capture_output=True, text=True, encoding="utf-8", check=False
    )
    lines = out.stdout.splitlines()
    return lines[0].strip() if lines else None


def target_platform() -> str:
    system = platform.system()
    plat = PLATFORMS.get(system)
    if not plat:
        fail(f"unsupported OS: {system}")
    return plat


def target_arch() -> str:
    machine = platform.machine()
    arch = ARCHES.get(machine)
    if not arch:
        fail(f"unsupported arch: {machine}")
    return arch


def main() -> None:
    bin_dir = Path.home() / ".cargo" / "bin"

    if installed_version() == VERSION:
        print(f"actionlint {VERSION} already installed", flush=True)
        return

    plat = target_platform()
    arch = target_arch()
    ext = "zip" if plat == "windows" else "tar.gz"
    asset = f"actionlint_{VERSION}_{plat}_{arch}.{ext}"
    member = "actionlint.exe" if plat == "windows" else "actionlint"

    base_url = f"https://github.com/rhysd/actionlint/releases/download/v{VERSION}"

    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        asset_path = tmp_dir / asset
        checksums_path = tmp_dir / "checksums.txt"

        download(f"{base_url}/{asset}", asset_path)
        download(f"{base_url}/actionlint_{VERSION}_checksums.txt", checksums_path)

        expected = checksum_for(checksums_path, asset)
        if sha256_of(asset_path) != expected:
            fail(f"checksum mismatch for {asset}")

        extracted = extract_binary(asset_path, member, tmp_dir)

        bin_dir.mkdir(parents=True, exist_ok=True)
        atomic_install(extracted, bin_dir, member)

    print(f"installed actionlint {VERSION} to {bin_dir}")


if __name__ == "__main__":
    main()
