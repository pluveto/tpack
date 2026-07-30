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
- arbitrary-precision numeric value model for `Decimal`,
  `Decimal(P,S)`, `BigInt`, and `BigUInt` (backed by `num-bigint`),
  with explicit decoder/encoder limits

## Arbitrary-Precision Numerics

The draft data model treats `Decimal`, `BigInt`, and `BigUInt` as
arbitrary-precision or arbitrary-size types and allows implementations
to impose limits. The Rust value model now matches that boundary:

- `Decimal { scale: i64, coefficient: BigInt }`
- `DecimalFixed(BigInt)` (scale lives in the schema as `Decimal(P,S)`)
- `BigInt(BigInt)`
- `BigUInt(BigUint)`

Scale remains `i64` in the public API for practical range; a scale
encoded as an SVarInt that overflows `i64` is rejected as
`VarintOverflow`. Coefficient and integer magnitudes use bigint
varint helpers only on those four value paths; lengths, field ids, and
counts still use the `u64` varint path.

Default resource limits (overridable via `Limits`):

- `max_bigint_bytes`: 1024 — maximum wire length of a single bigint
  UVarInt/SVarInt payload
- `max_decimal_digits`: 10_000 — maximum base-10 digit count for
  `Decimal` / `Decimal(P,S)` coefficients (`Decimal(P,S)` is also
  capped by schema precision `P`)

Values that fit in the historical `i64`/`u64` ranges still produce the
same wire bytes as before, so published draft flat-record test vectors
are unchanged.

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

`StdSchemaRegistry::insert` / `insert_shared` are also fail-closed and
now return `Result<(), SchemaBindingConflict>` (a breaking change from
earlier 0.1 always-succeed insert). Conflicting rebinds are rejected;
use `replace` / `replace_shared` to override.

For deployments that use `xxh64-v1`, another agreed profile, or a
locally assigned `SchemaId`, the core codec still only sees opaque
bytes. Scope, reset behavior, and `SchemaRef` admissibility remain
deployment policy outside the codec. Those deployments must stay
fail-closed on ambiguity, stale bindings, lost binding scope, or
observed collisions.

## Deliberately Not Changed In This Sync

- `TPACK`/`TPAK` magic stays unchanged in code or vectors; the current
  draft and implementation both use ASCII `TPAK`
- no changes to map sentinels, union tagging, field flags, or the core
  type model beyond numeric magnitude backing types
- no full decimal arithmetic API surface
