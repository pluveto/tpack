//! Benchmark harness as small objects with one sample shape.
//!
//! ```text
//! Workload ──encode/decode──► Endpoint
//!                               │
//!                               ▼
//!                          Observation ──► Catalog ──► view::markdown
//!                               ▲
//!                          suite experiments
//! ```

mod endpoint;
mod obs;
mod suite;
mod view;
mod work;

pub use endpoint::{Endpoint, Error, SerdeFormat, TpackEnvelope, TpackStyle, tpack_parts};
pub use obs::{Catalog, Observation, Quantity, dims, ratio};
pub use suite::{Cfg, record_allocs, run};
pub use view::markdown;
pub use work::{
    AMORTIZED_NS, BLOB_SIZES, BULK, FIELD_COUNTS, LIST_LENS, Workload, blob, flat_record, list,
    narrative, nested, wide,
};

pub type MatrixCfg = Cfg;

pub fn run_matrix(cfg: Cfg) -> Result<Catalog, Error> {
    run(cfg)
}

pub fn fill_allocs(
    cat: &mut Catalog,
    work: &Workload,
    mut observe: impl FnMut(&str, &mut dyn FnMut()) -> (usize, usize),
) -> Result<(), Error> {
    record_allocs(cat, work, move |op| observe("op", op))
}

pub fn size_report_markdown() -> Result<String, Error> {
    Ok(markdown(&run(Cfg::size_only())?))
}

pub fn prepared_speedup(work: &Workload, iters: u32) -> Result<(f64, f64, f64), Error> {
    use std::time::Instant;
    use tpack::{Encoder, EnvelopeMode, PreparedSchema, encode_message};

    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = encode_message(work.schema(), work.value(), EnvelopeMode::FullSchema, None)?;
    }
    let naive = t0.elapsed().as_secs_f64();

    let prepared = PreparedSchema::prepare_default(work.schema().clone())?;
    let mut enc = Encoder::new();
    let t1 = Instant::now();
    for _ in 0..iters {
        enc.clear();
        enc.encode_prepared_message(&prepared, work.value(), EnvelopeMode::FullSchema, None)?;
        std::hint::black_box(enc.as_slice());
    }
    let prep = t1.elapsed().as_secs_f64();

    let mut warm = Endpoint::tpack_warm(work, TpackEnvelope::SchemaRef)?;
    let t2 = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(warm.encode()?);
    }
    let sref = t2.elapsed().as_secs_f64();
    let inv = 1e9 / f64::from(iters);
    Ok((naive * inv, prep * inv, sref * inv))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_76() {
        let mut ep = Endpoint::tpack_warm(&flat_record(), TpackEnvelope::FullSchema).unwrap();
        assert_eq!(ep.encode().unwrap().len(), 76);
    }

    #[test]
    fn parts_match_data() {
        let w = flat_record();
        let mut full = Endpoint::tpack_warm(&w, TpackEnvelope::FullSchema).unwrap();
        let mut sref = Endpoint::tpack_warm(&w, TpackEnvelope::SchemaRef).unwrap();
        let (_, _, schema, df) =
            tpack_parts(&full.encode().unwrap(), TpackEnvelope::FullSchema).unwrap();
        let (_, _, sr, dr) =
            tpack_parts(&sref.encode().unwrap(), TpackEnvelope::SchemaRef).unwrap();
        assert!(schema > 0 && sr == 0 && df == dr);
    }

    #[test]
    fn amortize_improves() {
        let cat = run(Cfg::size_only()).unwrap();
        let mut ratios = Vec::new();
        for o in cat.in_section("amortized") {
            if o.dim("case") != "flat_record" {
                continue;
            }
            if let Quantity::Stream { n, tpack, cbor, .. } = o.quantity {
                ratios.push((n, tpack as f64 / cbor as f64));
            }
        }
        ratios.sort_by_key(|(n, _)| *n);
        assert!(ratios.last().unwrap().1 < ratios.first().unwrap().1);
    }

    #[test]
    fn md_has_axes() {
        let md = size_report_markdown().unwrap();
        assert!(md.contains("Steady-state"));
        assert!(md.contains("blob_"));
        assert!(md.contains("list_"));
        assert!(md.contains("fields_"));
    }

    #[test]
    fn roundtrip() {
        let w = flat_record();
        for env in [
            TpackEnvelope::FullSchema,
            TpackEnvelope::FullSchemaWithId,
            TpackEnvelope::SchemaRef,
        ] {
            let mut ep = Endpoint::tpack_warm(&w, env).unwrap();
            let b = ep.encode().unwrap();
            ep.decode(&b).unwrap();
        }
    }

    #[test]
    fn quick_host_smoke() {
        let cat = run(Cfg::quick()).unwrap();
        assert!(cat.in_section("latency").count() > 3);
        assert_eq!(cat.in_section("concurrent").count(), 2);
    }
}
