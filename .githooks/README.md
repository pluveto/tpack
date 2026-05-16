# Git Hooks

This repository ships versioned Git hooks under `.githooks/`.

## Enable

Configure Git to use the repository-managed hooks directory:

```bash
git config core.hooksPath .githooks
```

## Commit Message Policy

The `commit-msg` hook enforces Conventional Commits.

Accepted forms:

```text
type(scope): subject
type: subject
type(scope)!: subject
```

Supported types:

- `feat`
- `fix`
- `docs`
- `style`
- `refactor`
- `perf`
- `test`
- `build`
- `ci`
- `chore`

## Pre-Commit Checks

The `pre-commit` hook runs:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-features --all-targets -- -D warnings`
