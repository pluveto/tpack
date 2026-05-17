# TPACK Test Vectors

This directory contains the public byte-level vectors used as interop
anchors by the Rust reference implementation.

The example vectors are consumed directly by
`crates/tpack/tests/reference.rs`. Cached-schema validation behavior is
covered separately by `crates/tpack/tests/cache_validation.rs`.

The initial scope is intentionally small:

- draft Examples-section flat-record examples in all three envelope modes
- one repository-defined negative vector for canonical map ordering

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
| Non-canonical map order | `v1/reference/noncanonical-map-order/full-schema.hex` | Repository regression vector | Strict canonical decode must fail with `NonCanonicalMapKeyOrder` |

SchemaId-related vectors currently exercise the draft's string example
and the default fail-closed cache behavior. The documentation now also
defines two hash-based naming profiles for canonical schema descriptor
bytes:

- open interoperability default: SHA-256
- official compact profile: `xxh64-v1` = `xxHash64(seed=0)`, fixed
  8-byte big-endian output

Neither profile authenticates a binding by itself. Compact-profile
deployments still need an explicit scope and must reject `SchemaRef`
when the binding context is missing, expired, ambiguous, or conflicting.

## Quick Checks

```bash
cargo test -p tpack --test reference draft_examples_envelopes_decode_and_canonicalize
cargo test -p tpack --test reference canonical_map_ordering_and_nan_are_enforced
cargo test -p tpack --test cache_validation
```
