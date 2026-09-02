#!/usr/bin/env python3
import platform
import subprocess
import sys

STASH_MESSAGE = "pre-push-hook-autostash"


def has_uncommitted_changes() -> bool:
    out = subprocess.run(
        ["git", "status", "--porcelain"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    )
    return bool(out.stdout.strip())


def stash_worktree() -> None:
    print(
        "Uncommitted changes detected: stashing them so checks run against "
        "exactly what's being pushed (restored automatically afterward).",
        file=sys.stderr,
        flush=True,
    )
    subprocess.run(
        ["git", "stash", "push", "--include-untracked", "-m", STASH_MESSAGE],
        check=True,
    )


def restore_worktree() -> None:
    pop = subprocess.run(["git", "stash", "pop"], check=False)
    if pop.returncode != 0:
        print(
            "warning: could not restore stashed changes automatically; "
            "run `git stash pop` manually to recover them "
            f"(look for a stash entry named '{STASH_MESSAGE}').",
            file=sys.stderr,
        )


def run_checks(package_args: list) -> int:
    fmt = subprocess.run(["cargo", "fmt", "--all", "--", "--check"], check=False)
    if fmt.returncode != 0:
        print(
            "\ncargo fmt check failed. If this is just formatting drift, run: "
            "cargo fmt --all\n"
            "If you see a parser/syntax error above instead of a diff, fix that "
            "first.",
            file=sys.stderr,
        )
        return fmt.returncode

    clippy = subprocess.run(
        [
            "cargo",
            "clippy",
            *package_args,
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        check=False,
    )
    if clippy.returncode != 0:
        return clippy.returncode

    test = subprocess.run(["cargo", "test", *package_args, "--locked"], check=False)
    return test.returncode


def main() -> int:
    on_windows = platform.system() == "Windows"
    package_args = ["--workspace"] if on_windows else ["-p", "winsp-core"]

    if not on_windows:
        print(
            "Non-Windows host: scoping clippy/test to winsp-core "
            "(crates/app and crates/windows are windows-only; "
            "run `cargo win-clippy` / `cargo win-test` to check them locally).",
            file=sys.stderr,
            flush=True,
        )

    stashed = has_uncommitted_changes()
    if stashed:
        try:
            stash_worktree()
        except subprocess.CalledProcessError:
            print(
                "error: failed to stash uncommitted changes; aborting before "
                "running checks against a mixed working tree.",
                file=sys.stderr,
            )
            return 1

    try:
        return run_checks(package_args)
    finally:
        if stashed:
            restore_worktree()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
