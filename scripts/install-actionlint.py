#!/usr/bin/env python3
import hashlib
import os
import platform
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

VERSION = "1.7.12"
TIMEOUT_SECONDS = 30

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


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


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


def download(url: str, dest: Path) -> None:
    try:
        with (
            urllib.request.urlopen(url, timeout=TIMEOUT_SECONDS) as response,
            dest.open("wb") as f,
        ):
            while chunk := response.read(1024 * 1024):
                f.write(chunk)
    except urllib.error.URLError as e:
        fail(f"failed to download {url}: {e}")


def checksum_for(checksums_path: Path, asset: str) -> str:
    for line in checksums_path.read_text(encoding="utf-8").splitlines():
        digest, _, name = line.partition("  ")
        if name == asset:
            return digest
    fail(f"no checksum entry for {asset}")


def sha256_of(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as f:
        while chunk := f.read(1024 * 1024):
            hasher.update(chunk)
    return hasher.hexdigest()


def extract_binary(archive_path: Path, member: str, dest_dir: Path) -> Path:
    if archive_path.suffix == ".zip":
        with zipfile.ZipFile(archive_path) as zf:
            zf.extract(member, dest_dir)
    else:
        with tarfile.open(archive_path) as tf:
            tf.extract(member, dest_dir, filter="data")
    return dest_dir / member


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
        actual = sha256_of(asset_path)
        if actual != expected:
            fail(f"checksum mismatch for {asset}")

        extracted = extract_binary(asset_path, member, tmp_dir)

        bin_dir.mkdir(parents=True, exist_ok=True)
        install_path = bin_dir / member
        fd, tmp_name = tempfile.mkstemp(dir=bin_dir, prefix=f"{member}.", suffix=".tmp")
        tmp_install = Path(tmp_name)
        try:
            os.close(fd)
            shutil.copy(extracted, tmp_install)
            tmp_install.chmod(tmp_install.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
            tmp_install.replace(install_path)
        except BaseException:
            tmp_install.unlink(missing_ok=True)
            raise

    print(f"installed actionlint {VERSION} to {bin_dir}")


if __name__ == "__main__":
    main()
