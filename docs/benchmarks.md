# TPACK benchmark matrix

```text
Workload → Endpoint → Observation → Catalog → view::markdown
```

```bash
cargo run -p tpack-bench --example size_report
cargo run -p tpack-bench --release --example matrix_report
```

## How to read

| Regime | Look at |
|--------|--------|
| Tiny self-contained | §1.2 cold FullSchema |
| Steady multi-message | §1.1 SchemaRef + §1.4 amortization |
| Large payload | §1.5 blob + latency MiB/s |
| Shared registry | §2.3 concurrent |

Warm vs naive paths are **different endpoints**, never mixed.

## 1.1 Steady-state size

| Case | Format | Bytes | vs JSON | vs CBOR |
|------|--------|------:|--------:|--------:|
| `flat_record` | `cbor` | 48 | 0.66× | 1.00× |
| `flat_record` | `json` | 73 | 1.00× | 1.52× |
| `flat_record` | `msgpack` | 48 | 0.66× | 1.00× |
| `flat_record` | `tpack-schemaref` | 44 | 0.60× | 0.92× |
| `list_100` | `cbor` | 4802 | 0.65× | 1.00× |
| `list_100` | `json` | 7401 | 1.00× | 1.54× |
| `list_100` | `msgpack` | 4803 | 0.65× | 1.00× |
| `list_100` | `tpack-schemaref` | 2916 | 0.39× | 0.61× |
| `nested_struct` | `cbor` | 150 | 0.76× | 1.00× |
| `nested_struct` | `json` | 197 | 1.00× | 1.31× |
| `nested_struct` | `msgpack` | 150 | 0.76× | 1.00× |
| `nested_struct` | `tpack-schemaref` | 84 | 0.43× | 0.56× |

## 1.2 Cold FullSchema size

| Case | Format | Bytes | vs JSON | vs CBOR |
|------|--------|------:|--------:|--------:|
| `flat_record` | `tpack-fullschema` | 76 | 76.00× | 76.00× |
| `flat_record` | `tpack-fullschema-with-id` | 85 | 85.00× | 85.00× |
| `list_100` | `tpack-fullschema` | 2950 | 2950.00× | 2950.00× |
| `list_100` | `tpack-fullschema-with-id` | 2959 | 2959.00× | 2959.00× |
| `nested_struct` | `tpack-fullschema` | 147 | 147.00× | 147.00× |
| `nested_struct` | `tpack-fullschema-with-id` | 156 | 156.00× | 156.00× |

## 1.3 Component breakdown

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

## 1.4 Amortization (1× WithId + (n−1)× SchemaRef)

### `flat_record`

| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |
|--:|------:|-----:|-----:|--------:|--------:|
| 1 | 85 | 73 | 48 | 1.16× | 1.77× |
| 10 | 481 | 730 | 480 | 0.66× | 1.00× |
| 100 | 4441 | 7300 | 4800 | 0.61× | 0.93× |
| 1000 | 44041 | 73000 | 48000 | 0.60× | 0.92× |
| 10000 | 440041 | 730000 | 480000 | 0.60× | 0.92× |

### `list_100`

| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |
|--:|------:|-----:|-----:|--------:|--------:|
| 1 | 2959 | 7401 | 4802 | 0.40× | 0.62× |
| 10 | 29203 | 74010 | 48020 | 0.39× | 0.61× |
| 100 | 291643 | 740100 | 480200 | 0.39× | 0.61× |
| 1000 | 2916043 | 7401000 | 4802000 | 0.39× | 0.61× |
| 10000 | 29160043 | 74010000 | 48020000 | 0.39× | 0.61× |

### `nested_struct`

| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |
|--:|------:|-----:|-----:|--------:|--------:|
| 1 | 156 | 197 | 150 | 0.79× | 1.04× |
| 10 | 912 | 1970 | 1500 | 0.46× | 0.61× |
| 100 | 8472 | 19700 | 15000 | 0.43× | 0.56× |
| 1000 | 84072 | 197000 | 150000 | 0.43× | 0.56× |
| 10000 | 840072 | 1970000 | 1500000 | 0.43× | 0.56× |

## 1.5 Scale: blob

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

## 1.6 Scale: list

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

## 1.7 Scale: fields

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

## 2. Host-dependent

**Host:** linux/x86_64, 32 CPUs, AMD Ryzen 9 7950X 16-Core Processor

Not portable.

### 2.1 PreparedSchema speedup

| Path | ns/op | vs naive |
|------|------:|---------:|
| naive FullSchema | 515.8 | 1.00× |
| prepared + reuse | 217.8 | 2.37× |
| SchemaRef warm | 238.4 | 2.16× |

### 2.2 Latency tails

| Case | Format | Op | Path | n | p50 | p90 | p99 | max | mean | B/op | MiB/s@p50 |
|------|--------|----|------|--:|----:|----:|----:|----:|-----:|-----:|----------:|
| `flat_record` | `tpack-schemaref` | encode | warm-prepared | 5000 | 231 | 241 | 260 | 5259 | 236 | 44 | 181.7 |
| `flat_record` | `tpack-schemaref` | decode | warm-prepared | 5000 | 310 | 321 | 331 | 18655 | 314 | 44 | 135.4 |
| `flat_record` | `tpack-schemaref` | roundtrip | warm-prepared | 5000 | 511 | 521 | 671 | 4719 | 518 | 44 | 82.1 |
| `flat_record` | `tpack-fullschema` | encode | naive-message | 5000 | 511 | 551 | 661 | 5731 | 521 | 76 | 141.8 |
| `flat_record` | `tpack-fullschema` | encode | warm-prepared | 5000 | 231 | 250 | 280 | 4308 | 235 | 76 | 313.8 |
| `flat_record` | `json` | encode | oneshot | 5000 | 90 | 91 | 150 | 4329 | 91 | 73 | 773.5 |
| `flat_record` | `json` | decode | oneshot | 5000 | 300 | 311 | 361 | 3858 | 297 | 73 | 232.1 |
| `flat_record` | `cbor` | encode | oneshot | 5000 | 161 | 171 | 201 | 4829 | 166 | 48 | 284.3 |
| `nested_struct` | `tpack-schemaref` | encode | warm-prepared | 5000 | 331 | 341 | 361 | 13595 | 337 | 84 | 242.0 |
| `nested_struct` | `tpack-schemaref` | decode | warm-prepared | 5000 | 541 | 571 | 661 | 7123 | 550 | 84 | 148.1 |
| `nested_struct` | `tpack-schemaref` | roundtrip | warm-prepared | 5000 | 872 | 1072 | 1392 | 5019 | 916 | 84 | 91.9 |
| `nested_struct` | `tpack-fullschema` | encode | naive-message | 5000 | 732 | 782 | 871 | 15288 | 746 | 147 | 191.5 |
| `nested_struct` | `tpack-fullschema` | encode | warm-prepared | 5000 | 331 | 341 | 541 | 3957 | 339 | 147 | 423.5 |
| `nested_struct` | `json` | encode | oneshot | 5000 | 211 | 231 | 270 | 941 | 216 | 197 | 890.4 |
| `nested_struct` | `cbor` | encode | oneshot | 5000 | 341 | 361 | 380 | 3646 | 346 | 150 | 419.5 |
| `list_100` | `tpack-schemaref` | encode | warm-prepared | 5000 | 31578 | 32040 | 44372 | 52998 | 31900 | 2916 | 88.1 |
| `list_100` | `tpack-schemaref` | decode | warm-prepared | 5000 | 24415 | 25036 | 35797 | 68327 | 24794 | 2916 | 113.9 |
| `list_100` | `tpack-schemaref` | roundtrip | warm-prepared | 5000 | 45795 | 47308 | 67345 | 121986 | 46600 | 2916 | 60.7 |
| `list_100` | `tpack-fullschema` | encode | naive-message | 5000 | 22171 | 22823 | 32079 | 49802 | 22423 | 2950 | 126.9 |
| `list_100` | `tpack-fullschema` | encode | warm-prepared | 5000 | 21770 | 22181 | 32099 | 55513 | 22048 | 2950 | 129.2 |
| `list_100` | `json` | encode | oneshot | 5000 | 7003 | 7103 | 11561 | 23904 | 7105 | 7401 | 1007.9 |
| `list_100` | `cbor` | encode | oneshot | 5000 | 11541 | 12052 | 15208 | 28974 | 11254 | 4802 | 396.8 |
| `blob_0` | `tpack-schemaref` | encode | warm-prepared | 5000 | 40 | 40 | 50 | 90 | 40 | 24 | 572.2 |
| `blob_64` | `tpack-schemaref` | encode | warm-prepared | 5000 | 40 | 50 | 51 | 3686 | 43 | 88 | 2098.1 |
| `blob_1024` | `tpack-schemaref` | encode | warm-prepared | 5000 | 90 | 91 | 101 | 111 | 84 | 1049 | 11115.6 |
| `blob_16384` | `tpack-schemaref` | encode | warm-prepared | 5000 | 411 | 431 | 581 | 5019 | 419 | 16410 | 38077.4 |
| `blob_1048576` | `tpack-schemaref` | encode | warm-prepared | 200 | 30316 | 30607 | 35295 | 37580 | 30533 | 1048602 | 32986.7 |

### 2.3 Concurrent SchemaRef decode

| Mode | threads | total | wall | msgs/s |
|------|--------:|------:|-----:|-------:|
| shared-registry | 8 | 80000 | 15.027ms | 5323635 |
| per-thread-registry | 8 | 80000 | 13.897ms | 5756589 |

### 2.4 Allocations

| Case | Format | Op | Path | allocs | bytes | out_B |
|------|--------|----|------|-------:|------:|------:|
| `flat_record` | `tpack-schemaref` | encode | warm-prepared | 10 | 169 | 44 |
| `flat_record` | `tpack-fullschema` | encode | naive-message | 13 | 579 | 76 |
| `flat_record` | `tpack-fullschema` | encode | warm-prepared | 10 | 201 | 76 |
| `flat_record` | `tpack-schemaref` | decode | warm-prepared | 3 | 293 | 44 |
| `flat_record` | `json` | encode | oneshot | 1 | 128 | 73 |

Warm prepared still allocates on the value path; zero-alloc is a future target.

## 3. Regime conclusions

1. Tiny cold FullSchema loses to CBOR on **schema tax**.
2. SchemaRef / amortization win as structure or n grows.
3. Prepared FullSchema ≫ naive `encode_message`.
4. Large blobs converge; check MiB/s before CPU micro-opts.
5. Warm encode is **not zero-alloc** yet (value path / `BigInt`).
6. JSON can win tiny encode latency; TPACK’s case is structural density.

## 4. Methodology

- Objects: `Workload`, `Endpoint`, `Observation`, `Catalog`.
- Suite = list of experiments; view = pure projection.
- Competitors: named-map twins. Golden: `flat_record` FullSchema = **76**.
