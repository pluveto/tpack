# TPACK (Typed Pack)

[![Crates.io](https://img.shields.io/crates/v/tpack.svg)](https://crates.io/crates/tpack)
[![Documentation](https://docs.rs/tpack/badge.svg)](https://docs.rs/tpack)
[![CI Status](https://github.com/pluveto/tpack/actions/workflows/ci.yml/badge.svg)](https://github.com/pluveto/tpack/actions/workflows/ci.yml)

TPACK is a strictly typed, self-describing binary serialization format. This repository is the Rust reference implementation for v1.

The design target is simple: keep decoding deterministic, schema-aware, and low-overhead without requiring out-of-band `.proto` files or a separate schema negotiation protocol.

The intended positioning is narrower than "generic binary format for everything". TPACK defaults to one self-contained typed value per message, with the complete schema carried on the wire unless a cached-schema profile is explicitly in use. That makes it closer to schema-carrying interchange than to Avro single-object encoding, Arrow IPC batches, or CBOR values validated against a separate CDDL schema.

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

`FullSchemaWithId` reuses the cached schema AST when the schema ID is already in the registry. The decoder validates the embedded schema bytes against the cached schema by default; only deployments that already trust the schema-id namespace and registry binding should disable that check through `DecodeOptions::validate_embedded_schema_on_cache_hit`. If the embedded schema and cached binding differ, that is a schema-id collision or configuration error for that registry scope and the message must be rejected. The recommended SHA-256 helper below standardizes identifiers, but it does not authenticate a cache entry by itself.

`SchemaRef` requires an active registry entry. It must also be rejected when the binding is ambiguous, expired, or out of scope for the active cached-schema profile.

The format name is `TPACK`, while the v1 wire magic is the fixed 4-byte marker `TPAK`. This repository keeps that marker intentionally for the current draft line so the draft text, examples, tests, and implementation stay wire-compatible. The project does not currently rename the magic just to align spelling.

`SchemaId` remains opaque in the core format. For uncoordinated deployments that do not already have a registry convention, the recommended profile is to hash the exact bytes returned by `encode_schema(&schema)` and use that bare digest as the `SchemaId`. That hash input excludes the header, envelope fields, `SchemaLen`, and data bytes. This is a recommendation for interoperability, not a core wire requirement.

For constrained deployments that do not want SHA-256 on device, the format still permits registry-issued IDs, connection-local IDs, boot-session-local IDs, or other profile-defined names. A simpler or faster hash is only a local naming convention in those profiles, not proof of schema identity across deployments, and such profiles need an explicit scope and reset rule. Once that scope is lost or ambiguous, `SchemaRef` must fail instead of guessing.

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

That means the current implementation is a conforming executable reference only for messages whose values fit inside those ranges. This is an implementation boundary, not a wire-format redesign signal.

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
