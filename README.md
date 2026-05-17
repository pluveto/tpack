# TPACK (Typed Pack)

[![Crates.io](https://img.shields.io/crates/v/tpack.svg)](https://crates.io/crates/tpack)
[![Documentation](https://docs.rs/tpack/badge.svg)](https://docs.rs/tpack)
[![CI Status](https://github.com/pluveto/tpack/actions/workflows/ci.yml/badge.svg)](https://github.com/pluveto/tpack/actions/workflows/ci.yml)

TPACK is a strictly typed, self-describing binary serialization format. This repository is the Rust reference implementation for v1.

## Workspace Layout

The workspace is split by responsibility:

- `tpack-core`: `#![no_std] + alloc` core wire codec, schema AST, validation, and native traits
- `tpack-macros`: procedural macros for native derive support
- `tpack`: `std` facade, registry integration, and optional `serde` support
- `tpack-cli`: command-line tooling for inspection, verification, and canonicalization

## Wire Protocol v1

TPACK messages are built from a fixed header and an envelope.

```text
[Header: Magic 4B + Version 1B] [Envelope: EnvelopeMode 1B + Payload...]
```

Envelope modes:

- `0x00` `FullSchema`: `SchemaLen + Schema + Data`
- `0x01` `FullSchemaWithId`: `SchemaIdLen + SchemaId + SchemaLen + Schema + Data`
- `0x02` `SchemaRef`: `SchemaIdLen + SchemaId + Data`

`FullSchemaWithId` reuses the cached schema AST when the schema ID is already in the registry. The decoder validates the embedded schema bytes against the cached schema by default.

`SchemaRef` requires an active registry entry.

`SchemaId` remains opaque in the core format.

The only official recommended `SchemaId` profile is `xxh64-v1`: `xxHash64(seed=0)` over the canonical schema descriptor bytes, serialized as a fixed 8-byte big-endian value. This recommendation is for bounded or registry-backed deployments and does not change the core opaque-bytes semantics.

If a deployment uses `xxh64-v1`, it must keep the binding scope explicit and fail closed on ambiguity, collision, stale cache state, or loss of binding context after reset or reconnect. Deployments may use another profile by prior agreement, but that is outside the official recommendation.

## What The Core Guarantees

- Single-pass parsing after the active schema is available
- Borrowed strings and byte slices on the native data path
- Canonical encoding checks, including shortest varints and map ordering
- Explicit failure on malformed or non-canonical inputs
- Shared schema size limits on both decode and encode paths
- No dependency on host-language object layout

## Current Conformance Boundary

This repository is the Rust reference implementation for the envelope layout, schema encoding, validation rules, canonicalization behavior, and the example vectors in the Internet-Draft.

The draft data model defines `Decimal`, `BigInt`, and `BigUInt` as arbitrary-precision types. The current Rust value model does not fully implement that boundary yet:

- `Decimal` currently uses `i64` scale and `i64` coefficient
- `Decimal(P,S)` currently uses an `i64` coefficient
- `BigInt` currently maps to `i64`
- `BigUInt` currently maps to `u64`

That means the current implementation is a conforming executable reference only for messages whose values fit inside those ranges.

## Verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
```

Public interoperability vectors live under `test-vectors/` and are
consumed directly by `crates/tpack/tests/reference.rs`. Cached-schema
safety behavior is covered by `crates/tpack/tests/cache_validation.rs`.
The current reference-implementation boundary is summarized in
`docs/implementation-status.md`.

Additional repository checks are defined in `deny.toml`, `typos.toml`, and the GitHub Actions workflows.

## Internet-Draft

The Internet-Draft is maintained in `drafts/draft-zhang-tpack-format-00.md`
using `kramdown-rfc`.  Regenerate the rendered artifacts with:

```bash
make -C drafts
```

`make -C drafts` writes `drafts/draft-zhang-tpack-format-00.xml`,
`drafts/draft-zhang-tpack-format-00.txt`, and
`drafts/draft-zhang-tpack-format-00.html`.  Run `idnits` against the
generated `.txt` before submission.

## Release Flow

Releases are automated with `release-plz` and GitHub Actions. Release notes are accumulated in `CHANGELOG.md`, and the process is documented in `RELEASING.md`.
