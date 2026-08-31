#!/usr/bin/env python3
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

FIXTURES = [
    (
        "feat: real feature (#42)\n\n* feat: real feature\n\n* fix: real fix\n",
        ["real feature", "real fix"],
        [],
    ),
    (
        "fix: main fix\n\n* feat: sneaky bullet should not appear\n",
        ["main fix"],
        ["sneaky bullet should not appear"],
    ),
    (
        (
            "feat: add config validation\n\n"
            "The implementation fixes an edge case:\n\n"
            "fix: handle empty configuration\n"
        ),
        ["add config validation"],
        ["handle empty configuration"],
    ),
]


def repo_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    )
    return Path(out.stdout.strip())


def run_git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def build_fixture_repo(repo: Path) -> None:
    run_git(repo, "init", "-q")
    run_git(repo, "config", "user.email", "test@example.com")
    run_git(repo, "config", "user.name", "test")
    run_git(repo, "commit", "--allow-empty", "-q", "-m", "chore: init")
    for message, _, _ in FIXTURES:
        msg_file = repo / ".fixture-msg"
        msg_file.write_text(message, encoding="utf-8")
        run_git(repo, "commit", "--allow-empty", "-q", "-F", str(msg_file))
        msg_file.unlink()


def main() -> None:
    if not shutil.which("git-cliff"):
        print("git-cliff not found on PATH", file=sys.stderr)
        sys.exit(1)

    root = repo_root()
    cliff_toml = root / "cliff.toml"

    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        build_fixture_repo(repo)

        out = subprocess.run(
            [
                "git-cliff",
                "--config",
                str(cliff_toml),
                "--repository",
                str(repo),
                "--unreleased",
                "--tag",
                "v0.0.1-test",
                "--strip",
                "header",
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=True,
        )
        changelog = out.stdout

    changelog_lower = changelog.lower()
    failures = []
    for message, expected, forbidden in FIXTURES:
        for text in expected:
            if text.lower() not in changelog_lower:
                failures.append(f"expected {text!r} in changelog for: {message!r}")
        for text in forbidden:
            if text.lower() in changelog_lower:
                failures.append(f"forbidden {text!r} found in changelog for: {message!r}")

    if failures:
        print("cliff.toml regression check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        print("\n--- generated changelog ---", file=sys.stderr)
        print(changelog, file=sys.stderr)
        sys.exit(1)

    print("cliff.toml regression check passed")


if __name__ == "__main__":
    main()
