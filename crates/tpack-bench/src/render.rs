//! Pure functions: [`Report`] → Markdown.
//!
//! No measuring. No I/O.

use crate::model::{Format, Report, ratio};

pub fn markdown(report: &Report) -> String {
    let mut md = String::new();
    preamble(&mut md);
    how_to_read(&mut md);
    size_sections(&mut md, report);
    if report.host.is_some() || !report.latency.is_empty() {
        host_sections(&mut md, report);
    }
    conclusions(&mut md);
    methodology(&mut md);
    md
}

fn preamble(md: &mut String) {
    md.push_str("# TPACK benchmark matrix\n\n");
    md.push_str(
        "Generated from structured samples (`tpack_bench::matrix::run` → `render::markdown`).\n\n",
    );
    md.push_str("```bash\n");
    md.push_str("cargo run -p tpack-bench --example size_report\n");
    md.push_str("cargo run -p tpack-bench --release --example matrix_report\n");
    md.push_str("```\n\n");
}

fn how_to_read(md: &mut String) {
    md.push_str("## How to read this\n\n");
    md.push_str("| Regime | What matters | Look at |\n");
    md.push_str("|--------|--------------|--------|\n");
    md.push_str("| Tiny self-contained message | Schema tax | Cold FullSchema sizes |\n");
    md.push_str("| Steady multi-message | No repeated keys | SchemaRef + amortization |\n");
    md.push_str("| Large payload | Bandwidth | Blob scale + MiB/s |\n");
    md.push_str("| Shared registry | Locking | Concurrent decode |\n\n");
    md.push_str("**Cold and warm paths are labeled separately.** Naive `encode_message` re-encodes schema on FullSchema; hot paths use `PreparedSchema`.\n\n");
}

fn size_sections(md: &mut String, report: &Report) {
    md.push_str("## 1. Deterministic sizes\n\n");

    md.push_str("### 1.1 Steady-state (SchemaRef primary)\n\n");
    size_table(
        md,
        &report.sizes,
        &["flat_record", "nested_struct", "list_100"],
        &Format::STEADY,
    );

    md.push_str("### 1.2 Cold FullSchema\n\n");
    size_table(
        md,
        &report.sizes,
        &["flat_record", "nested_struct", "list_100"],
        &Format::COLD_TPACK,
    );

    md.push_str("### 1.3 Component breakdown\n\n");
    md.push_str("| Case | Format | Total | Header | SchemaId | Schema | Data |\n");
    md.push_str("|------|--------|------:|-------:|---------:|-------:|-----:|\n");
    for b in &report.breakdowns {
        md.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} | {} | {} |\n",
            b.case,
            b.format.label(),
            b.total,
            b.header,
            b.schema_id,
            b.schema,
            b.data
        ));
    }
    md.push('\n');

    md.push_str("### 1.4 Amortization scan (1× WithId + (n−1)× SchemaRef)\n\n");
    // group by case
    let mut cases: Vec<&str> = report.amortized.iter().map(|a| a.case.as_str()).collect();
    cases.sort_unstable();
    cases.dedup();
    for case in cases {
        md.push_str(&format!("#### `{case}`\n\n"));
        md.push_str("| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |\n");
        md.push_str("|--:|------:|-----:|-----:|--------:|--------:|\n");
        for a in report.amortized.iter().filter(|a| a.case == case) {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                a.n,
                a.tpack_total,
                a.json_total,
                a.cbor_total,
                ratio(a.tpack_total, a.json_total),
                ratio(a.tpack_total, a.cbor_total),
            ));
        }
        md.push('\n');
    }

    md.push_str("### 1.5 Scale: blob payload\n\n");
    scale_table(md, &report.scale_blob);
    md.push_str("### 1.6 Scale: list length\n\n");
    scale_table(md, &report.scale_list);
    md.push_str("### 1.7 Scale: field count\n\n");
    scale_table(md, &report.scale_fields);
}

fn size_table(
    md: &mut String,
    samples: &[crate::model::SizeSample],
    cases: &[&str],
    formats: &[Format],
) {
    md.push_str("| Case | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|------|--------|------:|--------:|--------:|\n");
    for &case in cases {
        let json = find_bytes(samples, case, Format::Json);
        let cbor = find_bytes(samples, case, Format::Cbor);
        for &format in formats {
            if let Some(bytes) = try_bytes(samples, case, format) {
                md.push_str(&format!(
                    "| `{}` | `{}` | {} | {} | {} |\n",
                    case,
                    format.label(),
                    bytes,
                    ratio(bytes, json),
                    ratio(bytes, cbor),
                ));
            }
        }
    }
    md.push('\n');
}

fn scale_table(md: &mut String, samples: &[crate::model::SizeSample]) {
    md.push_str("| Case | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|------|--------|------:|--------:|--------:|\n");
    let mut cases: Vec<&str> = samples.iter().map(|s| s.case.as_str()).collect();
    cases.sort_unstable();
    cases.dedup();
    for case in cases {
        let json = find_bytes(samples, case, Format::Json);
        let cbor = find_bytes(samples, case, Format::Cbor);
        let mut rows: Vec<_> = samples.iter().filter(|s| s.case == case).collect();
        rows.sort_by_key(|s| s.format.label());
        for s in rows {
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                s.case,
                s.format.label(),
                s.bytes,
                ratio(s.bytes, json),
                ratio(s.bytes, cbor),
            ));
        }
    }
    md.push('\n');
}

fn find_bytes(samples: &[crate::model::SizeSample], case: &str, format: Format) -> usize {
    try_bytes(samples, case, format).unwrap_or(1)
}

fn try_bytes(samples: &[crate::model::SizeSample], case: &str, format: Format) -> Option<usize> {
    samples
        .iter()
        .find(|s| s.case == case && s.format == format)
        .map(|s| s.bytes)
}

fn host_sections(md: &mut String, report: &Report) {
    md.push_str("## 2. Host-dependent performance\n\n");
    if let Some(host) = &report.host {
        md.push_str(&format!("**Host:** {host}\n\n"));
    }
    md.push_str("Not portable. Re-run before optimizing.\n\n");

    if let Some(s) = &report.speedup {
        md.push_str("### 2.1 PreparedSchema speedup (`flat_record` FullSchema)\n\n");
        md.push_str("| Path | ns/op | vs naive |\n|------|------:|---------:|\n");
        md.push_str(&format!(
            "| naive `encode_message` | {:.1} | 1.00× |\n",
            s.naive_ns
        ));
        md.push_str(&format!(
            "| prepared + reuse | {:.1} | {:.2}× |\n",
            s.prepared_ns,
            s.naive_ns / s.prepared_ns
        ));
        md.push_str(&format!(
            "| SchemaRef warm | {:.1} | {:.2}× |\n\n",
            s.schemaref_ns,
            s.naive_ns / s.schemaref_ns
        ));
    }

    if !report.latency.is_empty() {
        md.push_str("### 2.2 Latency tails\n\n");
        md.push_str(
            "| Case | Format | Op | Path | n | p50 | p90 | p99 | max | mean | B/op | MiB/s@p50 |\n",
        );
        md.push_str(
            "|------|--------|----|------|--:|----:|----:|----:|----:|-----:|-----:|----------:|\n",
        );
        for r in &report.latency {
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} |\n",
                r.case,
                r.format.label(),
                r.op.label(),
                r.path.label(),
                r.stats.n,
                r.stats.p50,
                r.stats.p90,
                r.stats.p99,
                r.stats.max,
                r.stats.mean,
                r.bytes_per_op,
                r.mib_s_at_p50(),
            ));
        }
        md.push('\n');
    }

    if !report.concurrent.is_empty() {
        md.push_str("### 2.3 Concurrent SchemaRef decode\n\n");
        md.push_str("| Mode | threads | total | wall | msgs/s | note |\n");
        md.push_str("|------|--------:|------:|-----:|-------:|------|\n");
        for c in &report.concurrent {
            md.push_str(&format!(
                "| {} | {} | {} | {:.3?} | {:.0} | {} |\n",
                c.label, c.threads, c.total_messages, c.wall, c.msgs_per_s, c.note
            ));
        }
        md.push('\n');
    }

    if !report.allocs.is_empty() {
        md.push_str("### 2.4 Allocations\n\n");
        md.push_str("| Case | Format | Op | Path | allocs | bytes_alloc | out_B |\n");
        md.push_str("|------|--------|----|------|-------:|------------:|------:|\n");
        for a in &report.allocs {
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {} | {} |\n",
                a.case,
                a.format.label(),
                a.op.label(),
                a.path.label(),
                a.allocations,
                a.bytes_allocated,
                a.out_bytes
            ));
        }
        md.push_str("\nWarm prepared encode still allocates on the value path today (`BigInt`, temps). Zero-alloc is a future target, not a claim.\n\n");
    }
}

fn conclusions(md: &mut String) {
    md.push_str("## 3. Regime conclusions\n\n");
    md.push_str("1. Tiny cold FullSchema loses to CBOR on **schema tax** (structure, not a slow encoder).\n");
    md.push_str("2. SchemaRef / amortization wins as nesting, list length, or n grows.\n");
    md.push_str(
        "3. Prepared FullSchema ≫ naive `encode_message`; never quote naive as hot path.\n",
    );
    md.push_str("4. Large blobs: formats converge (payload dominates); check MiB/s before micro-optimizing CPU.\n");
    md.push_str("5. Warm encode is **not zero-alloc** yet — primary next optimization surface.\n");
    md.push_str("6. JSON can win tiny single-thread encode latency; TPACK’s case is structural density + multi-message.\n\n");
}

fn methodology(md: &mut String) {
    md.push_str("## 4. Methodology\n\n");
    md.push_str("- Architecture: `Workload` → `measure::*` → `Report` → `render::markdown`.\n");
    md.push_str("- Competitors: named-map semantic twins (DecimalFixed→i64, Decimal→string).\n");
    md.push_str("- Golden: `flat_record` FullSchema == **76** bytes (draft-00).\n");
    md.push_str("- Criterion optional: `cargo bench -p tpack-bench` (not default CI).\n");
}
