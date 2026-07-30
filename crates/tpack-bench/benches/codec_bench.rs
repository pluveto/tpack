//! Criterion encode/decode throughput benches.
//!
//! TPACK encode paths use [`tpack_bench::SteadyEncoder`] (PreparedSchema +
//! buffer reuse). Competitors allocate per iteration as their one-shot APIs do.
//!
//! ```bash
//! cargo bench -p tpack-bench
//! cargo bench -p tpack-bench -- --quick
//! ```

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use tpack::Decoder;
use tpack_bench::{Format, Scenario, SteadyEncoder, encode, preload_registry, serde_payload};

fn encode_benches(c: &mut Criterion) {
    for scenario in Scenario::ALL {
        let mut group = c.benchmark_group(format!("encode/{}", scenario.name()));
        let json_len = encode(scenario, Format::Json)
            .map(|b| b.len() as u64)
            .unwrap_or(64);
        group.throughput(Throughput::Bytes(json_len));

        for format in Format::ALL {
            let id = format.label();
            if format.is_tpack() {
                let mut steady = SteadyEncoder::new(scenario, format).expect("steady");
                group.bench_function(id, |b| {
                    b.iter(|| {
                        let bytes = steady.encode_in_place().expect("encode");
                        black_box(bytes);
                    });
                });
            } else {
                group.bench_function(id, |b| {
                    b.iter(|| {
                        let bytes = encode(scenario, format).expect("encode");
                        black_box(bytes);
                    });
                });
            }
        }
        group.finish();
    }
}

fn decode_benches(c: &mut Criterion) {
    for scenario in Scenario::ALL {
        let mut group = c.benchmark_group(format!("decode/{}", scenario.name()));

        let encoded: Vec<(Format, Vec<u8>)> = Format::ALL
            .iter()
            .map(|&format| {
                let bytes = encode(scenario, format).expect("pre-encode");
                (format, bytes)
            })
            .collect();

        let sample_len = encoded
            .iter()
            .find(|(f, _)| *f == Format::Json)
            .map(|(_, b)| b.len() as u64)
            .unwrap_or(64);
        group.throughput(Throughput::Bytes(sample_len));

        let (registry, _schema, _id) = preload_registry(scenario);

        for (format, bytes) in &encoded {
            let label = format.label();
            match format {
                Format::TpackSchemaRef => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let mut decoder = Decoder::new(black_box(bytes.as_slice()));
                            let msg = decoder
                                .decode_message_with_registry(&registry)
                                .expect("decode SchemaRef");
                            black_box(msg);
                        });
                    });
                }
                Format::TpackFullSchema | Format::TpackFullSchemaWithId => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let mut decoder = Decoder::new(black_box(bytes.as_slice()));
                            let msg = decoder.decode_message().expect("decode tpack");
                            black_box(msg);
                        });
                    });
                }
                Format::Json => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let v: tpack_bench::SerdePayload =
                                serde_json::from_slice(black_box(bytes.as_slice()))
                                    .expect("decode json");
                            black_box(v);
                        });
                    });
                }
                Format::Cbor => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let v: tpack_bench::SerdePayload =
                                ciborium::from_reader(black_box(bytes.as_slice()))
                                    .expect("decode cbor");
                            black_box(v);
                        });
                    });
                }
                Format::MsgPack => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let v: tpack_bench::SerdePayload =
                                rmp_serde::from_slice(black_box(bytes.as_slice()))
                                    .expect("decode msgpack");
                            black_box(v);
                        });
                    });
                }
            }
        }
        group.finish();
    }
}

fn hot_flat_record(c: &mut Criterion) {
    // Microbench: SchemaRef encode/decode with full steady-state setup.
    let scenario = Scenario::FlatRecord;
    let mut group = c.benchmark_group("hot/flat_record");
    let mut steady = SteadyEncoder::new(scenario, Format::TpackSchemaRef).expect("steady");
    let bytes = steady.encode_once().expect("encode");
    let (registry, _, _) = preload_registry(scenario);

    group.bench_function("tpack-schemaref/encode", |b| {
        b.iter(|| {
            let out = steady.encode_in_place().expect("encode");
            black_box(out);
        });
    });
    group.bench_function("tpack-schemaref/decode", |b| {
        b.iter(|| {
            let mut decoder = Decoder::new(black_box(bytes.as_slice()));
            let msg = decoder
                .decode_message_with_registry(&registry)
                .expect("decode");
            black_box(msg);
        });
    });
    group.bench_function("json/encode", |b| {
        let payload = serde_payload(scenario);
        b.iter(|| {
            let out = serde_json::to_vec(black_box(&payload)).expect("json");
            black_box(out);
        });
    });
    group.finish();
}

criterion_group!(benches, encode_benches, decode_benches, hot_flat_record);
criterion_main!(benches);
