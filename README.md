# TPACK (Typed Pack)

[![Crates.io](https://img.shields.io/crates/v/tpack.svg)](https://crates.io/crates/tpack)
[![Documentation](https://docs.rs/tpack/badge.svg)](https://docs.rs/tpack)
[![CI Status](https://github.com/pluveto/tpack/actions/workflows/ci.yml/badge.svg)](https://github.com/pluveto/tpack/actions/workflows/ci.yml)

TPACK is a strictly typed, self-describing binary serialization format. This repository is the Rust reference implementation for v1.

The design target is simple: keep decoding deterministic, schema-aware, and low-overhead without requiring out-of-band `.proto` files or a separate schema negotiation protocol.

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

`FullSchemaWithId` reuses the cached schema AST when the schema ID is already in the registry. The decoder now validates the embedded schema bytes against the cached schema by default; callers that intentionally trust the registry entry can disable that check through `DecodeOptions::validate_embedded_schema_on_cache_hit`.

`SchemaRef` requires an active registry entry.

## What The Core Guarantees

- Single-pass parsing after the active schema is available
- Borrowed strings and byte slices on the native data path
- Canonical encoding checks, including shortest varints and map ordering
- Explicit failure on malformed or non-canonical inputs
- Shared schema size limits on both decode and encode paths
- No dependency on host-language object layout

## Verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
```

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
