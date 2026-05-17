# TPACK Rust Reference Implementation Status

This document records the current implementation boundary of the Rust
reference implementation relative to
`drafts/draft-zhang-tpack-format-00.md`.

## Implemented And Regression-Tested

- `FullSchema`, `FullSchemaWithId`, and `SchemaRef` envelope modes
- schema validation and shared schema length limits on encode/decode
- strict canonical checks for shortest varints, map key ordering,
  trailing bytes, and canonical NaN encodings
- cached-schema decode paths, including embedded-schema validation on
  `FullSchemaWithId` registry hits by default
- CLI inspection and canonicalization helpers for self-contained
  messages; the standalone CLI does not expose registry configuration
  for `SchemaRef`
- public byte-level vectors under `test-vectors/`, consumed by
  `crates/tpack/tests/reference.rs`, including the draft flat-record
  examples from the Examples section
- dedicated cache-hit regression tests in
  `crates/tpack/tests/cache_validation.rs`

## Known Boundary: Arbitrary Precision Is Not Fully Landed

The draft specifies `Decimal`, `BigInt`, and `BigUInt` as
arbitrary-precision or arbitrary-size data model types. The current
Rust value model is still bounded:

- `Decimal { coefficient: i64, scale: i64 }`
- `DecimalFixed(i64)`
- `BigInt(i64)`
- `BigUInt(u64)`

This means the reference implementation is useful for validating
envelope layout, schema descriptors, data ordering, and canonical byte
rules, but it does not yet provide the full unbounded numeric semantics
described by the draft.

The SHA-256 `recommended_schema_id_sha256` helper in the Rust API is
only a naming convention for uncoordinated deployments. It hashes the
canonical schema descriptor bytes, but it does not authenticate a cache
namespace or registry binding by itself.

## Deliberately Not Changed In This Sync

- no `TPACK`/`TPAK` magic rename in code or vectors; the current draft
  and implementation still both use ASCII `TPAK`
- no rewrite of `BigInt`, `BigUInt`, or decimal backing types
- no changes to map sentinels, union tagging, field flags, or the core
  type model
- no large `serde` or native API redesign
