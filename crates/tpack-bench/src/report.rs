//! Markdown report generation for deterministic + host-labeled sections.

use crate::codec_ops::EncodeError;
use crate::format::Format;
use crate::payload::{AMORTIZED_NS, Scenario};
use crate::perf::{
    HostInfo, PerfConfig, measure_concurrent_schemaref_decode,
    measure_concurrent_schemaref_decode_sharded, measure_latency_matrix, measure_prepared_speedup,
};
use crate::size::{
    breakdown_tpack, measure_all_sizes, measure_amortized, measure_amortized_scan,
    measure_blob_scale, measure_field_scale, measure_list_scale,
};

fn ratio(num: usize, den: usize) -> String {
    if den == 0 {
        return "n/a".into();
    }
    format!("{:.2}×", num as f64 / den as f64)
}

fn sample_bytes(
    samples: &[(Scenario, Format, usize)],
    scenario: Scenario,
    format: Format,
) -> usize {
    samples
        .iter()
        .find(|(s, f, _)| *s == scenario && *f == format)
        .map(|(_, _, b)| *b)
        .unwrap_or(0)
}

/// Options for building the full markdown document.
#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub perf: PerfConfig,
    /// When false, omit host-dependent latency/concurrent sections (size-only).
    pub include_host_metrics: bool,
    pub prepared_speedup_iters: u32,
}

impl Default for ReportOptions {
    fn default() -> Self {
        Self {
            perf: PerfConfig::report(),
            include_host_metrics: true,
            prepared_speedup_iters: 100_000,
        }
    }
}

impl ReportOptions {
    pub fn size_only() -> Self {
        Self {
            include_host_metrics: false,
            ..Self::default()
        }
    }

    pub fn quick() -> Self {
        Self {
            perf: PerfConfig::quick(),
            include_host_metrics: true,
            prepared_speedup_iters: 20_000,
        }
    }
}

/// Build the full `docs/benchmarks.md` body.
pub fn render_full_report(opts: &ReportOptions) -> Result<String, EncodeError> {
    let sizes = measure_all_sizes()?;
    let size_tuples: Vec<_> = sizes
        .iter()
        .map(|s| (s.scenario, s.format, s.bytes))
        .collect();

    let mut md = String::new();
    md.push_str("# TPACK benchmark matrix\n\n");
    md.push_str("This document is **generated**. Do not hand-edit numbers.\n\n");
    md.push_str("```bash\n");
    md.push_str("# Deterministic sizes only (fast, CI-safe)\n");
    md.push_str("cargo run -p tpack-bench --example size_report\n\n");
    md.push_str("# Full matrix: sizes + latency tails + allocs + concurrency (host-labeled)\n");
    md.push_str("cargo run -p tpack-bench --release --example matrix_report\n");
    md.push_str("```\n\n");

    md.push_str("## How to read this\n\n");
    md.push_str("TPACK is not one number. Regime matters:\n\n");
    md.push_str("| Regime | What matters | Primary metric |\n");
    md.push_str("|--------|--------------|----------------|\n");
    md.push_str(
        "| Tiny single message, self-contained | Schema tax | FullSchema size vs CBOR map |\n",
    );
    md.push_str("| Steady multi-message | No repeated keys | SchemaRef size + warm CPU |\n");
    md.push_str("| Large payload | Bandwidth / memcpy | MiB/s on blob scale |\n");
    md.push_str("| Shared registry, multi-thread | Locking | Concurrent decode msgs/s |\n\n");
    md.push_str("**Cold vs warm must never be mixed.** Naive `encode_message` re-encodes the schema every FullSchema call; hot paths must use `PreparedSchema` + a reused `Encoder`.\n\n");

    // --- Deterministic sizes ---
    md.push_str("## 1. Deterministic size matrix\n\n");
    md.push_str("These sizes are machine-independent (same on every host).\n\n");

    md.push_str("### 1.1 Steady-state size (SchemaRef primary)\n\n");
    md.push_str("| Scenario | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|----------|--------|------:|--------:|--------:|\n");
    for scenario in Scenario::ALL {
        let json_b = sample_bytes(&size_tuples, scenario, Format::Json);
        let cbor_b = sample_bytes(&size_tuples, scenario, Format::Cbor);
        for format in Format::STEADY {
            let bytes = sample_bytes(&size_tuples, scenario, format);
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                scenario.name(),
                format.label(),
                bytes,
                ratio(bytes, json_b),
                ratio(bytes, cbor_b),
            ));
        }
    }
    md.push('\n');

    md.push_str("### 1.2 Cold / self-contained size (FullSchema)\n\n");
    md.push_str("Includes embedded schema every message. Expected to lose to CBOR on **tiny** single records.\n\n");
    md.push_str("| Scenario | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|----------|--------|------:|--------:|--------:|\n");
    for scenario in Scenario::ALL {
        let json_b = sample_bytes(&size_tuples, scenario, Format::Json);
        let cbor_b = sample_bytes(&size_tuples, scenario, Format::Cbor);
        for format in Format::COLD_TPACK {
            let bytes = sample_bytes(&size_tuples, scenario, format);
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                scenario.name(),
                format.label(),
                bytes,
                ratio(bytes, json_b),
                ratio(bytes, cbor_b),
            ));
        }
    }
    md.push('\n');

    md.push_str("### 1.3 TPACK component breakdown\n\n");
    md.push_str("| Scenario | Mode | Total | Hdr+ver+mode | SchemaId | Schema | Data |\n");
    md.push_str("|----------|------|------:|-------------:|---------:|-------:|-----:|\n");
    for scenario in Scenario::ALL {
        for format in [
            Format::TpackFullSchema,
            Format::TpackFullSchemaWithId,
            Format::TpackSchemaRef,
        ] {
            let b = breakdown_tpack(scenario, format)?;
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {} | {} |\n",
                scenario.name(),
                format.label(),
                b.total,
                b.header_and_mode,
                b.schema_id,
                b.schema,
                b.data,
            ));
        }
    }
    md.push_str(
        "\nLength prefixes for id/schema are included in total but not separate columns.\n\n",
    );

    md.push_str("### 1.4 Amortized multi-message scan\n\n");
    md.push_str("Stream model: **1× FullSchemaWithId** (cold bind) + **(n−1)× SchemaRef** (hot). Competitors pay full size every message.\n\n");
    for scenario in Scenario::ALL {
        md.push_str(&format!("#### `{}`\n\n", scenario.name()));
        md.push_str(
            "| n | TPACK total | TPACK avg/msg | JSON total | CBOR total | vs JSON | vs CBOR |\n",
        );
        md.push_str(
            "|--:|------------:|--------------:|-----------:|-----------:|--------:|--------:|\n",
        );
        for (tpack, json, cbor) in measure_amortized_scan(scenario)? {
            md.push_str(&format!(
                "| {} | {} | {:.1} | {} | {} | {} | {} |\n",
                tpack.n,
                tpack.total_bytes,
                tpack.per_message,
                json.total_bytes,
                cbor.total_bytes,
                ratio(tpack.total_bytes, json.total_bytes),
                ratio(tpack.total_bytes, cbor.total_bytes),
            ));
        }
        md.push('\n');
    }

    md.push_str("### 1.5 Scaling: blob payload size (steady formats)\n\n");
    md.push_str("| blob_len | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|---------:|--------|------:|--------:|--------:|\n");
    let blobs = measure_blob_scale()?;
    // Group by param
    let mut params: Vec<usize> = blobs.iter().map(|s| s.param).collect();
    params.sort_unstable();
    params.dedup();
    for p in params {
        let json_b = blobs
            .iter()
            .find(|s| s.param == p && s.format == Format::Json)
            .map(|s| s.bytes)
            .unwrap_or(1);
        let cbor_b = blobs
            .iter()
            .find(|s| s.param == p && s.format == Format::Cbor)
            .map(|s| s.bytes)
            .unwrap_or(1);
        for s in blobs.iter().filter(|s| s.param == p) {
            md.push_str(&format!(
                "| {} | `{}` | {} | {} | {} |\n",
                p,
                s.format.label(),
                s.bytes,
                ratio(s.bytes, json_b),
                ratio(s.bytes, cbor_b),
            ));
        }
    }
    md.push('\n');

    md.push_str("### 1.6 Scaling: list length (steady formats)\n\n");
    md.push_str("| list_len | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|---------:|--------|------:|--------:|--------:|\n");
    let lists = measure_list_scale()?;
    let mut params: Vec<usize> = lists.iter().map(|s| s.param).collect();
    params.sort_unstable();
    params.dedup();
    for p in params {
        let json_b = lists
            .iter()
            .find(|s| s.param == p && s.format == Format::Json)
            .map(|s| s.bytes)
            .unwrap_or(1);
        let cbor_b = lists
            .iter()
            .find(|s| s.param == p && s.format == Format::Cbor)
            .map(|s| s.bytes)
            .unwrap_or(1);
        for s in lists.iter().filter(|s| s.param == p) {
            md.push_str(&format!(
                "| {} | `{}` | {} | {} | {} |\n",
                p,
                s.format.label(),
                s.bytes,
                ratio(s.bytes, json_b),
                ratio(s.bytes, cbor_b),
            ));
        }
    }
    md.push('\n');

    md.push_str("### 1.7 Scaling: field count (all formats)\n\n");
    md.push_str("| fields | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|-------:|--------|------:|--------:|--------:|\n");
    let fields = measure_field_scale()?;
    let mut params: Vec<usize> = fields.iter().map(|s| s.param).collect();
    params.sort_unstable();
    params.dedup();
    for p in params {
        let json_b = fields
            .iter()
            .find(|s| s.param == p && s.format == Format::Json)
            .map(|s| s.bytes)
            .unwrap_or(1);
        let cbor_b = fields
            .iter()
            .find(|s| s.param == p && s.format == Format::Cbor)
            .map(|s| s.bytes)
            .unwrap_or(1);
        for s in fields.iter().filter(|s| s.param == p) {
            md.push_str(&format!(
                "| {} | `{}` | {} | {} | {} |\n",
                p,
                s.format.label(),
                s.bytes,
                ratio(s.bytes, json_b),
                ratio(s.bytes, cbor_b),
            ));
        }
    }
    md.push('\n');

    // Highlight amortized n=100 for narrative continuity
    let _ = (measure_amortized(Scenario::BulkList, 100)?, AMORTIZED_NS);

    if opts.include_host_metrics {
        let host = HostInfo::detect();
        md.push_str("## 2. Host-dependent performance\n\n");
        md.push_str(&format!("**Host:** {}  \n", host.summary));
        md.push_str(&format!(
            "**Config:** warmup={}, samples={}, threads={}, msgs/thread={}  \n\n",
            opts.perf.warmup,
            opts.perf.samples,
            opts.perf.concurrent_threads,
            opts.perf.concurrent_messages_per_thread
        ));
        md.push_str(
            "These numbers are **not** portable. Re-run on your machine before optimizing.\n\n",
        );

        // Prepared speedup
        let (naive_ns, prep_ns, sref_ns) = measure_prepared_speedup(opts.prepared_speedup_iters)?;
        md.push_str("### 2.1 PreparedSchema speedup (flat_record FullSchema)\n\n");
        md.push_str(&format!(
            "iters={}  \n\n| Path | ns/op | vs naive |\n|------|------:|---------:|\n",
            opts.prepared_speedup_iters
        ));
        md.push_str(&format!(
            "| naive `encode_message` FullSchema | {:.1} | 1.00× |\n",
            naive_ns
        ));
        md.push_str(&format!(
            "| prepared FullSchema + Encoder reuse | {:.1} | {:.2}× |\n",
            prep_ns,
            naive_ns / prep_ns
        ));
        md.push_str(&format!(
            "| SchemaRef SteadyEncoder | {:.1} | {:.2}× |\n\n",
            sref_ns,
            naive_ns / sref_ns
        ));

        // Latency matrix
        md.push_str("### 2.2 Latency tails (single-thread)\n\n");
        md.push_str("| Label | path | n | p50 ns | p90 ns | p99 ns | max ns | mean ns | B/op | MiB/s@p50 |\n");
        md.push_str(
            "|-------|------|--:|------:|------:|------:|-------:|-------:|-----:|----------:|\n",
        );
        let latency = measure_latency_matrix(&opts.perf)?;
        for row in &latency {
            md.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} |\n",
                row.label,
                row.path,
                row.stats.samples,
                row.stats.p50_ns,
                row.stats.p90_ns,
                row.stats.p99_ns,
                row.stats.max_ns,
                row.stats.mean_ns,
                row.bytes_per_op,
                row.mib_per_s_at_p50,
            ));
        }
        md.push('\n');

        // Concurrent
        md.push_str("### 2.3 Concurrent SchemaRef decode\n\n");
        let shared = measure_concurrent_schemaref_decode(&opts.perf)?;
        let sharded = measure_concurrent_schemaref_decode_sharded(&opts.perf)?;
        md.push_str("| Mode | threads | msgs/thread | total | wall | msgs/s | note |\n");
        md.push_str("|------|--------:|------------:|------:|-----:|-------:|------|\n");
        for (name, row) in [
            ("shared-registry", &shared),
            ("per-thread-registry", &sharded),
        ] {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {:.3?} | {:.0} | {} |\n",
                name,
                row.threads,
                row.messages_per_thread,
                row.total_messages,
                row.wall,
                row.msgs_per_s,
                row.note,
            ));
        }
        md.push('\n');

        md.push_str("### 2.4 Allocations\n\n");
        md.push_str(
            "Allocation deltas are filled by `matrix_report` via `stats_alloc` (placeholder below is replaced).\n\n",
        );
        md.push_str("<!-- ALLOC_TABLE -->\n\n");
    }

    md.push_str("## 3. Regime conclusions (update after each matrix run)\n\n");
    md.push_str("Derived from the deterministic tables above; refine after host metrics:\n\n");
    md.push_str("1. **Tiny cold FullSchema loses to CBOR on purpose** (`flat_record` 76 vs 48): ~40B schema tax on ~29B data.\n");
    md.push_str("2. **Steady SchemaRef wins on structure** as nesting/list length grows (see list scale vs JSON/CBOR).\n");
    md.push_str("3. **Amortized 1 cold + (n−1) hot** approaches SchemaRef density as n increases; n=1 is the worst case for TPACK.\n");
    md.push_str("4. **Large blobs** converge across formats (payload dominates); optimize CPU only after confirming you are not bandwidth-bound.\n");
    md.push_str("5. **Never quote naive FullSchema encode as hot-path performance** — use PreparedSchema (section 2.1).\n");
    md.push_str("6. **Warm encode is not zero-alloc yet** (alloc table): value-path heap traffic remains; that is a primary optimization target before chasing varint micro-wins.\n");
    md.push_str("7. **JSON encode is faster on tiny messages** in host timings; TPACK's volume win is structural (size under nesting/amortization), not single-digit-ns flat encode.\n\n");

    md.push_str("## 4. Methodology notes\n\n");
    md.push_str("- Competitors use **named maps** (field names repeated). Semantic twins: DecimalFixed→i64 unscaled, Decimal→decimal string.\n");
    md.push_str("- Default decoder `validate_embedded_schema_on_cache_hit` remains enabled in production; benches label paths explicitly.\n");
    md.push_str("- Golden: `flat_record` FullSchema == **76** bytes (draft-00 vector).\n");
    md.push_str("- Criterion: `cargo bench -p tpack-bench` (not in default CI).\n");

    Ok(md)
}

/// Size-only markdown (fast).
pub fn render_size_report() -> Result<String, EncodeError> {
    render_full_report(&ReportOptions::size_only())
}
