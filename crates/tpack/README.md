# tpack

The `std` facade for TPACK.

This crate re-exports the core API and derive macros, and it hosts convenience features that depend on the standard library.

## Features

- `derive` default feature for native derive support
- `serde_support` for schema-aware serde integration
- `std` for registry and convenience APIs built on top of the core crate

## Native Path

For low-latency use cases, prefer the native traits and a schema registry that can resolve `SchemaRef` payloads without extra work.

When decoding `FullSchemaWithId` with a registry hit, the default path now reparses the embedded schema bytes and requires them to match the cached schema before reusing the cached AST. If a deployment intentionally trusts the registry entry and wants the older skip-only behavior, set `DecodeOptions::validate_embedded_schema_on_cache_hit` to `false`.

For deployments that want the draft's recommended default `SchemaId`
convention, use `recommended_schema_id_sha256(&schema)` and pass the
resulting 32-byte digest as opaque `SchemaId` bytes. This helper is only
for convenience; the wire format still treats `SchemaId` as opaque.

Current conformance boundary:

- `Decimal` and `Decimal(P,S)` are still `i64`-backed in the exposed
  value model
- `BigInt` is still `i64`-backed
- `BigUInt` is still `u64`-backed

## Serde Path

The serde bridge is available when the `serde_support` feature is enabled. It is intended for compatibility and convenience, not the fastest decode path.

`from_slice` and `from_value` keep the default path small. When serde decoding needs a registry, custom limits, or custom `DecodeOptions`, use `serde_support::Deserializer::new()` and configure it with builder-style methods before calling `slice` or `value`.

## Reference Assets

- root `test-vectors/` contains the public example vectors
- `crates/tpack/tests/reference.rs` validates the draft example bytes
- root `docs/implementation-status.md` tracks the current executable
  reference boundary
