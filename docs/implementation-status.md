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
- CLI inspection and canonicalization helpers
- public byte-level vectors under `test-vectors/`, including the draft
  flat-record examples from the Examples section

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

## Deliberately Not Changed In This Sync

- no `TPACK`/`TPAK` magic rename in code or vectors; the current draft
  and implementation still both use ASCII `TPAK`
- no rewrite of `BigInt`, `BigUInt`, or decimal backing types
- no changes to map sentinels, union tagging, field flags, or the core
  type model
- no large `serde` or native API redesign
