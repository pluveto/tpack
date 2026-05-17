# tpack

The `std` facade for TPACK.

This crate re-exports the core API and derive macros, and it hosts convenience features that depend on the standard library.

## Features

- `derive` default feature for native derive support
- `serde_support` for schema-aware serde integration
- `std` for registry and convenience APIs built on top of the core crate

## Native Path

For low-latency use cases, prefer the native traits and a schema registry that can resolve `SchemaRef` payloads without extra work.

`StdSchemaRegistry` follows a fail-closed rule at insert time: `insert` / `insert_shared` reject rebinding the same `SchemaId` to different schema content and preserve the existing binding. Callers that need to override a binding can opt into `replace` / `replace_shared`.

`recommended_schema_id_xxh64_v1(&schema)` returns the official helper for
the repository's `xxh64-v1` profile: a fixed 8-byte big-endian
`SchemaId` derived from `encode_schema(&schema)`.

Current conformance boundary:

- `Decimal` and `Decimal(P,S)` are still `i64`-backed in the exposed
  value model
- `BigInt` is still `i64`-backed
- `BigUInt` is still `u64`-backed

## Serde Path

The serde bridge is available when the `serde_support` feature is enabled.

`from_slice` and `from_value` keep the default path small. When serde decoding needs a registry, custom limits, or custom `DecodeOptions`, use `serde_support::Deserializer::new()` and configure it with builder-style methods before calling `slice` or `value`.

## Reference Assets

- root `test-vectors/` contains the public example vectors
- `crates/tpack/tests/reference.rs` validates the draft example bytes
- `crates/tpack/tests/cache_validation.rs` covers default cache-hit
  validation, collision handling, and the explicit opt-out path
- root `docs/implementation-status.md` tracks the current executable
  reference boundary
