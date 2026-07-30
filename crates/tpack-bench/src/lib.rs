//! TPACK benchmark harness — structured experiments, not a script pile.
//!
//! ```text
//! Workload  →  measure::*  →  Report  →  render::markdown
//!                ↑
//!            matrix::run (declares what to measure)
//! ```
//!
//! - **Portable:** sizes, breakdowns, scaling, amortization
//! - **Host-labeled:** latency tails, allocs, concurrency, prepared speedup

mod codec;
mod matrix;
mod measure;
mod model;
mod render;
mod workload;

pub use codec::{Error, WarmTpack, decode, encode};
pub use matrix::{MatrixCfg, fill_allocs, run as run_matrix};
pub use measure::{LatencyCfg, amortized, breakdown, latency, prepared_speedup, size_of, sizes};
pub use model::{
    AllocSample, AmortizedSample, Breakdown, ConcurrentSample, ExecPath, Format, LatencySample,
    LatencyStats, Op, Report, SizeSample, SpeedupSample, ratio,
};
pub use render::markdown;
pub use workload::{
    AMORTIZED_NS, BLOB_SIZES, BULK_LEN, FIELD_COUNTS, LIST_LENS, Twin, Workload, blob, bulk_list,
    flat_record, list_of, narrative, nested_struct, wide,
};

/// Size-only report markdown (no host metrics).
pub fn size_report_markdown() -> Result<String, Error> {
    let report = run_matrix(MatrixCfg::size_only())?;
    Ok(markdown(&report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_flat_fullschema_76() {
        let w = flat_record();
        let s = size_of(&w, Format::TpackFullSchema).unwrap();
        assert_eq!(s.bytes, 76);
    }

    #[test]
    fn breakdown_splits_schema_and_data() {
        let w = flat_record();
        let b = breakdown(&w, Format::TpackFullSchema).unwrap();
        let r = breakdown(&w, Format::TpackSchemaRef).unwrap();
        assert_eq!(b.data, r.data);
        assert!(b.schema > 0);
        assert_eq!(r.schema, 0);
    }

    #[test]
    fn amortization_improves_vs_cbor() {
        let w = flat_record();
        let a1 = amortized(&w, 1).unwrap();
        let a_big = amortized(&w, 10_000).unwrap();
        let r1 = a1.tpack_total as f64 / a1.cbor_total as f64;
        let r_big = a_big.tpack_total as f64 / a_big.cbor_total as f64;
        assert!(r_big < r1);
    }

    #[test]
    fn warm_encode_stable() {
        let w = flat_record();
        let a = encode(&w, Format::TpackSchemaRef, ExecPath::WarmPrepared).unwrap();
        let b = encode(&w, Format::TpackSchemaRef, ExecPath::WarmPrepared).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn size_matrix_deterministic() {
        let a = run_matrix(MatrixCfg::size_only()).unwrap();
        let b = run_matrix(MatrixCfg::size_only()).unwrap();
        assert_eq!(a.sizes, b.sizes);
        assert_eq!(a.scale_list.len(), b.scale_list.len());
    }

    #[test]
    fn markdown_has_axes() {
        let md = size_report_markdown().unwrap();
        assert!(md.contains("Steady-state"));
        assert!(md.contains("Amortization"));
        assert!(md.contains("blob_"));
        assert!(md.contains("list_"));
        assert!(md.contains("fields_"));
    }

    #[test]
    fn latency_smoke() {
        let w = flat_record();
        let row = latency(
            &w,
            Format::TpackSchemaRef,
            Op::Encode,
            ExecPath::WarmPrepared,
            LatencyCfg::quick(),
        )
        .unwrap();
        assert!(row.stats.p50 > 0);
        assert!(row.stats.p99 >= row.stats.p50);
    }

    #[test]
    fn roundtrip_formats() {
        let w = flat_record();
        for f in Format::ALL {
            let bytes = encode(&w, f, ExecPath::WarmPrepared).unwrap();
            decode(&w, f, &bytes).unwrap();
        }
    }

    #[test]
    fn list_scale_name() {
        assert_eq!(list_of(100).name, "list_100");
    }
}
