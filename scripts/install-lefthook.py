#!/usr/bin/env python3
import hashlib
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

VERSION = "2.1.12"

PLATFORMS = {
    "Linux": "Linux",
    "Darwin": "MacOS",
    "Windows": "Windows",
}

ARCHES = {
    "x86_64": "x86_64",
    "amd64": "x86_64",
    "AMD64": "x86_64",
    "arm64": "arm64",
    "aarch64": "arm64",
}


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def repo_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    )
    return Path(out.stdout.strip())


def installed_version() -> str | None:
    lefthook = shutil.which("lefthook")
    if not lefthook:
        return None
    out = subprocess.run(
        [lefthook, "version"], capture_output=True, text=True, encoding="utf-8", check=False
    )
    match = re.search(r"\d+\.\d+\.\d+", out.stdout + out.stderr)
    return match.group(0) if match else None


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


def download(url: str, dest: Path) -> None:
    with urllib.request.urlopen(url) as response, dest.open("wb") as f:
        shutil.copyfileobj(response, f)


def checksum_for(checksums_path: Path, asset: str) -> str:
    for line in checksums_path.read_text(encoding="utf-8").splitlines():
        digest, _, name = line.partition("  ")
        if name == asset:
            return digest
    fail(f"no checksum entry for {asset}")


def main() -> None:
    root = repo_root()
    bin_dir = Path.home() / ".cargo" / "bin"

    if installed_version() == VERSION:
        print(f"lefthook {VERSION} already installed", flush=True)
        subprocess.run(["lefthook", "install"], cwd=root, check=True)
        return

    plat = target_platform()
    arch = target_arch()
    asset = f"lefthook_{VERSION}_{plat}_{arch}"
    if plat == "Windows":
        asset += ".exe"

    base_url = f"https://github.com/evilmartians/lefthook/releases/download/v{VERSION}"

    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        asset_path = tmp_dir / asset
        checksums_path = tmp_dir / "checksums.txt"

        download(f"{base_url}/{asset}", asset_path)
        download(f"{base_url}/lefthook_checksums.txt", checksums_path)

        expected = checksum_for(checksums_path, asset)
        hasher = hashlib.sha256()
        with asset_path.open("rb") as f:
            for chunk in iter(lambda: f.read(1024 * 1024), b""):
                hasher.update(chunk)
        if hasher.hexdigest() != expected:
            fail(f"checksum mismatch for {asset}")

        bin_dir.mkdir(parents=True, exist_ok=True)
        install_name = "lefthook.exe" if plat == "Windows" else "lefthook"
        install_path = bin_dir / install_name
        tmp_install = install_path.with_suffix(install_path.suffix + ".tmp")
        shutil.copy(asset_path, tmp_install)
        tmp_install.chmod(tmp_install.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        tmp_install.replace(install_path)

    path_dirs = os.environ.get("PATH", "").split(os.pathsep)
    if str(bin_dir) not in path_dirs:
        print(f"warning: {bin_dir} is not on PATH", file=sys.stderr)

    print(f"installed lefthook {VERSION} to {bin_dir}", flush=True)
    subprocess.run([str(install_path), "install"], cwd=root, check=True)


if __name__ == "__main__":
    main()
