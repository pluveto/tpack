# TPACK Test Vectors

This directory contains the public byte-level vectors used as interop
anchors by the Rust reference implementation.

The example vectors are consumed directly by
`crates/tpack/tests/reference.rs`. Cached-schema validation behavior is
covered separately by `crates/tpack/tests/cache_validation.rs`.

The initial scope is intentionally small:

- draft Examples-section flat-record examples in all three envelope modes
- repository-defined reference vectors for the official `xxh64-v1`
  SchemaId profile and for canonical map ordering

Keep numbered-draft vectors immutable. If a future draft revision
changes example bytes, add a new sibling directory such as
`v1/draft-01/` instead of rewriting `v1/draft-00/`.

## Encoding Format

All `.hex` files use uppercase hexadecimal octets separated by
whitespace only. Blank lines are allowed.

## Current Vectors

| Vector | Path | Source | Expected result |
| --- | --- | --- | --- |
| Flat record, FullSchema | `v1/draft-00/flat-record/full-schema.hex` | `draft-zhang-tpack-format-00` Section 15.1 | Decodes successfully as a self-contained message |
| Flat record, FullSchemaWithId | `v1/draft-00/flat-record/full-schema-with-id.hex` | `draft-zhang-tpack-format-00` Section 15.4 | Decodes successfully without a registry; on a registry hit the reference implementation only reuses the cached schema after the embedded descriptor matches, otherwise decode fails |
| Flat record, SchemaRef | `v1/draft-00/flat-record/schema-ref.hex` | `draft-zhang-tpack-format-00` Section 15.5 | Requires an external binding for `example.record.v1`; if the binding is missing, ambiguous, or out of profile scope, decode must fail |
| Flat record, FullSchemaWithId (`xxh64-v1`) | `v1/reference/xxh64-v1-flat-record/full-schema-with-id.hex` | Repository reference vector for official `xxh64-v1` | Same flat-record schema/value as the draft examples, but SchemaId is the fixed 8-byte BE `xxh64-v1` digest; decodes without a registry |
| Flat record, SchemaRef (`xxh64-v1`) | `v1/reference/xxh64-v1-flat-record/schema-ref.hex` | Repository reference vector for official `xxh64-v1` | Requires a registry binding for the 8-byte `xxh64-v1` SchemaId; fails closed without that binding |
| Non-canonical map order | `v1/reference/noncanonical-map-order/full-schema.hex` | Repository regression vector | Strict canonical decode must fail with `NonCanonicalMapKeyOrder` |

The draft `flat-record` vectors exercise the draft's opaque string
SchemaId example (`example.record.v1`) and the default fail-closed
cache behavior. The documentation defines one official recommended
naming profile for canonical schema descriptor bytes:

- `xxh64-v1` = `xxHash64(seed=0)`, fixed 8-byte big-endian output

The `v1/reference/xxh64-v1-flat-record/` vectors are the on-wire
anchors for that profile. For the shared flat-record schema the digest
is `23 73 76 F7 21 B6 0A 41`. Regenerate them with:

```bash
cargo run -p tpack --example gen_xxh64_v1_vectors
```

`xxh64-v1` does not authenticate a binding by itself. Deployments that
use it still need an explicit bounded or registry-backed scope and must
reject `SchemaRef` when the binding context is missing, reset, expired,
ambiguous, conflicting, or otherwise out of scope.

## Quick Checks

```bash
cargo test -p tpack --test reference draft_examples_envelopes_decode_and_canonicalize
cargo test -p tpack --test reference xxh64_v1_flat_record_vectors_decode_with_official_schema_id
cargo test -p tpack --test reference canonical_map_ordering_and_nan_are_enforced
cargo test -p tpack --test cache_validation
```
