#!/usr/bin/env python3
import platform
import subprocess
import sys


def main() -> None:
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
        sys.exit(clippy.returncode)

    test = subprocess.run(["cargo", "test", *package_args, "--locked"], check=False)
    sys.exit(test.returncode)


if __name__ == "__main__":
    main()
