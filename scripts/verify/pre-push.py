#!/usr/bin/env python3
import platform
import shutil
import subprocess
import sys

STASH_MESSAGE = "pre-push-hook-autostash"
WINE_TARGET = "x86_64-pc-windows-gnu"
WINE_MINGW_LINKER = "x86_64-w64-mingw32-gcc"
WINE_TEST_SKIPS = [
    "encapsulation",
    "build_toast_escapes_reserved_characters_into_valid_xml",
    "build_toast_produces_xml_the_real_parser_accepts",
    "build_package_sets_text_content_successfully",
]


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


def wine_toolchain_available() -> bool:
    if not shutil.which("wine") or not shutil.which(WINE_MINGW_LINKER):
        return False
    try:
        targets = subprocess.run(
            ["rustup", "target", "list", "--installed"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
        )
    except FileNotFoundError:
        return False
    return WINE_TARGET in targets.stdout.split()


def select_profile() -> tuple:
    if platform.system() == "Windows":
        return ["--workspace"], [], [], None

    if wine_toolchain_available():
        test_extra = []
        for name in WINE_TEST_SKIPS:
            test_extra += ["--skip", name]
        notice = (
            "Wine toolchain detected: running the full workspace suite "
            f"cross-compiled for {WINE_TARGET} (this is slower than a native "
            "run; crates/app and crates/windows are exercised for real)."
        )
        return ["--workspace"], ["--target", WINE_TARGET], test_extra, notice

    notice = (
        "Non-Windows host without a Wine toolchain: scoping clippy/test to "
        "winsp-core (crates/app and crates/windows are windows-only; install "
        f"the {WINE_TARGET} rustup target plus wine and {WINE_MINGW_LINKER} to "
        "check them locally)."
    )
    return ["-p", "winsp-core"], [], [], notice


def run_checks(package_args: list, target_args: list, test_extra_args: list) -> int:
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
            *target_args,
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        check=False,
    )
    if clippy.returncode != 0:
        return clippy.returncode

    test_cmd = ["cargo", "test", *package_args, *target_args, "--locked"]
    if test_extra_args:
        test_cmd += ["--", *test_extra_args]
    test = subprocess.run(test_cmd, check=False)
    return test.returncode


def main() -> int:
    package_args, target_args, test_extra_args, notice = select_profile()
    if notice:
        print(notice, file=sys.stderr, flush=True)

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
        return run_checks(package_args, target_args, test_extra_args)
    finally:
        if stashed:
            restore_worktree()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
