use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use tpack_bench::{Endpoint, SerdeFormat, TpackEnvelope, flat_record, narrative};

fn hot(c: &mut Criterion) {
    let mut g = c.benchmark_group("hot");
    for work in narrative() {
        let mut ep = Endpoint::tpack_warm(&work, TpackEnvelope::SchemaRef).unwrap();
        let bytes = ep.encode().unwrap();
        g.bench_function(format!("{}/schemaref/encode", work.name()), |b| {
            b.iter(|| black_box(ep.encode().unwrap().len()));
        });
        g.bench_function(format!("{}/schemaref/decode", work.name()), |b| {
            b.iter(|| ep.decode(black_box(bytes.as_slice())).unwrap());
        });
        let mut json = Endpoint::serde(&work, SerdeFormat::Json);
        g.bench_function(format!("{}/json/encode", work.name()), |b| {
            b.iter(|| black_box(json.encode().unwrap().len()));
        });
    }
    let flat = flat_record();
    let mut naive = Endpoint::tpack_naive(&flat, TpackEnvelope::FullSchema);
    g.bench_function("flat/fullschema/naive", |b| {
        b.iter(|| black_box(naive.encode().unwrap().len()));
    });
    let mut warm = Endpoint::tpack_warm(&flat, TpackEnvelope::FullSchema).unwrap();
    g.bench_function("flat/fullschema/warm", |b| {
        b.iter(|| black_box(warm.encode().unwrap().len()));
    });
    g.finish();
}

criterion_group!(benches, hot);
criterion_main!(benches);
