import hashlib
import os
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

TIMEOUT_SECONDS = 30


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def download(url: str, dest: Path) -> None:
    try:
        with (
            urllib.request.urlopen(url, timeout=TIMEOUT_SECONDS) as response,
            dest.open("wb") as f,
        ):
            shutil.copyfileobj(response, f)
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


def atomic_install(src: Path, bin_dir: Path, name: str) -> Path:
    install_path = bin_dir / name
    fd, tmp_name = tempfile.mkstemp(dir=bin_dir, prefix=f"{name}.", suffix=".tmp")
    tmp_install = Path(tmp_name)
    try:
        os.close(fd)
        shutil.copy(src, tmp_install)
        tmp_install.chmod(tmp_install.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        tmp_install.replace(install_path)
    except BaseException:
        tmp_install.unlink(missing_ok=True)
        raise
    return install_path
