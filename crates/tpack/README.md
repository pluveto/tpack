# tpack

The `std` facade for TPACK.

This crate re-exports the core API and derive macros, and it hosts convenience features that depend on the standard library.

## Features

- `derive` default feature for native derive support
- `serde_support` for schema-aware serde integration
- `std` for registry and convenience APIs built on top of the core crate

## Native Path

For low-latency use cases, prefer the native traits and a schema registry that can resolve `SchemaRef` payloads without extra work.

When decoding `FullSchemaWithId` with a registry hit, the default path reparses the embedded schema bytes and requires them to match the cached schema before reusing the cached AST. Set `DecodeOptions::validate_embedded_schema_on_cache_hit` to `false` only when the schema-id namespace and registry binding are already authenticated or otherwise trusted for the deployment. The recommended SHA-256 helper below is only an identifier convention; it does not authenticate a cache entry by itself.

For deployments that want the draft's recommended default `SchemaId`
convention, use `recommended_schema_id_sha256(&schema)` and pass the
resulting bare 32-byte digest as opaque `SchemaId` bytes. The hash input
is the exact descriptor bytes returned by `encode_schema(&schema)`, not
the message header, envelope, `SchemaLen`, or data block. This helper is
only for convenience; the wire format still treats `SchemaId` as opaque,
using the helper is only a local convention, and the digest does not
authenticate a cache entry by itself.

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
- `crates/tpack/tests/cache_validation.rs` covers default cache-hit
  validation and the explicit opt-out path
- root `docs/implementation-status.md` tracks the current executable
  reference boundary
