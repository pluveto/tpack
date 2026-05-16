# Releasing

This repository uses `release-plz` for crate release automation.

## Release flow

1. Merge changes to the default branch.
2. `release-plz` computes crate version bumps and updates changelog entries.
3. A release pull request is opened or refreshed.
4. After the release PR is merged, GitHub Actions publishes the crates to crates.io.

## Notes

- Keep crate versions synchronized through the release tooling.
- Changelog entries should remain concise and user-facing.
- The repository root `CHANGELOG.md` is the public changelog entry point.

