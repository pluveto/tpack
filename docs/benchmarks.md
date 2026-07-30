# TPACK benchmark matrix

This document is **generated**. Do not hand-edit numbers.

```bash
# Deterministic sizes only (fast, CI-safe)
cargo run -p tpack-bench --example size_report

# Full matrix: sizes + latency tails + allocs + concurrency (host-labeled)
cargo run -p tpack-bench --release --example matrix_report
```

## How to read this

TPACK is not one number. Regime matters:

| Regime | What matters | Primary metric |
|--------|--------------|----------------|
| Tiny single message, self-contained | Schema tax | FullSchema size vs CBOR map |
| Steady multi-message | No repeated keys | SchemaRef size + warm CPU |
| Large payload | Bandwidth / memcpy | MiB/s on blob scale |
| Shared registry, multi-thread | Locking | Concurrent decode msgs/s |

**Cold vs warm must never be mixed.** Naive `encode_message` re-encodes the schema every FullSchema call; hot paths must use `PreparedSchema` + a reused `Encoder`.

## 1. Deterministic size matrix

These sizes are machine-independent (same on every host).

### 1.1 Steady-state size (SchemaRef primary)

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

### 1.2 Cold / self-contained size (FullSchema)

Includes embedded schema every message. Expected to lose to CBOR on **tiny** single records.

| Scenario | Format | Bytes | vs JSON | vs CBOR |
|----------|--------|------:|--------:|--------:|
| `flat_record` | `tpack-fullschema` | 76 | 1.04× | 1.58× |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 1.16× | 1.77× |
| `nested_struct` | `tpack-fullschema` | 147 | 0.75× | 0.98× |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 0.79× | 1.04× |
| `bulk_list` | `tpack-fullschema` | 2950 | 0.40× | 0.61× |
| `bulk_list` | `tpack-fullschema-with-id` | 2959 | 0.40× | 0.62× |

### 1.3 TPACK component breakdown

| Scenario | Mode | Total | Hdr+ver+mode | SchemaId | Schema | Data |
|----------|------|------:|-------------:|---------:|-------:|-----:|
| `flat_record` | `tpack-fullschema` | 76 | 6 | 0 | 40 | 29 |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 6 | 8 | 40 | 29 |
| `flat_record` | `tpack-schemaref` | 44 | 6 | 8 | 0 | 29 |
| `nested_struct` | `tpack-fullschema` | 147 | 6 | 0 | 71 | 69 |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 6 | 8 | 71 | 69 |
| `nested_struct` | `tpack-schemaref` | 84 | 6 | 8 | 0 | 69 |
| `bulk_list` | `tpack-fullschema` | 2950 | 6 | 0 | 42 | 2901 |
| `bulk_list` | `tpack-fullschema-with-id` | 2959 | 6 | 8 | 42 | 2901 |
| `bulk_list` | `tpack-schemaref` | 2916 | 6 | 8 | 0 | 2901 |

Length prefixes for id/schema are included in total but not separate columns.

### 1.4 Amortized multi-message scan

Stream model: **1× FullSchemaWithId** (cold bind) + **(n−1)× SchemaRef** (hot). Competitors pay full size every message.

#### `flat_record`

| n | TPACK total | TPACK avg/msg | JSON total | CBOR total | vs JSON | vs CBOR |
|--:|------------:|--------------:|-----------:|-----------:|--------:|--------:|
| 1 | 85 | 85.0 | 73 | 48 | 1.16× | 1.77× |
| 10 | 481 | 48.1 | 730 | 480 | 0.66× | 1.00× |
| 100 | 4441 | 44.4 | 7300 | 4800 | 0.61× | 0.93× |
| 1000 | 44041 | 44.0 | 73000 | 48000 | 0.60× | 0.92× |
| 10000 | 440041 | 44.0 | 730000 | 480000 | 0.60× | 0.92× |

#### `nested_struct`

| n | TPACK total | TPACK avg/msg | JSON total | CBOR total | vs JSON | vs CBOR |
|--:|------------:|--------------:|-----------:|-----------:|--------:|--------:|
| 1 | 156 | 156.0 | 197 | 150 | 0.79× | 1.04× |
| 10 | 912 | 91.2 | 1970 | 1500 | 0.46× | 0.61× |
| 100 | 8472 | 84.7 | 19700 | 15000 | 0.43× | 0.56× |
| 1000 | 84072 | 84.1 | 197000 | 150000 | 0.43× | 0.56× |
| 10000 | 840072 | 84.0 | 1970000 | 1500000 | 0.43× | 0.56× |

#### `bulk_list`

| n | TPACK total | TPACK avg/msg | JSON total | CBOR total | vs JSON | vs CBOR |
|--:|------------:|--------------:|-----------:|-----------:|--------:|--------:|
| 1 | 2959 | 2959.0 | 7401 | 4802 | 0.40× | 0.62× |
| 10 | 29203 | 2920.3 | 74010 | 48020 | 0.39× | 0.61× |
| 100 | 291643 | 2916.4 | 740100 | 480200 | 0.39× | 0.61× |
| 1000 | 2916043 | 2916.0 | 7401000 | 4802000 | 0.39× | 0.61× |
| 10000 | 29160043 | 2916.0 | 74010000 | 48020000 | 0.39× | 0.61× |

### 1.5 Scaling: blob payload size (steady formats)

| blob_len | Format | Bytes | vs JSON | vs CBOR |
|---------:|--------|------:|--------:|--------:|
| 0 | `tpack-schemaref` | 24 | 0.92× | 1.33× |
| 0 | `json` | 26 | 1.00× | 1.44× |
| 0 | `cbor` | 18 | 0.69× | 1.00× |
| 0 | `msgpack` | 18 | 0.69× | 1.00× |
| 0 | `tpack-fullschema` | 33 | 1.27× | 1.83× |
| 0 | `tpack-fullschema-with-id` | 42 | 1.62× | 2.33× |
| 64 | `tpack-schemaref` | 88 | 0.35× | 0.62× |
| 64 | `json` | 251 | 1.00× | 1.78× |
| 64 | `cbor` | 141 | 0.56× | 1.00× |
| 64 | `msgpack` | 115 | 0.46× | 0.82× |
| 64 | `tpack-fullschema` | 97 | 0.39× | 0.69× |
| 64 | `tpack-fullschema-with-id` | 106 | 0.42× | 0.75× |
| 1024 | `tpack-schemaref` | 1049 | 0.28× | 0.53× |
| 1024 | `json` | 3681 | 1.00× | 1.87× |
| 1024 | `cbor` | 1972 | 0.54× | 1.00× |
| 1024 | `msgpack` | 1556 | 0.42× | 0.79× |
| 1024 | `tpack-fullschema` | 1058 | 0.29× | 0.54× |
| 1024 | `tpack-fullschema-with-id` | 1067 | 0.29× | 0.54× |
| 16384 | `tpack-schemaref` | 16410 | 0.28× | 0.53× |
| 16384 | `json` | 58521 | 1.00× | 1.87× |
| 16384 | `cbor` | 31252 | 0.53× | 1.00× |
| 16384 | `msgpack` | 24596 | 0.42× | 0.79× |
| 1048576 | `tpack-schemaref` | 1048602 | 0.28× | 0.52× |
| 1048576 | `json` | 3743769 | 1.00× | 1.87× |
| 1048576 | `cbor` | 1998870 | 0.53× | 1.00× |
| 1048576 | `msgpack` | 1572886 | 0.42× | 0.79× |

### 1.6 Scaling: list length (steady formats)

| list_len | Format | Bytes | vs JSON | vs CBOR |
|---------:|--------|------:|--------:|--------:|
| 1 | `tpack-schemaref` | 45 | 0.60× | 0.92× |
| 1 | `json` | 75 | 1.00× | 1.53× |
| 1 | `cbor` | 49 | 0.65× | 1.00× |
| 1 | `msgpack` | 49 | 0.65× | 1.00× |
| 1 | `tpack-fullschema` | 79 | 1.05× | 1.61× |
| 1 | `tpack-fullschema-with-id` | 88 | 1.17× | 1.80× |
| 10 | `tpack-schemaref` | 306 | 0.41× | 0.64× |
| 10 | `json` | 741 | 1.00× | 1.54× |
| 10 | `cbor` | 481 | 0.65× | 1.00× |
| 10 | `msgpack` | 481 | 0.65× | 1.00× |
| 10 | `tpack-fullschema` | 340 | 0.46× | 0.71× |
| 10 | `tpack-fullschema-with-id` | 349 | 0.47× | 0.73× |
| 100 | `tpack-schemaref` | 2916 | 0.39× | 0.61× |
| 100 | `json` | 7401 | 1.00× | 1.54× |
| 100 | `cbor` | 4802 | 0.65× | 1.00× |
| 100 | `msgpack` | 4803 | 0.65× | 1.00× |
| 100 | `tpack-fullschema` | 2950 | 0.40× | 0.61× |
| 100 | `tpack-fullschema-with-id` | 2959 | 0.40× | 0.62× |
| 1000 | `tpack-schemaref` | 29017 | 0.39× | 0.60× |
| 1000 | `json` | 74001 | 1.00× | 1.54× |
| 1000 | `cbor` | 48003 | 0.65× | 1.00× |
| 1000 | `msgpack` | 48003 | 0.65× | 1.00× |
| 10000 | `tpack-schemaref` | 299017 | 0.40× | 0.61× |
| 10000 | `json` | 749001 | 1.00× | 1.53× |
| 10000 | `cbor` | 489003 | 0.65× | 1.00× |
| 10000 | `msgpack` | 489003 | 0.65× | 1.00× |

### 1.7 Scaling: field count (all formats)

| fields | Format | Bytes | vs JSON | vs CBOR |
|-------:|--------|------:|--------:|--------:|
| 4 | `tpack-fullschema` | 65 | 2.83× | 4.33× |
| 4 | `tpack-fullschema-with-id` | 74 | 3.22× | 4.93× |
| 4 | `tpack-schemaref` | 47 | 2.04× | 3.13× |
| 4 | `json` | 23 | 1.00× | 1.53× |
| 4 | `cbor` | 15 | 0.65× | 1.00× |
| 4 | `msgpack` | 13 | 0.57× | 0.87× |
| 16 | `tpack-fullschema` | 239 | 3.46× | 5.97× |
| 16 | `tpack-fullschema-with-id` | 248 | 3.59× | 6.20× |
| 16 | `tpack-schemaref` | 143 | 2.07× | 3.58× |
| 16 | `json` | 69 | 1.00× | 1.73× |
| 16 | `cbor` | 40 | 0.58× | 1.00× |
| 16 | `msgpack` | 36 | 0.52× | 0.90× |
| 64 | `tpack-fullschema` | 960 | 3.61× | 5.19× |
| 64 | `tpack-fullschema-with-id` | 969 | 3.64× | 5.24× |
| 64 | `tpack-schemaref` | 527 | 1.98× | 2.85× |
| 64 | `json` | 266 | 1.00× | 1.44× |
| 64 | `cbor` | 185 | 0.70× | 1.00× |
| 64 | `msgpack` | 180 | 0.68× | 0.97× |
| 256 | `tpack-fullschema` | 4126 | 3.37× | 5.41× |
| 256 | `tpack-fullschema-with-id` | 4135 | 3.37× | 5.43× |
| 256 | `tpack-schemaref` | 2063 | 1.68× | 2.71× |
| 256 | `json` | 1226 | 1.00× | 1.61× |
| 256 | `cbor` | 762 | 0.62× | 1.00× |
| 256 | `msgpack` | 756 | 0.62× | 0.99× |

## 2. Host-dependent performance

**Host:** linux/x86_64, 32 logical CPUs, AMD Ryzen 9 7950X 16-Core Processor  
**Config:** warmup=500, samples=5000, threads=8, msgs/thread=10000  

These numbers are **not** portable. Re-run on your machine before optimizing.

### 2.1 PreparedSchema speedup (flat_record FullSchema)

iters=100000  

| Path | ns/op | vs naive |
|------|------:|---------:|
| naive `encode_message` FullSchema | 531.2 | 1.00× |
| prepared FullSchema + Encoder reuse | 216.8 | 2.45× |
| SchemaRef SteadyEncoder | 216.7 | 2.45× |

### 2.2 Latency tails (single-thread)

| Label | path | n | p50 ns | p90 ns | p99 ns | max ns | mean ns | B/op | MiB/s@p50 |
|-------|------|--:|------:|------:|------:|-------:|-------:|-----:|----------:|
| `flat_record/tpack-schemaref/encode/warm` | warm-prepared | 5000 | 221 | 241 | 260 | 3948 | 228 | 44 | 189.9 |
| `flat_record/tpack-schemaref/decode/warm` | warm-registry | 5000 | 290 | 301 | 391 | 3647 | 292 | 44 | 144.7 |
| `flat_record/tpack-schemaref/roundtrip/warm` | warm-prepared+registry | 5000 | 521 | 551 | 621 | 4007 | 526 | 44 | 80.5 |
| `flat_record/tpack-fullschema/encode/naive-cold-schema` | naive-reencode-schema | 5000 | 541 | 611 | 661 | 4578 | 549 | 76 | 134.0 |
| `flat_record/tpack-fullschema/encode/warm-prepared` | warm-prepared | 5000 | 221 | 241 | 251 | 3396 | 226 | 76 | 328.0 |
| `flat_record/json/encode` | oneshot | 5000 | 70 | 71 | 80 | 90 | 70 | 73 | 994.5 |
| `flat_record/json/decode` | oneshot | 5000 | 271 | 300 | 330 | 15939 | 278 | 73 | 256.9 |
| `flat_record/cbor/encode` | oneshot | 5000 | 141 | 180 | 231 | 4117 | 151 | 48 | 324.7 |
| `nested_struct/tpack-schemaref/encode/warm` | warm-prepared | 5000 | 340 | 351 | 361 | 3336 | 338 | 84 | 235.6 |
| `nested_struct/tpack-schemaref/decode/warm` | warm-registry | 5000 | 571 | 601 | 631 | 5179 | 575 | 84 | 140.3 |
| `nested_struct/tpack-schemaref/roundtrip/warm` | warm-prepared+registry | 5000 | 901 | 942 | 992 | 4418 | 906 | 84 | 88.9 |
| `nested_struct/tpack-fullschema/encode/naive-cold-schema` | naive-reencode-schema | 5000 | 761 | 841 | 902 | 5319 | 771 | 147 | 184.2 |
| `nested_struct/tpack-fullschema/encode/warm-prepared` | warm-prepared | 5000 | 331 | 360 | 371 | 3847 | 340 | 147 | 423.5 |
| `nested_struct/json/encode` | oneshot | 5000 | 171 | 191 | 211 | 5380 | 178 | 197 | 1098.7 |
| `nested_struct/json/decode` | oneshot | 5000 | 692 | 731 | 752 | 5069 | 699 | 197 | 271.5 |
| `nested_struct/cbor/encode` | oneshot | 5000 | 300 | 330 | 431 | 4959 | 305 | 150 | 476.8 |
| `bulk_list/tpack-schemaref/encode/warm` | warm-prepared | 5000 | 21088 | 21299 | 24966 | 34954 | 21183 | 2916 | 131.9 |
| `bulk_list/tpack-schemaref/decode/warm` | warm-registry | 5000 | 24595 | 24956 | 35685 | 41806 | 24877 | 2916 | 113.1 |
| `bulk_list/tpack-schemaref/roundtrip/warm` | warm-prepared+registry | 5000 | 46104 | 46605 | 51253 | 76039 | 46422 | 2916 | 60.3 |
| `bulk_list/tpack-fullschema/encode/naive-cold-schema` | naive-reencode-schema | 5000 | 21559 | 21810 | 25707 | 43018 | 21664 | 2950 | 130.5 |
| `bulk_list/tpack-fullschema/encode/warm-prepared` | warm-prepared | 5000 | 21179 | 21509 | 25246 | 38600 | 21305 | 2950 | 132.8 |
| `bulk_list/json/encode` | oneshot | 5000 | 5390 | 5430 | 6151 | 13124 | 5424 | 7401 | 1309.5 |
| `bulk_list/json/decode` | oneshot | 5000 | 27610 | 27931 | 41886 | 83913 | 27962 | 7401 | 255.6 |
| `bulk_list/cbor/encode` | oneshot | 5000 | 8105 | 8145 | 8806 | 27890 | 8148 | 4802 | 565.0 |
| `blob_0/tpack-schemaref/encode/warm` | warm-prepared | 5000 | 30 | 30 | 40 | 60 | 30 | 24 | 762.9 |
| `blob_64/tpack-schemaref/encode/warm` | warm-prepared | 5000 | 30 | 40 | 60 | 71 | 32 | 88 | 2797.4 |
| `blob_1024/tpack-schemaref/encode/warm` | warm-prepared | 5000 | 40 | 40 | 41 | 3697 | 40 | 1049 | 25010.1 |
| `blob_16384/tpack-schemaref/encode/warm` | warm-prepared | 5000 | 170 | 180 | 191 | 4508 | 171 | 16410 | 92057.6 |
| `blob_1048576/tpack-schemaref/encode/warm` | warm-prepared | 200 | 14937 | 15048 | 21289 | 26278 | 15244 | 1048602 | 66949.5 |

### 2.3 Concurrent SchemaRef decode

| Mode | threads | msgs/thread | total | wall | msgs/s | note |
|------|--------:|------------:|------:|-----:|-------:|------|
| shared-registry | 8 | 10000 | 80000 | 17.854ms | 4480872 | shared StdSchemaRegistry (RwLock) SchemaRef decode |
| per-thread-registry | 8 | 10000 | 80000 | 12.822ms | 6239455 | per-thread registry (no shared lock) |

### 2.4 Allocations

Allocation deltas are filled by `matrix_report` via `stats_alloc` (placeholder below is replaced).

| Label | path | allocs | deallocs | bytes_alloc | bytes_dealloc | realloc_bytes | out_B |
|-------|------|-------:|---------:|------------:|--------------:|--------------:|------:|
| `flat_record/tpack-schemaref/encode/warm` | warm-prepared | 10 | 9 | 189 | 125 | 56 | 44 |
| `flat_record/tpack-fullschema/encode/naive` | naive-reencode-schema | 13 | 13 | 579 | 579 | 142 | 76 |
| `flat_record/tpack-fullschema/encode/warm-prepared` | warm-prepared | 10 | 9 | 219 | 125 | 86 | 76 |
| `flat_record/tpack-schemaref/decode/warm` | warm-registry | 3 | 3 | 293 | 293 | 0 | 44 |
| `flat_record/json/encode` | oneshot | 1 | 1 | 128 | 128 | 0 | 73 |

Notes: **warm prepared encode still allocates today** (see numbers) — mostly value-path heap traffic (`BigInt` coefficients, temporary buffers), not schema re-encoding. Naive FullSchema pays additional schema serialization + a fresh output `Vec` every call. Decode allocates AST/values on every message. Zero-alloc hot paths are a future optimization target, not a current claim.

## 3. Regime conclusions (update after each matrix run)

Derived from the deterministic tables above; refine after host metrics:

1. **Tiny cold FullSchema loses to CBOR on purpose** (`flat_record` 76 vs 48): ~40B schema tax on ~29B data.
2. **Steady SchemaRef wins on structure** as nesting/list length grows (see list scale vs JSON/CBOR).
3. **Amortized 1 cold + (n−1) hot** approaches SchemaRef density as n increases; n=1 is the worst case for TPACK.
4. **Large blobs** converge across formats (payload dominates); optimize CPU only after confirming you are not bandwidth-bound.
5. **Never quote naive FullSchema encode as hot-path performance** — use PreparedSchema (section 2.1).
6. **Warm encode is not zero-alloc yet** (alloc table): value-path heap traffic remains; that is a primary optimization target before chasing varint micro-wins.
7. **JSON encode is faster on tiny messages** in host timings; TPACK's volume win is structural (size under nesting/amortization), not single-digit-ns flat encode.

## 4. Methodology notes

- Competitors use **named maps** (field names repeated). Semantic twins: DecimalFixed→i64 unscaled, Decimal→decimal string.
- Default decoder `validate_embedded_schema_on_cache_hit` remains enabled in production; benches label paths explicitly.
- Golden: `flat_record` FullSchema == **76** bytes (draft-00 vector).
- Criterion: `cargo bench -p tpack-bench` (not in default CI).
