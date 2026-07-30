# tpack

The `std` facade for TPACK.

This crate re-exports the core API and derive macros, and it hosts convenience features that depend on the standard library.

## Features

- `derive` default feature for native derive support
- `serde_support` for schema-aware serde integration
- `std` for registry and convenience APIs built on top of the core crate

## Native Path

For low-latency use cases, prefer the native traits and a schema registry that can resolve `SchemaRef` payloads without extra work.

**Breaking API change (relative to earlier 0.1 behavior):**
`StdSchemaRegistry::insert` / `insert_shared` now return
`Result<(), SchemaBindingConflict>` instead of always succeeding.
They follow a fail-closed rule at insert time: rebinding the same
`SchemaId` to different schema content is rejected and the existing
binding is preserved. Callers that need to override a binding should
use `replace` / `replace_shared`.

`recommended_schema_id_xxh64_v1(&schema)` returns the official helper for
the repository's `xxh64-v1` profile: a fixed 8-byte big-endian
`SchemaId` derived from `encode_schema(&schema)`.

`tpack-core` intentionally stops at `encode_schema(&schema)` and does not
carry any hash dependency. The official `xxh64-v1` helper lives in this
`tpack` integration crate (not in `tpack-core`).

Numeric value model:

- `Decimal`, `Decimal(P,S)`, `BigInt`, and `BigUInt` use `num-bigint`
  magnitudes (see `tpack-core` README / `docs/implementation-status.md`)
- This is a **source-breaking** change relative to earlier 0.1.x
  `i64`/`u64`-backed variants; construct values with `BigInt::from(...)`
  / `BigUint::from(...)`

## Serde Path

The serde bridge is available when the `serde_support` feature is enabled.

`from_slice` and `from_value` keep the default path small. When serde decoding needs a registry, custom limits, or custom `DecodeOptions`, use `serde_support::Deserializer::new()` and configure it with builder-style methods before calling `slice` or `value`.

Oversized `BigInt` / `BigUInt` / `DecimalFixed` values that do not fit
`i64`/`u64` are exposed to serde visitors as **decimal strings** rather
than silently truncated. Values that fit the host integer range still
visit as `i64`/`u64`.

## Reference Assets

- root `test-vectors/` contains the public example vectors
- `crates/tpack/tests/reference.rs` validates the draft example bytes
- `crates/tpack/tests/cache_validation.rs` covers default cache-hit
  validation, collision handling, and the explicit opt-out path
- root `docs/implementation-status.md` tracks the current executable
  reference boundary
