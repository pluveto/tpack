//! Criterion microbenches (host-dependent; not in default CI).
//!
//! Uses steady-state TPACK APIs (`SteadyEncoder`).

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
                let mut steady = SteadyEncoder::for_scenario(scenario, format).expect("steady");
                group.bench_function(id, |b| {
                    b.iter(|| {
                        let len = steady.encode_in_place().expect("encode").len();
                        black_box(len);
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
            .map(|&format| (format, encode(scenario, format).expect("pre-encode")))
            .collect();
        let sample_len = encoded
            .iter()
            .find(|(f, _)| *f == Format::Json)
            .map(|(_, b)| b.len() as u64)
            .unwrap_or(64);
        group.throughput(Throughput::Bytes(sample_len));
        let (registry, _, _) = preload_registry(scenario);

        for (format, bytes) in &encoded {
            let label = format.label();
            match format {
                Format::TpackSchemaRef => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let mut decoder = Decoder::new(black_box(bytes.as_slice()));
                            black_box(
                                decoder
                                    .decode_message_with_registry(&registry)
                                    .expect("decode"),
                            );
                        });
                    });
                }
                Format::TpackFullSchema | Format::TpackFullSchemaWithId => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let mut decoder = Decoder::new(black_box(bytes.as_slice()));
                            black_box(decoder.decode_message().expect("decode"));
                        });
                    });
                }
                Format::Json => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let v: tpack_bench::SerdePayload =
                                serde_json::from_slice(black_box(bytes.as_slice())).expect("json");
                            black_box(v);
                        });
                    });
                }
                Format::Cbor => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let v: tpack_bench::SerdePayload =
                                ciborium::from_reader(black_box(bytes.as_slice())).expect("cbor");
                            black_box(v);
                        });
                    });
                }
                Format::MsgPack => {
                    group.bench_function(label, |b| {
                        b.iter(|| {
                            let v: tpack_bench::SerdePayload =
                                rmp_serde::from_slice(black_box(bytes.as_slice())).expect("mp");
                            black_box(v);
                        });
                    });
                }
            }
        }
        group.finish();
    }
}

fn hot_and_naive(c: &mut Criterion) {
    let scenario = Scenario::FlatRecord;
    let mut group = c.benchmark_group("paths/flat_record");

    let mut steady = SteadyEncoder::for_scenario(scenario, Format::TpackSchemaRef).expect("s");
    let bytes = steady.encode_once().expect("e");
    let (registry, _, _) = preload_registry(scenario);

    group.bench_function("schemaref/encode/warm", |b| {
        b.iter(|| black_box(steady.encode_in_place().expect("e").len()));
    });
    group.bench_function("schemaref/decode/warm", |b| {
        b.iter(|| {
            let mut d = Decoder::new(black_box(bytes.as_slice()));
            black_box(d.decode_message_with_registry(&registry).expect("d"));
        });
    });

    let schema = tpack_bench::tpack_schema(scenario);
    let value = tpack_bench::tpack_value(scenario);
    group.bench_function("fullschema/encode/naive", |b| {
        b.iter(|| {
            black_box(
                tpack::encode_message(&schema, &value, tpack::EnvelopeMode::FullSchema, None)
                    .expect("e")
                    .len(),
            );
        });
    });
    let mut prepared = SteadyEncoder::for_scenario(scenario, Format::TpackFullSchema).expect("p");
    group.bench_function("fullschema/encode/warm-prepared", |b| {
        b.iter(|| black_box(prepared.encode_in_place().expect("e").len()));
    });

    let payload = serde_payload(scenario);
    group.bench_function("json/encode", |b| {
        b.iter(|| black_box(serde_json::to_vec(black_box(&payload)).expect("j")));
    });

    group.finish();
}

criterion_group!(benches, encode_benches, decode_benches, hot_and_naive);
criterion_main!(benches);
