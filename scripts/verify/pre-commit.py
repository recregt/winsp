#!/usr/bin/env python3
import subprocess
import sys


def main() -> None:
    result = subprocess.run(["cargo", "fmt", "--all", "--", "--check"], check=False)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
