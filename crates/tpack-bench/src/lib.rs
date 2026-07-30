//! TPACK size and performance benchmark harness (not published).
//!
//! # What this crate measures
//!
//! | Class | Portable? | Entry point |
//! |-------|-----------|-------------|
//! | Wire sizes, breakdowns, scaling axes, amortization | Yes | [`render_size_report`] / `size_report` example |
//! | Latency p50/p99, concurrent decode, prepared speedup | No (host-labeled) | `matrix_report` example |
//! | Heap allocation deltas | No | `matrix_report` via `stats_alloc` |
//! | Criterion microbenches | No | `cargo bench -p tpack-bench` |
//!
//! Competitor formats encode *semantic twins* (serde maps with repeated keys),
//! not TPACK's native type system.

mod codec_ops;
mod format;
mod payload;
mod perf;
mod report;
mod size;

pub use codec_ops::{
    EncodeError, SteadyEncoder, decode, decode_tpack_with_registry, encode, encode_blob,
    encode_list, encode_tpack_naive, encode_wide, preload_registry, preload_registry_from_schema,
};
pub use format::Format;
pub use payload::{
    AMORTIZED_NS, BLOB_SIZES, BULK_LIST_LEN, BlobSerde, FIELD_COUNTS, FlatRecordSerde,
    LIST_LENGTHS, LineItemSerde, NestedOrderSerde, Scenario, SerdePayload, WideSerde, blob_schema,
    blob_serde, blob_value, flat_record_schema, flat_record_value, list_schema, list_serde,
    list_value, schema_id_bytes, schema_id_for, serde_payload, tpack_schema, tpack_value,
    wide_schema, wide_serde, wide_value,
};
pub use perf::{
    AllocDelta, AllocRow, ConcurrentRow, HostInfo, LatencyRow, LatencyStats, PerfConfig,
    measure_alloc_rows, measure_concurrent_schemaref_decode,
    measure_concurrent_schemaref_decode_sharded, measure_latency_matrix, measure_prepared_speedup,
};
pub use report::{ReportOptions, render_full_report, render_size_report};
pub use size::{
    AmortizedSample, ScaleSizeSample, SizeSample, TpackBreakdown, breakdown_tpack,
    measure_all_sizes, measure_amortized, measure_amortized_competitor, measure_amortized_scan,
    measure_blob_scale, measure_field_scale, measure_list_scale,
};

#[cfg(test)]
mod tests {
    use super::*;

    const FLAT_RECORD_FULLSCHEMA_BYTES: usize = 76;

    #[test]
    fn flat_record_fullschema_matches_golden_size() {
        let bytes = encode(Scenario::FlatRecord, Format::TpackFullSchema).expect("encode");
        assert_eq!(bytes.len(), FLAT_RECORD_FULLSCHEMA_BYTES);
    }

    #[test]
    fn breakdown_data_matches_across_tpack_modes() {
        let full = breakdown_tpack(Scenario::FlatRecord, Format::TpackFullSchema).unwrap();
        let sref = breakdown_tpack(Scenario::FlatRecord, Format::TpackSchemaRef).unwrap();
        assert_eq!(full.data, sref.data);
        assert!(full.schema > 0);
        assert_eq!(sref.schema, 0);
        assert!(sref.schema_id > 0);
    }

    #[test]
    fn amortized_scan_improves_relative_to_cbor_as_n_grows() {
        let scan = measure_amortized_scan(Scenario::FlatRecord).unwrap();
        let first = &scan[0];
        let last = scan.last().unwrap();
        // n=1 TPACK is cold FullSchemaWithId only; vs CBOR should be worse than large n.
        let first_vs = first.0.total_bytes as f64 / first.2.total_bytes as f64;
        let last_vs = last.0.total_bytes as f64 / last.2.total_bytes as f64;
        assert!(
            last_vs < first_vs,
            "amortization should improve vs CBOR: n=1 {first_vs} vs n={} {last_vs}",
            last.0.n
        );
    }

    #[test]
    fn list_scale_schemaref_beats_json_at_100() {
        let samples = measure_list_scale().unwrap();
        let sref = samples
            .iter()
            .find(|s| s.param == 100 && s.format == Format::TpackSchemaRef)
            .unwrap();
        let json = samples
            .iter()
            .find(|s| s.param == 100 && s.format == Format::Json)
            .unwrap();
        assert!(sref.bytes < json.bytes);
    }

    #[test]
    fn field_scale_fullschema_grows_with_names() {
        let samples = measure_field_scale().unwrap();
        let a = samples
            .iter()
            .find(|s| s.param == 4 && s.format == Format::TpackFullSchema)
            .unwrap()
            .bytes;
        let b = samples
            .iter()
            .find(|s| s.param == 64 && s.format == Format::TpackFullSchema)
            .unwrap()
            .bytes;
        assert!(b > a);
    }

    #[test]
    fn steady_encoder_matches_oneshot() {
        let oneshot = encode(Scenario::FlatRecord, Format::TpackSchemaRef).unwrap();
        let mut steady =
            SteadyEncoder::for_scenario(Scenario::FlatRecord, Format::TpackSchemaRef).unwrap();
        let a = steady.encode_once().unwrap();
        let b = steady.encode_once().unwrap();
        assert_eq!(oneshot, a);
        assert_eq!(a, b);
    }

    #[test]
    fn sizes_deterministic() {
        let a = measure_all_sizes().unwrap();
        let b = measure_all_sizes().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn size_report_contains_scaling_sections() {
        let md = render_size_report().unwrap();
        assert!(md.contains("Steady-state size"));
        assert!(md.contains("Amortized multi-message"));
        assert!(md.contains("blob payload"));
        assert!(md.contains("list length"));
        assert!(md.contains("field count"));
        assert!(md.contains("Regime conclusions"));
    }

    #[test]
    fn latency_quick_smoke() {
        // Ensures perf paths compile and run without asserting absolute times.
        let rows = measure_latency_matrix(&PerfConfig::quick()).unwrap();
        assert!(rows.len() > 5);
        for row in &rows {
            assert!(row.stats.p50_ns > 0);
            assert!(row.stats.p99_ns >= row.stats.p50_ns);
        }
    }

    #[test]
    fn concurrent_smoke() {
        let cfg = PerfConfig {
            concurrent_threads: 2,
            concurrent_messages_per_thread: 50,
            ..PerfConfig::quick()
        };
        let shared = measure_concurrent_schemaref_decode(&cfg).unwrap();
        let sharded = measure_concurrent_schemaref_decode_sharded(&cfg).unwrap();
        assert!(shared.msgs_per_s > 0.0);
        assert!(sharded.msgs_per_s > 0.0);
    }

    #[test]
    fn roundtrip_all_formats_flat_record() {
        for format in Format::ALL {
            let bytes = encode(Scenario::FlatRecord, format).expect("encode");
            decode(Scenario::FlatRecord, format, &bytes).unwrap();
        }
    }

    #[test]
    fn percentile_monotonic() {
        let stats = LatencyStats::from_samples(vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        assert!(stats.p50_ns <= stats.p90_ns);
        assert!(stats.p90_ns <= stats.p99_ns);
        assert!(stats.p99_ns <= stats.max_ns);
    }
}
