# tpack-core

The core execution layer for TPACK.

This crate is designed for `#![no_std]` with `alloc`. It owns the wire codec, schema AST, validation logic, and the native traits used by the higher-level crates.

## Execution Model

`Decoder<'de>` operates on a borrowed input buffer and advances a cursor through the message. The data path is intended to stay allocation-free for borrowed values.

When `FullSchemaWithId` hits a schema registry entry, the decoder reuses the cached schema AST. By default it still reparses the embedded schema bytes and requires them to match the cached schema before the cached AST is accepted. Disable that comparison only when the schema-id namespace and registry binding are already authenticated or otherwise trusted for the deployment.

If a deployment ever binds the same `SchemaId` bytes to a different
schema locally, default `FullSchemaWithId` cache hits fail closed with an
embedded-schema mismatch instead of silently trusting the cached AST.
`SchemaRef` cannot do that because it carries no embedded schema bytes,
so conflicting, stale, or out-of-scope bindings must be treated as a
profile or registry error by the caller.

## Recommended `SchemaId` Helper

`recommended_schema_id_xxh64_v1(&Schema)` derives this crate's compact
helper for the repository's official `xxh64-v1` profile:
`xxHash64(seed=0)` over the exact bytes returned by
`encode_schema(&schema)`, encoded as a fixed 8-byte big-endian output.

That hash input excludes the header, envelope fields, `SchemaLen`, and
data bytes.

This is a helper, not a wire requirement. `SchemaId` remains opaque in
the core format, and deployments may still use registry-issued or
application-defined identifiers. Using this helper is still only a local
convention, and it does not authenticate a registry binding or cached
schema reuse decision by itself.

Deployments that use `recommended_schema_id_xxh64_v1` still need an
explicit scope, reset rule, and collision policy. Once the binding
scope is lost, expired, ambiguous, or conflicting, `SchemaRef` must be
rejected.

## Value Model

Borrowed payloads stay borrowed:

- strings are represented as `&'de str`
- bytes are represented as `&'de [u8]`
- structural values are decoded according to the active schema

Current conformance boundary:

- `Decimal { coefficient, scale }` is still `i64`-backed
- `DecimalFixed` is still `i64`-backed
- `BigInt` is still `i64`-backed
- `BigUInt` is still `u64`-backed

The envelope layout, schema encoding, validation rules, and canonical
checks are implemented, but arbitrary-precision numeric semantics are
not fully landed yet.

## Canonical Mode

When canonical checking is enabled, the decoder rejects:

- overlong varints
- unordered map keys
- non-canonical floating-point NaN encodings
- trailing bytes after a valid message

## Limits

`Limits` apply to both schema validation and value processing. In particular, `max_schema_len` is enforced symmetrically on decode and encode paths so an encoder cannot emit a schema that the decoder would reject under the same limits.

## Reference Assets

- root `test-vectors/` exposes the public byte-level vectors
- `crates/tpack/tests/reference.rs` verifies the draft flat-record
  examples and canonical regression cases
- `crates/tpack/tests/cache_validation.rs` verifies default cache-hit
  validation and the explicit opt-out path
- `docs/implementation-status.md` in the repository root summarizes the
  current implementation boundary
