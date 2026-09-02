#!/usr/bin/env python3
import subprocess
import sys


def main() -> None:
    try:
        result = subprocess.run(["cargo", "fmt", "--all", "--", "--check"], check=False)
    except FileNotFoundError as e:
        print(f"error: 'cargo' not found on PATH ({e}).", file=sys.stderr)
        sys.exit(1)
    sys.exit(result.returncode)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
