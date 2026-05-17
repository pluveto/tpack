# tpack-core

The core execution layer for TPACK.

This crate is designed for `#![no_std]` with `alloc`. It owns the wire codec, schema AST, validation logic, and the native traits used by the higher-level crates.

## Execution Model

`Decoder<'de>` operates on a borrowed input buffer and advances a cursor through the message. The data path is intended to stay allocation-free for borrowed values.

When `FullSchemaWithId` hits a schema registry entry, the decoder reuses the cached schema AST. By default it still reparses the embedded schema bytes and requires them to match the cached schema before the cached AST is accepted. This keeps cache hits fast without silently trusting mismatched embedded schema payloads.

## Recommended `SchemaId` Helper

`recommended_schema_id_sha256(&Schema)` derives the draft's recommended
`SchemaId` convention for uncoordinated deployments: SHA-256 over the
encoded schema descriptor bytes only.

This is a helper, not a wire requirement. `SchemaId` remains opaque in
the core format, and deployments may still use registry-issued or
application-defined identifiers.

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
- `docs/implementation-status.md` in the repository root summarizes the
  current implementation boundary
