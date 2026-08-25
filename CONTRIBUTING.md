# Contributing to WinSP

## Local Setup

Install the git hooks (managed via `lefthook`):

```bash
scripts/install-lefthook.sh
```

This enforces `cargo fmt` and commit message linting on commit, and `cargo test`/`clippy` before push.

## Commit Format

Follow standard conventional commits: `type(scope): imperative description`

* **Scope:** Matches any crate under `crates/` (e.g. `indexer`, `app`) or top-level file (e.g. `Cargo.toml`, `.github`). Use `crates` if touching multiple crates; omit the scope entirely if the commit also touches something outside `crates/`.
* **Types:** `feat`, `fix`, `refactor`, `perf`, `test`, `ci`, `chore`, `docs`.

## Commands

```bash
# Tests
cargo test --workspace --locked

# Benchmarks
cargo bench -p winsp-core
```

**Keep PR descriptions brief: 1–2 sentences in `## Summary` and `Fixes #N`.**
