# TPACK size & throughput benchmarks

This document is **generated** by the `tpack-bench` harness.

```bash
cargo run -p tpack-bench --example size_report
```

Sizes are deterministic. Throughput is hardware-specific; see below.

## How to read this table

TPACK has three envelope modes with different cost profiles:

- **`tpack-schemaref`** (primary steady-state metric): schema lives in a registry; message is id + data.
- **`tpack-fullschema` / `with-id`**: self-contained; **includes the full schema descriptor** every message (cold / bootstrap).
- Competing JSON/CBOR/MessagePack maps repeat field names every message.

A single tiny FullSchema message is **not** the design center for density vs CBOR.
Prefer SchemaRef rows and the [amortized multi-message](#amortized-multi-message-stream) section.

## Methodology

### Formats

| Label | What is measured |
|-------|------------------|
| `tpack-fullschema` | Cold self-contained message (schema + data) |
| `tpack-fullschema-with-id` | Cold + `xxh64-v1` schema id |
| `tpack-schemaref` | Steady-state id + data (registry on decode) |
| `json` | Compact `serde_json` |
| `cbor` | `ciborium` named maps |
| `msgpack` | `rmp-serde` named maps |

### Semantic twins

Competitors encode *semantic twins* (i64 unscaled for DecimalFixed, decimal **string** for Decimal, serde structs for nesting).
They are not the same type system; comparisons are about application payload footprint.

### Scenarios

- **`flat_record`**: Draft flat-record example: id, DecimalFixed price, Decimal tax, qty, ts
- **`nested_struct`**: Order header plus three line items (sku, qty, unit_price) to stress key/schema overhead
- **`bulk_list`**: List of 100 flat-like product rows (amortized FullSchema cost)

## Steady-state size (SchemaRef primary)

| Scenario | Format | Bytes | vs JSON | vs CBOR |
|----------|--------|------:|--------:|--------:|
| `flat_record` | `tpack-schemaref` | 44 | 0.60× | 0.92× |
| `flat_record` | `json` | 73 | 1.00× | 1.52× |
| `flat_record` | `cbor` | 48 | 0.66× | 1.00× |
| `flat_record` | `msgpack` | 48 | 0.66× | 1.00× |
| `nested_struct` | `tpack-schemaref` | 84 | 0.43× | 0.56× |
| `nested_struct` | `json` | 197 | 1.00× | 1.31× |
| `nested_struct` | `cbor` | 150 | 0.76× | 1.00× |
| `nested_struct` | `msgpack` | 150 | 0.76× | 1.00× |
| `bulk_list` | `tpack-schemaref` | 2916 | 0.39× | 0.61× |
| `bulk_list` | `json` | 7401 | 1.00× | 1.54× |
| `bulk_list` | `cbor` | 4802 | 0.65× | 1.00× |
| `bulk_list` | `msgpack` | 4803 | 0.65× | 1.00× |

## Cold / self-contained size (FullSchema)

Includes embedded schema every message — expected to lose to CBOR on **tiny** single records.

| Scenario | Format | Bytes | vs JSON | vs CBOR |
|----------|--------|------:|--------:|--------:|
| `flat_record` | `tpack-fullschema` | 76 | 1.04× | 1.58× |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 1.16× | 1.77× |
| `nested_struct` | `tpack-fullschema` | 147 | 0.75× | 0.98× |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 0.79× | 1.04× |
| `bulk_list` | `tpack-fullschema` | 2950 | 0.40× | 0.61× |
| `bulk_list` | `tpack-fullschema-with-id` | 2959 | 0.40× | 0.62× |

## TPACK component breakdown

| Scenario | Mode | Total | Header+ver+mode | SchemaId payload | Schema | Data |
|----------|------|------:|----------------:|-----------------:|-------:|-----:|
| `flat_record` | `tpack-fullschema` | 76 | 6 | 0 | 40 | 29 |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 6 | 8 | 40 | 29 |
| `flat_record` | `tpack-schemaref` | 44 | 6 | 8 | 0 | 29 |
| `nested_struct` | `tpack-fullschema` | 147 | 6 | 0 | 71 | 69 |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 6 | 8 | 71 | 69 |
| `nested_struct` | `tpack-schemaref` | 84 | 6 | 8 | 0 | 69 |
| `bulk_list` | `tpack-fullschema` | 2950 | 6 | 0 | 42 | 2901 |
| `bulk_list` | `tpack-fullschema-with-id` | 2959 | 6 | 8 | 42 | 2901 |
| `bulk_list` | `tpack-schemaref` | 2916 | 6 | 8 | 0 | 2901 |

Length prefixes for schema id / schema len are part of total but not expanded as separate columns.

## Amortized multi-message stream

Models a realistic session: **1× FullSchemaWithId** (cold bind) + **99× SchemaRef** (hot).
Competitors pay full map size every message.

| Scenario | Stream | Total bytes (n=100) | Per message avg | vs 100× JSON | vs 100× CBOR |
|----------|--------|--------------------:|----------------:|-------------:|-------------:|
| `flat_record` | `tpack-1×with-id+(n-1)×schemaref` | 4441 | 44.4 | 0.61× | 0.93× |
| `flat_record` | `json` ×100 | 7300 | 73.0 | 1.00× | 1.52× |
| `flat_record` | `cbor` ×100 | 4800 | 48.0 | 0.66× | 1.00× |
| `nested_struct` | `tpack-1×with-id+(n-1)×schemaref` | 8472 | 84.7 | 0.43× | 0.56× |
| `nested_struct` | `json` ×100 | 19700 | 197.0 | 1.00× | 1.31× |
| `nested_struct` | `cbor` ×100 | 15000 | 150.0 | 0.76× | 1.00× |
| `bulk_list` | `tpack-1×with-id+(n-1)×schemaref` | 291643 | 2916.4 | 0.39× | 0.61× |
| `bulk_list` | `json` ×100 | 740100 | 7401.0 | 1.00× | 1.54× |
| `bulk_list` | `cbor` ×100 | 480200 | 4802.0 | 0.65× | 1.00× |

## Throughput (informational)

Criterion benches use **PreparedSchema + Encoder buffer reuse** (steady-state API).
Naive `encode_message` still re-encodes schema for FullSchema modes; prefer `PreparedSchema` in hot loops.

```bash
cargo bench -p tpack-bench
cargo bench -p tpack-bench -- --quick
```

| Environment | Notes |
|-------------|-------|
| CPU | *fill in when publishing absolute numbers* |
| CI policy | Full Criterion is **not** part of default CI |

## Golden guards

- `flat_record` × FullSchema == 76 (draft-00 vector length)
- Component breakdown sums are consistent with total size
