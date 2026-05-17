# tpack

The `std` facade for TPACK.

This crate re-exports the core API and derive macros, and it hosts convenience features that depend on the standard library.

## Features

- `derive` default feature for native derive support
- `serde_support` for schema-aware serde integration
- `std` for registry and convenience APIs built on top of the core crate

## Native Path

For low-latency use cases, prefer the native traits and a schema registry that can resolve `SchemaRef` payloads without extra work.

When decoding `FullSchemaWithId` with a registry hit, the default path reparses the embedded schema bytes and requires them to match the cached schema before reusing the cached AST. Set `DecodeOptions::validate_embedded_schema_on_cache_hit` to `false` only when the schema-id namespace and registry binding are already authenticated or otherwise trusted for the deployment. A mismatch is a schema-id collision or configuration error for that binding scope and should be treated as fatal. The SHA-256 default and the compact `xxh64-v1` profile are only identifier conventions; neither authenticates a cache entry by itself.

`StdSchemaRegistry` now follows that default fail-closed rule at insert
time: `insert` / `insert_shared` reject rebinding the same `SchemaId` to
different schema content and preserve the existing binding. Callers that
intentionally need to override a binding must opt into `replace` /
`replace_shared` explicitly.

If a deployment binds the same `SchemaId` bytes to a different schema in
its local registry, default `FullSchemaWithId` cache hits fail closed
with an embedded-schema mismatch instead of silently trusting the cached
AST. `SchemaRef` has no embedded schema bytes, so collision handling and
registry freshness stay with the embedding application. Ambiguous,
expired, or out-of-scope bindings must be rejected before `SchemaRef`
decode.

For deployments that want the draft's open-interoperability default
`SchemaId` convention, use `recommended_schema_id_sha256(&schema)` and
pass the resulting bare 32-byte digest as opaque `SchemaId` bytes. The
hash input is the exact descriptor bytes returned by
`encode_schema(&schema)`, not the message header, envelope,
`SchemaLen`, or data block. This helper is only for convenience; the
wire format still treats `SchemaId` as opaque, using the helper is only
an identifier convention, and the digest does not authenticate a cache
entry by itself.

The documentation also defines an official compact profile named
`xxh64-v1`: compute `xxHash64(seed=0)` over the same canonical schema
descriptor bytes and encode the result as a fixed 8-byte big-endian
`SchemaId`. That profile is meant for bounded registries, local caches,
or constrained deployments where 8-byte identifiers matter more than
open cross-deployment collision resistance. This crate re-exports
`recommended_schema_id_xxh64_v1(&schema)` from `tpack-core` for that
profile and provides `recommended_schema_id_sha256(&schema)` at the
facade layer for the stronger digest.

Deployments that do not want SHA-256 can skip the helper entirely and
use `xxh64-v1`, another local convention, or a registry-issued
identifier. `recommended_schema_id_sha256` remains the documented main
default, not an API requirement. Those local or compact identifiers
still need a documented scope, reset rule, and collision policy; once
that context is lost, expired, or ambiguous, `SchemaRef` must fail
instead of guessing.

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
