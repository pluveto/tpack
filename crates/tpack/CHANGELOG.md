# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Re-export `PreparedSchema` / `encode_prepared_message` from `tpack-core`
  for steady-state encode (pair with a reused `Encoder`)

### Changed

- **Breaking (via tpack-core):** `TpackValue` numeric variants and
  `Decimal` now use `num-bigint` magnitudes; update call sites to
  `BigInt::from` / `BigUint::from`
- serde_support: oversized big integers visit as decimal strings instead
  of truncating
- `SchemaRef` encode no longer serializes the schema descriptor on every
  message (via `tpack-core`)

## [0.1.1](https://github.com/pluveto/tpack/compare/tpack-v0.1.0...tpack-v0.1.1) - 2026-07-28

### Other

- Revise draft positioning and publish test vectors ([#4](https://github.com/pluveto/tpack/pull/4))

## [0.1.0](https://github.com/pluveto/tpack/releases/tag/tpack-v0.1.0) - 2026-05-16

### Added

- *(repo)* add implementation and draft assets ([#1](https://github.com/pluveto/tpack/pull/1))

### Fixed

- *(release)* prepare crates for release-plz ([#2](https://github.com/pluveto/tpack/pull/2))
# Changelog

All notable changes to this crate will be documented in this file.
