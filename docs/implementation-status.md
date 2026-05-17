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
  messages; the standalone CLI keeps registry configuration out of
  scope for `SchemaRef`
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
rules, while the full unbounded numeric semantics described by the draft
remain outside the current value model.

The Rust API exposes one official helper profile:

- `tpack::recommended_schema_id_xxh64_v1` for the compact `xxh64-v1`
  profile, defined as `xxHash64(seed=0)` over the canonical schema
  descriptor bytes with a fixed 8-byte big-endian output

This helper leaves cache namespace and registry binding authentication
to the embedding application.

The current decoder already fails closed on `FullSchemaWithId` cache-hit
conflicts: if a registry entry exists for a `SchemaId` and the embedded
schema decodes differently, decode fails with
`EmbeddedSchemaMismatch` instead of replacing the binding.

For deployments that use `xxh64-v1`, another agreed profile, or a
locally assigned `SchemaId`, the core codec still only sees opaque
bytes. Scope, reset behavior, and `SchemaRef` admissibility remain
deployment policy outside the codec. Those deployments must stay
fail-closed on ambiguity, stale bindings, lost binding scope, or
observed collisions.

## Deliberately Not Changed In This Sync

- `TPACK`/`TPAK` magic stays unchanged in code or vectors; the current
  draft and implementation both use ASCII `TPAK`
- no rewrite of `BigInt`, `BigUInt`, or decimal backing types
- no changes to map sentinels, union tagging, field flags, or the core
  type model
- no large `serde` or native API redesign
