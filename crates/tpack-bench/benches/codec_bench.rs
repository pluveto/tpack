//! Criterion microbenches — same WarmTpack path as the matrix.
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use tpack_bench::{ExecPath, Format, WarmTpack, encode, flat_record, narrative};

fn hot_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("hot");
    for work in narrative() {
        let mut warm = WarmTpack::new(&work, Format::TpackSchemaRef).expect("warm");
        let bytes = warm.encode_vec().expect("e");
        group.bench_function(format!("{}/schemaref/encode", work.name), |b| {
            b.iter(|| black_box(warm.encode().expect("e").len()));
        });
        group.bench_function(format!("{}/schemaref/decode", work.name), |b| {
            b.iter(|| {
                warm.decode(black_box(bytes.as_slice())).expect("d");
            });
        });
        group.bench_function(format!("{}/json/encode", work.name), |b| {
            b.iter(|| {
                black_box(
                    encode(&work, Format::Json, ExecPath::NaiveMessage)
                        .expect("j")
                        .len(),
                );
            });
        });
    }

    let flat = flat_record();
    group.bench_function("flat/fullschema/naive", |b| {
        b.iter(|| {
            black_box(
                encode(&flat, Format::TpackFullSchema, ExecPath::NaiveMessage)
                    .expect("n")
                    .len(),
            );
        });
    });
    let mut prep = WarmTpack::new(&flat, Format::TpackFullSchema).expect("p");
    group.bench_function("flat/fullschema/warm", |b| {
        b.iter(|| black_box(prep.encode().expect("e").len()));
    });
    group.finish();
}

criterion_group!(benches, hot_paths);
criterion_main!(benches);
