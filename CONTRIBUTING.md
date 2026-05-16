# Contributing

Thank you for contributing to TPACK.

## Scope

This repository is currently focused on the Rust reference implementation, repository documentation, CI, and release
automation. Please keep changes aligned with the active workspace layout and the current draft specification.

The workspace targets Rust 1.85 as its minimum supported version. CI also checks the current stable toolchain.

## Expectations

- Keep changes small and reviewable.
- Prefer English for commit messages, issues, pull requests, and documentation.
- Use Conventional Commits for commit messages. The repository ships a versioned `commit-msg` hook in `.githooks/`.
- Update tests and documentation when behavior changes.
- Do not introduce API or wire-format changes without matching specification updates.

## Enable Hooks

```bash
git config core.hooksPath .githooks
```

This enables the repository-managed `commit-msg` and `pre-commit` hooks.

## Local Checks

Run the following before opening a pull request:

```bash
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
```

If you are working on repository policy or dependency changes, also run the audit and typo checks configured in the root files.

## Pull Requests

Include a concise summary of the change, the motivation, and any compatibility impact. If the change affects public
behavior, add or update tests.
