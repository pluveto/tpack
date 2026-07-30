# TPACK size & throughput benchmarks

This document is **generated** by the `tpack-bench` harness.

Regenerate the size table with:

```bash
cargo run -p tpack-bench --example size_report
```

Sizes are deterministic (same on every machine). Throughput numbers
depend on hardware and are **not** checked into this table as absolute
truth; see [Example throughput](#example-throughput-informational).

## Methodology

### Formats

| Label | What is measured |
|-------|------------------|
| `tpack-fullschema` | `EnvelopeMode::FullSchema` (schema + data) |
| `tpack-fullschema-with-id` | `FullSchemaWithId` + official `xxh64-v1` 8-byte schema id |
| `tpack-schemaref` | `SchemaRef` (id + data only; decode uses a preloaded registry) |
| `json` | Compact `serde_json` (no pretty-print) |
| `cbor` | `ciborium` (CBOR via serde) |
| `msgpack` | `rmp-serde` named maps |

### Semantic twins

JSON / CBOR / MessagePack do not share TPACK's native type system.
Competitor payloads are *semantic twins*:

- String and integer fields map directly.
- `DecimalFixed` becomes an `i64` unscaled coefficient (scale is schema-side in TPACK).
- `Decimal` becomes a decimal **string** (e.g. `"13.725"`) so competitors are not forced into binary floats.
- Nested structs and lists use ordinary serde structs / `Vec`.

TPACK numbers therefore include typed schema descriptors; competitor numbers include
repeated field names (JSON/MessagePack maps) or CBOR map keys. Neither side is
claimed to "win" every column.

### Scenarios

- **`flat_record`**: Draft flat-record example: id, DecimalFixed price, Decimal tax, qty, ts
- **`nested_struct`**: Order header plus three line items (sku, qty, unit_price) to stress key/schema overhead
- **`bulk_list`**: List of 100 flat-like product rows (amortized FullSchema cost)

## Size comparison

| Scenario | Format | Bytes | vs JSON |
|----------|--------|------:|--------:|
| `flat_record` | `tpack-fullschema` | 76 | 1.04× |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 1.16× |
| `flat_record` | `tpack-schemaref` | 44 | 0.60× |
| `flat_record` | `json` | 73 | 1.00× |
| `flat_record` | `cbor` | 48 | 0.66× |
| `flat_record` | `msgpack` | 48 | 0.66× |
| `nested_struct` | `tpack-fullschema` | 147 | 0.75× |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 0.79× |
| `nested_struct` | `tpack-schemaref` | 84 | 0.43× |
| `nested_struct` | `json` | 197 | 1.00× |
| `nested_struct` | `cbor` | 150 | 0.76× |
| `nested_struct` | `msgpack` | 150 | 0.76× |
| `bulk_list` | `tpack-fullschema` | 2950 | 0.40× |
| `bulk_list` | `tpack-fullschema-with-id` | 2959 | 0.40× |
| `bulk_list` | `tpack-schemaref` | 2916 | 0.39× |
| `bulk_list` | `json` | 7401 | 1.00× |
| `bulk_list` | `cbor` | 4802 | 0.65× |
| `bulk_list` | `msgpack` | 4803 | 0.65× |

`vs JSON` is `format_bytes / json_bytes` for the same scenario (lower is smaller).

## Example throughput (informational)

Throughput is measured with [Criterion](https://github.com/bheisler/criterion.rs).
**Do not** treat absolute ns/iter numbers as portable; re-run on your hardware.

```bash
# Full Criterion run (writes HTML under target/criterion/)
cargo bench -p tpack-bench

# Faster smoke run
cargo bench -p tpack-bench -- --quick
```

Benches cover encode and decode for each scenario × format pair.
`tpack-schemaref` decode uses a registry preloaded outside the timed loop.

| Environment | Notes |
|-------------|-------|
| CPU | *run locally / on CI runner; fill in when publishing numbers* |
| Command | `cargo bench -p tpack-bench` |
| CI policy | Full Criterion is **not** part of default CI |

## Golden size guard

Unit tests in `tpack-bench` pin the `flat_record` × `tpack-fullschema` size to the
draft-00 test vector length so accidental encoding drift fails CI without running
multi-minute benches.
