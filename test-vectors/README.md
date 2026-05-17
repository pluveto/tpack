# TPACK Test Vectors

This directory contains the public byte-level vectors used as interop
anchors by the Rust reference implementation.

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
| Flat record, FullSchemaWithId | `v1/draft-00/flat-record/full-schema-with-id.hex` | `draft-zhang-tpack-format-00` Section 15.4 | Decodes successfully; can reuse a cached schema if `example.record.v1` is trusted |
| Flat record, SchemaRef | `v1/draft-00/flat-record/schema-ref.hex` | `draft-zhang-tpack-format-00` Section 15.5 | Requires an external binding for `example.record.v1`; otherwise decode must fail |
| Non-canonical map order | `v1/reference/noncanonical-map-order/full-schema.hex` | Repository regression vector | Strict canonical decode must fail with `NonCanonicalMapKeyOrder` |

## Quick Checks

```bash
cargo test -p tpack --test reference draft_examples_envelopes_decode_and_canonicalize
cargo test -p tpack --test reference canonical_map_ordering_and_nan_are_enforced
```
