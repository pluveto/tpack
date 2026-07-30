//! Criterion encode/decode throughput benches for TPACK and competitor formats.
//!
//! ```bash
//! cargo bench -p tpack-bench
//! cargo bench -p tpack-bench -- --quick
//! ```

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use tpack::{Decoder, EnvelopeMode, encode_message};
use tpack_bench::{
    Format, Scenario, encode, preload_registry, schema_id_bytes, serde_payload, tpack_schema,
    tpack_value,
};

fn encode_benches(c: &mut Criterion) {
    for scenario in Scenario::ALL {
        let mut group = c.benchmark_group(format!("encode/{}", scenario.name()));
        // Approximate input size with JSON twin for throughput reporting.
        let json_len = encode(scenario, Format::Json)
            .map(|b| b.len() as u64)
            .unwrap_or(64);
        group.throughput(Throughput::Bytes(json_len));

        for format in Format::ALL {
            let id = format.label();
            group.bench_function(id, |b| {
                b.iter(|| {
                    let bytes = encode(scenario, format).expect("encode");
                    black_box(bytes);
                });
            });
        }
        group.finish();
    }
}

fn decode_benches(c: &mut Criterion) {
    for scenario in Scenario::ALL {
        let mut group = c.benchmark_group(format!("decode/{}", scenario.name()));

        // Pre-encode all formats once; SchemaRef registry is preloaded outside the loop.
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

/// Focused micro-bench that reuses the same schema/value without re-building payloads.
fn tpack_hot_path(c: &mut Criterion) {
    let scenario = Scenario::FlatRecord;
    let schema = tpack_schema(scenario);
    let value = tpack_value(scenario);
    let id = schema_id_bytes(scenario);
    let full = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).expect("encode");
    let sref =
        encode_message(&schema, &value, EnvelopeMode::SchemaRef, Some(&id)).expect("encode ref");
    let (registry, _, _) = preload_registry(scenario);
    let json = serde_json::to_vec(&serde_payload(scenario)).expect("json");

    let mut group = c.benchmark_group("hot/flat_record");
    group.throughput(Throughput::Bytes(json.len() as u64));

    group.bench_function("tpack_fullschema_encode", |b| {
        b.iter(|| {
            black_box(
                encode_message(
                    black_box(&schema),
                    black_box(&value),
                    EnvelopeMode::FullSchema,
                    None,
                )
                .expect("encode"),
            );
        });
    });

    group.bench_function("tpack_schemaref_encode", |b| {
        b.iter(|| {
            black_box(
                encode_message(
                    black_box(&schema),
                    black_box(&value),
                    EnvelopeMode::SchemaRef,
                    Some(black_box(&id)),
                )
                .expect("encode"),
            );
        });
    });

    group.bench_function("tpack_fullschema_decode", |b| {
        b.iter(|| {
            let mut decoder = Decoder::new(black_box(full.as_slice()));
            black_box(decoder.decode_message().expect("decode"));
        });
    });

    group.bench_function("tpack_schemaref_decode", |b| {
        b.iter(|| {
            let mut decoder = Decoder::new(black_box(sref.as_slice()));
            black_box(
                decoder
                    .decode_message_with_registry(&registry)
                    .expect("decode"),
            );
        });
    });

    group.bench_function("json_encode", |b| {
        let payload = serde_payload(scenario);
        b.iter(|| {
            black_box(serde_json::to_vec(black_box(&payload)).expect("json"));
        });
    });

    group.bench_function("json_decode", |b| {
        b.iter(|| {
            let v: tpack_bench::SerdePayload =
                serde_json::from_slice(black_box(json.as_slice())).expect("json");
            black_box(v);
        });
    });

    group.finish();
}

criterion_group!(benches, encode_benches, decode_benches, tpack_hot_path);
criterion_main!(benches);
