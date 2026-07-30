# TPACK benchmark matrix

Generated from structured samples (`tpack_bench::matrix::run` → `render::markdown`).

```bash
cargo run -p tpack-bench --example size_report
cargo run -p tpack-bench --release --example matrix_report
```

## How to read this

| Regime | What matters | Look at |
|--------|--------------|--------|
| Tiny self-contained message | Schema tax | Cold FullSchema sizes |
| Steady multi-message | No repeated keys | SchemaRef + amortization |
| Large payload | Bandwidth | Blob scale + MiB/s |
| Shared registry | Locking | Concurrent decode |

**Cold and warm paths are labeled separately.** Naive `encode_message` re-encodes schema on FullSchema; hot paths use `PreparedSchema`.

## 1. Deterministic sizes

### 1.1 Steady-state (SchemaRef primary)

| Case | Format | Bytes | vs JSON | vs CBOR |
|------|--------|------:|--------:|--------:|
| `flat_record` | `tpack-schemaref` | 44 | 0.60× | 0.92× |
| `flat_record` | `json` | 73 | 1.00× | 1.52× |
| `flat_record` | `cbor` | 48 | 0.66× | 1.00× |
| `flat_record` | `msgpack` | 48 | 0.66× | 1.00× |
| `nested_struct` | `tpack-schemaref` | 84 | 0.43× | 0.56× |
| `nested_struct` | `json` | 197 | 1.00× | 1.31× |
| `nested_struct` | `cbor` | 150 | 0.76× | 1.00× |
| `nested_struct` | `msgpack` | 150 | 0.76× | 1.00× |
| `list_100` | `tpack-schemaref` | 2916 | 0.39× | 0.61× |
| `list_100` | `json` | 7401 | 1.00× | 1.54× |
| `list_100` | `cbor` | 4802 | 0.65× | 1.00× |
| `list_100` | `msgpack` | 4803 | 0.65× | 1.00× |

### 1.2 Cold FullSchema

| Case | Format | Bytes | vs JSON | vs CBOR |
|------|--------|------:|--------:|--------:|
| `flat_record` | `tpack-fullschema` | 76 | 1.04× | 1.58× |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 1.16× | 1.77× |
| `nested_struct` | `tpack-fullschema` | 147 | 0.75× | 0.98× |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 0.79× | 1.04× |
| `list_100` | `tpack-fullschema` | 2950 | 0.40× | 0.61× |
| `list_100` | `tpack-fullschema-with-id` | 2959 | 0.40× | 0.62× |

### 1.3 Component breakdown

| Case | Format | Total | Header | SchemaId | Schema | Data |
|------|--------|------:|-------:|---------:|-------:|-----:|
| `flat_record` | `tpack-fullschema` | 76 | 6 | 0 | 40 | 29 |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 6 | 8 | 40 | 29 |
| `flat_record` | `tpack-schemaref` | 44 | 6 | 8 | 0 | 29 |
| `nested_struct` | `tpack-fullschema` | 147 | 6 | 0 | 71 | 69 |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 6 | 8 | 71 | 69 |
| `nested_struct` | `tpack-schemaref` | 84 | 6 | 8 | 0 | 69 |
| `list_100` | `tpack-fullschema` | 2950 | 6 | 0 | 42 | 2901 |
| `list_100` | `tpack-fullschema-with-id` | 2959 | 6 | 8 | 42 | 2901 |
| `list_100` | `tpack-schemaref` | 2916 | 6 | 8 | 0 | 2901 |

### 1.4 Amortization scan (1× WithId + (n−1)× SchemaRef)

#### `flat_record`

| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |
|--:|------:|-----:|-----:|--------:|--------:|
| 1 | 85 | 73 | 48 | 1.16× | 1.77× |
| 10 | 481 | 730 | 480 | 0.66× | 1.00× |
| 100 | 4441 | 7300 | 4800 | 0.61× | 0.93× |
| 1000 | 44041 | 73000 | 48000 | 0.60× | 0.92× |
| 10000 | 440041 | 730000 | 480000 | 0.60× | 0.92× |

#### `list_100`

| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |
|--:|------:|-----:|-----:|--------:|--------:|
| 1 | 2959 | 7401 | 4802 | 0.40× | 0.62× |
| 10 | 29203 | 74010 | 48020 | 0.39× | 0.61× |
| 100 | 291643 | 740100 | 480200 | 0.39× | 0.61× |
| 1000 | 2916043 | 7401000 | 4802000 | 0.39× | 0.61× |
| 10000 | 29160043 | 74010000 | 48020000 | 0.39× | 0.61× |

#### `nested_struct`

| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |
|--:|------:|-----:|-----:|--------:|--------:|
| 1 | 156 | 197 | 150 | 0.79× | 1.04× |
| 10 | 912 | 1970 | 1500 | 0.46× | 0.61× |
| 100 | 8472 | 19700 | 15000 | 0.43× | 0.56× |
| 1000 | 84072 | 197000 | 150000 | 0.43× | 0.56× |
| 10000 | 840072 | 1970000 | 1500000 | 0.43× | 0.56× |

### 1.5 Scale: blob payload

| Case | Format | Bytes | vs JSON | vs CBOR |
|------|--------|------:|--------:|--------:|
| `blob_0` | `cbor` | 18 | 0.69× | 1.00× |
| `blob_0` | `json` | 26 | 1.00× | 1.44× |
| `blob_0` | `msgpack` | 18 | 0.69× | 1.00× |
| `blob_0` | `tpack-fullschema` | 33 | 1.27× | 1.83× |
| `blob_0` | `tpack-fullschema-with-id` | 42 | 1.62× | 2.33× |
| `blob_0` | `tpack-schemaref` | 24 | 0.92× | 1.33× |
| `blob_1024` | `cbor` | 1972 | 0.54× | 1.00× |
| `blob_1024` | `json` | 3681 | 1.00× | 1.87× |
| `blob_1024` | `msgpack` | 1556 | 0.42× | 0.79× |
| `blob_1024` | `tpack-fullschema` | 1058 | 0.29× | 0.54× |
| `blob_1024` | `tpack-fullschema-with-id` | 1067 | 0.29× | 0.54× |
| `blob_1024` | `tpack-schemaref` | 1049 | 0.28× | 0.53× |
| `blob_1048576` | `cbor` | 1998870 | 0.53× | 1.00× |
| `blob_1048576` | `json` | 3743769 | 1.00× | 1.87× |
| `blob_1048576` | `msgpack` | 1572886 | 0.42× | 0.79× |
| `blob_1048576` | `tpack-schemaref` | 1048602 | 0.28× | 0.52× |
| `blob_16384` | `cbor` | 31252 | 0.53× | 1.00× |
| `blob_16384` | `json` | 58521 | 1.00× | 1.87× |
| `blob_16384` | `msgpack` | 24596 | 0.42× | 0.79× |
| `blob_16384` | `tpack-schemaref` | 16410 | 0.28× | 0.53× |
| `blob_64` | `cbor` | 141 | 0.56× | 1.00× |
| `blob_64` | `json` | 251 | 1.00× | 1.78× |
| `blob_64` | `msgpack` | 115 | 0.46× | 0.82× |
| `blob_64` | `tpack-fullschema` | 97 | 0.39× | 0.69× |
| `blob_64` | `tpack-fullschema-with-id` | 106 | 0.42× | 0.75× |
| `blob_64` | `tpack-schemaref` | 88 | 0.35× | 0.62× |

### 1.6 Scale: list length

| Case | Format | Bytes | vs JSON | vs CBOR |
|------|--------|------:|--------:|--------:|
| `list_1` | `cbor` | 49 | 0.65× | 1.00× |
| `list_1` | `json` | 75 | 1.00× | 1.53× |
| `list_1` | `msgpack` | 49 | 0.65× | 1.00× |
| `list_1` | `tpack-fullschema` | 79 | 1.05× | 1.61× |
| `list_1` | `tpack-fullschema-with-id` | 88 | 1.17× | 1.80× |
| `list_1` | `tpack-schemaref` | 45 | 0.60× | 0.92× |
| `list_10` | `cbor` | 481 | 0.65× | 1.00× |
| `list_10` | `json` | 741 | 1.00× | 1.54× |
| `list_10` | `msgpack` | 481 | 0.65× | 1.00× |
| `list_10` | `tpack-fullschema` | 340 | 0.46× | 0.71× |
| `list_10` | `tpack-fullschema-with-id` | 349 | 0.47× | 0.73× |
| `list_10` | `tpack-schemaref` | 306 | 0.41× | 0.64× |
| `list_100` | `cbor` | 4802 | 0.65× | 1.00× |
| `list_100` | `json` | 7401 | 1.00× | 1.54× |
| `list_100` | `msgpack` | 4803 | 0.65× | 1.00× |
| `list_100` | `tpack-fullschema` | 2950 | 0.40× | 0.61× |
| `list_100` | `tpack-fullschema-with-id` | 2959 | 0.40× | 0.62× |
| `list_100` | `tpack-schemaref` | 2916 | 0.39× | 0.61× |
| `list_1000` | `cbor` | 48003 | 0.65× | 1.00× |
| `list_1000` | `json` | 74001 | 1.00× | 1.54× |
| `list_1000` | `msgpack` | 48003 | 0.65× | 1.00× |
| `list_1000` | `tpack-schemaref` | 29017 | 0.39× | 0.60× |
| `list_10000` | `cbor` | 489003 | 0.65× | 1.00× |
| `list_10000` | `json` | 749001 | 1.00× | 1.53× |
| `list_10000` | `msgpack` | 489003 | 0.65× | 1.00× |
| `list_10000` | `tpack-schemaref` | 299017 | 0.40× | 0.61× |

### 1.7 Scale: field count

| Case | Format | Bytes | vs JSON | vs CBOR |
|------|--------|------:|--------:|--------:|
| `fields_16` | `cbor` | 86 | 0.60× | 1.00× |
| `fields_16` | `json` | 144 | 1.00× | 1.67× |
| `fields_16` | `msgpack` | 82 | 0.57× | 0.95× |
| `fields_16` | `tpack-fullschema` | 239 | 1.66× | 2.78× |
| `fields_16` | `tpack-fullschema-with-id` | 248 | 1.72× | 2.88× |
| `fields_16` | `tpack-schemaref` | 143 | 0.99× | 1.66× |
| `fields_256` | `cbor` | 1924 | 0.66× | 1.00× |
| `fields_256` | `json` | 2897 | 1.00× | 1.51× |
| `fields_256` | `msgpack` | 1918 | 0.66× | 1.00× |
| `fields_256` | `tpack-fullschema` | 4126 | 1.42× | 2.14× |
| `fields_256` | `tpack-fullschema-with-id` | 4135 | 1.43× | 2.15× |
| `fields_256` | `tpack-schemaref` | 2063 | 0.71× | 1.07× |
| `fields_4` | `cbor` | 19 | 0.59× | 1.00× |
| `fields_4` | `json` | 32 | 1.00× | 1.68× |
| `fields_4` | `msgpack` | 17 | 0.53× | 0.89× |
| `fields_4` | `tpack-fullschema` | 65 | 2.03× | 3.42× |
| `fields_4` | `tpack-fullschema-with-id` | 74 | 2.31× | 3.89× |
| `fields_4` | `tpack-schemaref` | 47 | 1.47× | 2.47× |
| `fields_64` | `cbor` | 423 | 0.67× | 1.00× |
| `fields_64` | `json` | 629 | 1.00× | 1.49× |
| `fields_64` | `msgpack` | 418 | 0.66× | 0.99× |
| `fields_64` | `tpack-fullschema` | 960 | 1.53× | 2.27× |
| `fields_64` | `tpack-fullschema-with-id` | 969 | 1.54× | 2.29× |
| `fields_64` | `tpack-schemaref` | 527 | 0.84× | 1.25× |

## 2. Host-dependent performance

**Host:** linux/x86_64, 32 logical CPUs, AMD Ryzen 9 7950X 16-Core Processor

Not portable. Re-run before optimizing.

### 2.1 PreparedSchema speedup (`flat_record` FullSchema)

| Path | ns/op | vs naive |
|------|------:|---------:|
| naive `encode_message` | 551.1 | 1.00× |
| prepared + reuse | 216.3 | 2.55× |
| SchemaRef warm | 217.6 | 2.53× |

### 2.2 Latency tails

| Case | Format | Op | Path | n | p50 | p90 | p99 | max | mean | B/op | MiB/s@p50 |
|------|--------|----|------|--:|----:|----:|----:|----:|-----:|-----:|----------:|
| `flat_record` | `tpack-schemaref` | encode | warm-prepared | 5000 | 241 | 261 | 271 | 5971 | 246 | 44 | 174.1 |
| `flat_record` | `tpack-schemaref` | decode | warm-prepared | 5000 | 311 | 321 | 331 | 16550 | 321 | 44 | 134.9 |
| `flat_record` | `tpack-schemaref` | roundtrip | warm-prepared | 5000 | 551 | 572 | 651 | 4979 | 559 | 44 | 76.2 |
| `flat_record` | `tpack-fullschema` | encode | naive-message | 5000 | 561 | 611 | 702 | 5520 | 573 | 76 | 129.2 |
| `flat_record` | `tpack-fullschema` | encode | warm-prepared | 5000 | 241 | 251 | 260 | 4909 | 246 | 76 | 300.7 |
| `flat_record` | `json` | encode | naive-message | 5000 | 80 | 90 | 91 | 100 | 81 | 73 | 870.2 |
| `flat_record` | `cbor` | encode | naive-message | 5000 | 140 | 161 | 180 | 4038 | 143 | 48 | 327.0 |
| `flat_record` | `json` | decode | naive-message | 5000 | 311 | 340 | 361 | 4348 | 318 | 73 | 223.9 |
| `nested_struct` | `tpack-schemaref` | encode | warm-prepared | 5000 | 361 | 381 | 391 | 4058 | 365 | 84 | 221.9 |
| `nested_struct` | `tpack-schemaref` | decode | warm-prepared | 5000 | 591 | 601 | 621 | 4599 | 591 | 84 | 135.5 |
| `nested_struct` | `tpack-schemaref` | roundtrip | warm-prepared | 5000 | 962 | 1012 | 1443 | 20167 | 982 | 84 | 83.3 |
| `nested_struct` | `tpack-fullschema` | encode | naive-message | 5000 | 801 | 862 | 1212 | 10720 | 815 | 147 | 175.0 |
| `nested_struct` | `tpack-fullschema` | encode | warm-prepared | 5000 | 361 | 391 | 401 | 4408 | 369 | 147 | 388.3 |
| `nested_struct` | `json` | encode | naive-message | 5000 | 190 | 231 | 261 | 5981 | 197 | 197 | 988.8 |
| `nested_struct` | `cbor` | encode | naive-message | 5000 | 301 | 330 | 361 | 8776 | 308 | 150 | 475.3 |
| `list_100` | `tpack-schemaref` | encode | warm-prepared | 5000 | 23113 | 23323 | 28031 | 41326 | 23280 | 2916 | 120.3 |
| `list_100` | `tpack-schemaref` | decode | warm-prepared | 5000 | 27651 | 28823 | 32200 | 48399 | 27909 | 2916 | 100.6 |
| `list_100` | `tpack-schemaref` | roundtrip | warm-prepared | 5000 | 47007 | 51245 | 63147 | 102930 | 48819 | 2916 | 59.2 |
| `list_100` | `tpack-fullschema` | encode | naive-message | 5000 | 21750 | 22041 | 26069 | 52637 | 21792 | 2950 | 129.3 |
| `list_100` | `tpack-fullschema` | encode | warm-prepared | 5000 | 21229 | 21420 | 25347 | 40515 | 21348 | 2950 | 132.5 |
| `list_100` | `json` | encode | naive-message | 5000 | 5781 | 5971 | 6502 | 10279 | 5829 | 7401 | 1220.9 |
| `list_100` | `cbor` | encode | naive-message | 5000 | 7153 | 7203 | 7874 | 18063 | 7183 | 4802 | 640.2 |
| `blob_0` | `tpack-schemaref` | encode | warm-prepared | 5000 | 30 | 30 | 40 | 41 | 30 | 24 | 762.9 |
| `blob_64` | `tpack-schemaref` | encode | warm-prepared | 5000 | 30 | 30 | 40 | 51 | 30 | 88 | 2797.4 |
| `blob_1024` | `tpack-schemaref` | encode | warm-prepared | 5000 | 40 | 40 | 41 | 50 | 38 | 1049 | 25010.1 |
| `blob_16384` | `tpack-schemaref` | encode | warm-prepared | 5000 | 190 | 200 | 211 | 3577 | 189 | 16410 | 82367.3 |
| `blob_1048576` | `tpack-schemaref` | encode | warm-prepared | 200 | 14857 | 14937 | 19175 | 19977 | 14954 | 1048602 | 67310.0 |

### 2.3 Concurrent SchemaRef decode

| Mode | threads | total | wall | msgs/s | note |
|------|--------:|------:|-----:|-------:|------|
| shared-registry | 8 | 80000 | 17.337ms | 4614305 | shared StdSchemaRegistry (RwLock) |
| per-thread-registry | 8 | 80000 | 13.939ms | 5739189 | per-thread registry (no shared lock) |

### 2.4 Allocations

| Case | Format | Op | Path | allocs | bytes_alloc | out_B |
|------|--------|----|------|-------:|------------:|------:|
| `flat_record` | `tpack-schemaref` | encode | warm-prepared | 10 | 189 | 44 |
| `flat_record` | `tpack-fullschema` | encode | naive-message | 13 | 579 | 76 |
| `flat_record` | `tpack-fullschema` | encode | warm-prepared | 10 | 219 | 76 |
| `flat_record` | `tpack-schemaref` | decode | warm-prepared | 3 | 293 | 44 |
| `flat_record` | `json` | encode | naive-message | 1 | 128 | 73 |

Warm prepared encode still allocates on the value path today (`BigInt`, temps). Zero-alloc is a future target, not a claim.

## 3. Regime conclusions

1. Tiny cold FullSchema loses to CBOR on **schema tax** (structure, not a slow encoder).
2. SchemaRef / amortization wins as nesting, list length, or n grows.
3. Prepared FullSchema ≫ naive `encode_message`; never quote naive as hot path.
4. Large blobs: formats converge (payload dominates); check MiB/s before micro-optimizing CPU.
5. Warm encode is **not zero-alloc** yet — primary next optimization surface.
6. JSON can win tiny single-thread encode latency; TPACK’s case is structural density + multi-message.

## 4. Methodology

- Architecture: `Workload` → `measure::*` → `Report` → `render::markdown`.
- Competitors: named-map semantic twins (DecimalFixed→i64, Decimal→string).
- Golden: `flat_record` FullSchema == **76** bytes (draft-00).
- Criterion optional: `cargo bench -p tpack-bench` (not default CI).
