//! One-off timing: naive FullSchema encode vs PreparedSchema reuse.
use std::time::Instant;
use tpack::{Encoder, EnvelopeMode, PreparedSchema, encode_message};
use tpack_bench::{Scenario, SteadyEncoder, tpack_schema, tpack_value};

fn main() {
    let scenario = Scenario::FlatRecord;
    let schema = tpack_schema(scenario);
    let value = tpack_value(scenario);
    let n = 200_000u32;

    let t0 = Instant::now();
    for _ in 0..n {
        let _ = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).unwrap();
    }
    let naive = t0.elapsed();

    let prepared = PreparedSchema::prepare_default(schema.clone()).unwrap();
    let mut enc = Encoder::new();
    let t1 = Instant::now();
    for _ in 0..n {
        enc.clear();
        enc.encode_prepared_message(&prepared, &value, EnvelopeMode::FullSchema, None)
            .unwrap();
        std::hint::black_box(enc.as_slice());
    }
    let prepared_fs = t1.elapsed();

    let mut steady = SteadyEncoder::new(scenario, tpack_bench::Format::TpackSchemaRef).unwrap();
    let t2 = Instant::now();
    for _ in 0..n {
        std::hint::black_box(steady.encode_in_place().unwrap());
    }
    let schemaref = t2.elapsed();

    println!("iters={n}");
    println!(
        "naive FullSchema encode_message: {naive:?} ({:.1} ns/op)",
        naive.as_secs_f64() * 1e9 / n as f64
    );
    println!(
        "prepared FullSchema + reuse:     {prepared_fs:?} ({:.1} ns/op)",
        prepared_fs.as_secs_f64() * 1e9 / n as f64
    );
    println!(
        "SchemaRef SteadyEncoder:         {schemaref:?} ({:.1} ns/op)",
        schemaref.as_secs_f64() * 1e9 / n as f64
    );
    println!(
        "prepared speedup vs naive FullSchema: {:.2}x",
        naive.as_secs_f64() / prepared_fs.as_secs_f64()
    );
}
