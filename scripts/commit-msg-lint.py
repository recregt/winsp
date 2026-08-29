#!/usr/bin/env python3
import re
import subprocess
import sys
from pathlib import Path

TYPES = ["feat", "fix", "refactor", "ci", "perf", "test", "chore", "docs"]

IMPERATIVE_BASE_FORMS = {
    "added": "add",
    "adds": "add",
    "fixed": "fix",
    "fixes": "fix",
    "updated": "update",
    "updates": "update",
    "removed": "remove",
    "removes": "remove",
    "changed": "change",
    "changes": "change",
    "created": "create",
    "creates": "create",
    "resolved": "resolve",
    "resolves": "resolve",
    "renamed": "rename",
    "renames": "rename",
    "moved": "move",
    "moves": "move",
    "deleted": "delete",
    "deletes": "delete",
    "improved": "improve",
    "improves": "improve",
    "enhanced": "enhance",
    "enhances": "enhance",
    "implemented": "implement",
    "implements": "implement",
    "refactored": "refactor",
    "refactors": "refactor",
    "supported": "support",
    "supports": "support",
}

EXEMPT_PREFIXES = ("Merge ", "Revert ", "fixup! ", "squash! ")


def fail(header: str, reason: str) -> None:
    print("Invalid commit message:", file=sys.stderr)
    print(f"  {header}", file=sys.stderr)
    print(file=sys.stderr)
    print(reason, file=sys.stderr)
    sys.exit(1)


def repo_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        check=True,
    )
    return Path(out.stdout.strip())


def valid_scopes(root: Path) -> list:
    scopes = []
    for entry in root.iterdir():
        if entry.name in (".git", "target"):
            continue
        scopes.append(entry.name)
    crates_dir = root / "crates"
    if crates_dir.is_dir():
        for entry in crates_dir.iterdir():
            if entry.is_dir():
                scopes.append(entry.name)
    return sorted(set(scopes))


def touched_dirs(changed_files):
    touched = set()
    all_crate_scoped = True
    for path in changed_files:
        first, _, rest = path.partition("/")
        if first == "crates" and "/" in rest:
            touched.add(rest.split("/", 1)[0])
        else:
            touched.add(first)
            all_crate_scoped = False
    return sorted(touched), all_crate_scoped


def changed_files_for(sha):
    if sha:
        cmd = ["git", "diff-tree", "--no-commit-id", "--name-only", "-r", sha]
    else:
        cmd = ["git", "diff", "--cached", "--name-only"]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True)
    return [line for line in out.stdout.splitlines() if line]


def main() -> None:
    msg_file = sys.argv[1]
    sha = sys.argv[2] if len(sys.argv) > 2 else None

    lines = Path(msg_file).read_text().splitlines()
    header = lines[0] if lines else ""

    if header.startswith(EXEMPT_PREFIXES):
        return

    if len(header) > 50:
        fail(
            header,
            f"Subject line is {len(header)} characters, must be 50 or fewer.",
        )

    for line in lines[2:]:
        if len(line) > 72:
            print("Invalid commit message body:", file=sys.stderr)
            print(f"  {line}", file=sys.stderr)
            print(file=sys.stderr)
            print(
                f"Body line is {len(line)} characters, must be 72 or fewer.",
                file=sys.stderr,
            )
            sys.exit(1)

    scopes = valid_scopes(repo_root())
    types_re = "|".join(TYPES)
    scope_re = "|".join(re.escape(s) for s in scopes) if scopes else "(?!)"
    scope_part = rf"(?:\((?P<scope>{scope_re})\))?"
    pattern = rf"^(?P<type>{types_re}){scope_part}: [a-z0-9].*[^.]$"

    if not re.match(pattern, header):
        fail(
            header,
            "Expected: <type>(<scope>): <imperative description>\n"
            "  scope is optional: <type>: <description> is also valid\n"
            "  scope must match a file or folder name exactly, as it appears in the repo\n"
            f"  type:  {' '.join(TYPES)}\n"
            f"  scope: {' '.join(scopes)}\n"
            "  example: fix(indexer): handle null pointer dereference in shell enumeration",
        )

    full_match = re.match(rf"^(?P<type>{types_re}){scope_part}: (?P<desc>.*)$", header)
    commit_type = full_match.group("type")
    written_scope = full_match.group("scope")
    description = full_match.group("desc")

    first_word = description.split(" ", 1)[0]
    base_form = IMPERATIVE_BASE_FORMS.get(first_word)
    if base_form:
        fail(
            header,
            f"Use imperative mood: '{first_word}' should be '{base_form}' "
            f"(write what the commit does, not what it did).",
        )

    changed_files = changed_files_for(sha)
    if not changed_files:
        return

    touched, all_crate_scoped = touched_dirs(changed_files)
    files_note = f"  files touched: {' '.join(changed_files)}"

    if len(touched) == 1:
        if written_scope and written_scope != touched[0]:
            fail(
                header,
                f"This commit only touches '{touched[0]}', but the scope says "
                f"'{written_scope}'.\n{files_note}",
            )
        if touched[0] == ".github" and commit_type != "ci":
            fail(
                header,
                f"This commit only touches '.github', use type 'ci' (not "
                f"'{commit_type}') so CHANGELOG.md skips it.\n{files_note}",
            )
    elif all_crate_scoped:
        if written_scope and written_scope != "crates":
            fail(
                header,
                f"This commit touches multiple crates ({' '.join(touched)}); "
                f"use scope 'crates' or omit the scope.\n{files_note}",
            )
    else:
        if written_scope:
            fail(
                header,
                f"This commit touches multiple unrelated areas "
                f"({' '.join(touched)}); omit the scope.\n{files_note}",
            )


if __name__ == "__main__":
    main()
