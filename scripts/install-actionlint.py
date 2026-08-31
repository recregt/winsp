#!/usr/bin/env python3
import hashlib
import stat
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

VERSION = "1.7.12"


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def download(url: str, dest: Path) -> None:
    with urllib.request.urlopen(url) as response, dest.open("wb") as f:
        while chunk := response.read(1024 * 1024):
            f.write(chunk)


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


def main() -> None:
    asset = f"actionlint_{VERSION}_linux_amd64.tar.gz"
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

        subprocess.run(["tar", "xzf", str(asset_path), "actionlint"], check=True)

    binary = Path("actionlint")
    binary.chmod(binary.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    print(f"installed actionlint {VERSION}")


if __name__ == "__main__":
    main()
