#!/usr/bin/env python3
import platform
import shutil
import signal
import subprocess
import sys

sys.dont_write_bytecode = True

STASH_MESSAGE = "pre-push-hook-autostash"
WINE_TARGET = "x86_64-pc-windows-gnu"
WINE_MINGW_LINKER = "x86_64-w64-mingw32-gcc"
WINE_TEST_SKIPS = [
    "encapsulation",
    "build_toast_escapes_reserved_characters_into_valid_xml",
    "build_toast_produces_xml_the_real_parser_accepts",
    "build_package_sets_text_content_successfully",
]


class HookError(Exception):
    pass


def run_tool(cmd: list, *, check: bool = False, **kwargs) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(cmd, check=check, **kwargs)
    except FileNotFoundError as e:
        raise HookError(f"'{cmd[0]}' not found on PATH ({e}).") from e


def isolated_kwargs() -> dict:
    # New process group: a second Ctrl+C during stash apply/drop can't reach the child.
    if platform.system() == "Windows":
        return {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP}
    return {"start_new_session": True}


def has_uncommitted_tracked_changes() -> bool:
    try:
        out = run_tool(
            ["git", "status", "--porcelain", "--untracked-files=no"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=True,
        )
    except subprocess.CalledProcessError as e:
        detail = e.stderr.strip() if e.stderr else str(e)
        raise HookError(f"'git status' failed: {detail}") from e
    return bool(out.stdout.strip())


def stash_worktree() -> str:
    print(
        "Uncommitted changes to tracked files detected: stashing them so "
        "checks run against exactly what's being pushed (untracked files are "
        "left alone; restored automatically afterward).",
        file=sys.stderr,
        flush=True,
    )
    previous_handler = signal.signal(signal.SIGINT, signal.SIG_IGN)
    try:
        run_tool(
            ["git", "stash", "push", "-m", STASH_MESSAGE],
            check=True,
            **isolated_kwargs(),
        )
        sha = run_tool(
            ["git", "rev-parse", "--verify", "refs/stash"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=True,
            **isolated_kwargs(),
        ).stdout.strip()
    except subprocess.CalledProcessError as e:
        raise HookError(
            "failed to stash uncommitted changes (see git output above); "
            "aborting before running checks against a mixed working tree."
        ) from e
    finally:
        signal.signal(signal.SIGINT, previous_handler)
    return sha


def find_stash_ref(sha: str) -> str | None:
    out = run_tool(
        ["git", "stash", "list", "--format=%H %gd"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    for line in out.stdout.splitlines():
        commit, _, ref = line.partition(" ")
        if commit == sha:
            return ref
    return None


def restore_worktree(sha: str) -> None:
    previous_handler = signal.signal(signal.SIGINT, signal.SIG_IGN)
    try:
        apply_result = run_tool(
            ["git", "stash", "apply", sha], check=False, **isolated_kwargs()
        )
        if apply_result.returncode != 0:
            print(
                "warning: could not restore stashed changes automatically; run "
                f"`git stash apply {sha}` manually to recover them.",
                file=sys.stderr,
            )
            return

        ref = find_stash_ref(sha)
        if ref is None:
            print(
                "warning: restored stashed changes but could not find the stash "
                f"entry to drop it; run `git stash list` and drop the entry for "
                f"commit {sha} manually.",
                file=sys.stderr,
            )
            return

        drop_result = run_tool(
            ["git", "stash", "drop", ref], check=False, **isolated_kwargs()
        )
        if drop_result.returncode != 0:
            print(
                f"warning: restored stashed changes but could not drop stash "
                f"entry {ref}; run `git stash drop {ref}` manually to clean it up.",
                file=sys.stderr,
            )
    finally:
        signal.signal(signal.SIGINT, previous_handler)


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


def cargo_subcommand_available(name: str) -> bool:
    probe = run_tool(
        ["cargo", name, "--version"], check=False, capture_output=True, text=True
    )
    return probe.returncode == 0


def run_checks(package_args: list, target_args: list, test_extra_args: list) -> int:
    if not cargo_subcommand_available("fmt"):
        print(
            "error: 'cargo fmt' isn't available (rustfmt component missing?). "
            "Install it with: rustup component add rustfmt",
            file=sys.stderr,
        )
        return 1

    fmt = run_tool(["cargo", "fmt", "--all", "--", "--check"], check=False)
    if fmt.returncode != 0:
        print(
            "\ncargo fmt check failed. If this is just formatting drift, run: "
            "cargo fmt --all\n"
            "If you see a parser/syntax error above instead of a diff, fix that "
            "first.",
            file=sys.stderr,
        )
        return fmt.returncode

    if not cargo_subcommand_available("clippy"):
        print(
            "error: 'cargo clippy' isn't available (clippy component missing?). "
            "Install it with: rustup component add clippy",
            file=sys.stderr,
        )
        return 1

    clippy = run_tool(
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
    test = run_tool(test_cmd, check=False)
    return test.returncode


def main() -> int:
    package_args, target_args, test_extra_args, notice = select_profile()
    if notice:
        print(notice, file=sys.stderr, flush=True)

    stash_sha = stash_worktree() if has_uncommitted_tracked_changes() else None

    try:
        return run_checks(package_args, target_args, test_extra_args)
    finally:
        if stash_sha:
            restore_worktree(stash_sha)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
    except HookError as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)
