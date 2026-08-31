#!/usr/bin/env python3
import re
import sys
from pathlib import Path


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def main() -> None:
    cargo_toml = Path("Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"', cargo_toml)
    if not match:
        fail("could not find [workspace.package] version in Cargo.toml")
    cargo_version = match.group(1)

    changelog = Path("CHANGELOG.md").read_text(encoding="utf-8")
    heading = re.search(r"(?m)^## \[([^\]]+)\]", changelog)
    if not heading:
        fail("could not find a version heading in CHANGELOG.md")
    changelog_version = heading.group(1)

    if changelog_version != cargo_version:
        fail(
            f"CHANGELOG.md's newest entry is [{changelog_version}], but Cargo.toml is "
            f"at {cargo_version}. Regenerate CHANGELOG.md before tagging."
        )

    print(f"CHANGELOG.md is up to date for {cargo_version}")


if __name__ == "__main__":
    main()
