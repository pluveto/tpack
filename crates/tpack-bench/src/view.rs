//! Pure projection: Catalog → Markdown.
//! The view never measures; it only groups observations.

use crate::obs::{Catalog, Quantity, mib_s, ratio};
// Quantity used throughout match arms

pub fn markdown(cat: &Catalog) -> String {
    let mut md = String::new();
    md.push_str("# TPACK benchmark matrix\n\n");
    md.push_str("```text\nWorkload → Endpoint → Observation → Catalog → view::markdown\n```\n\n");
    md.push_str("```bash\ncargo run -p tpack-bench --example size_report\n");
    md.push_str("cargo run -p tpack-bench --release --example matrix_report\n```\n\n");

    md.push_str("## How to read\n\n");
    md.push_str("| Regime | Look at |\n|--------|--------|\n");
    md.push_str("| Tiny self-contained | §1.2 cold FullSchema |\n");
    md.push_str("| Steady multi-message | §1.1 SchemaRef + §1.4 amortization |\n");
    md.push_str("| Large payload | §1.5 blob + latency MiB/s |\n");
    md.push_str("| Shared registry | §2.3 concurrent |\n\n");
    md.push_str("Warm vs naive paths are **different endpoints**, never mixed.\n\n");

    section_size_table(&mut md, "1.1 Steady-state size", cat, "size.steady");
    section_size_table(&mut md, "1.2 Cold FullSchema size", cat, "size.cold");
    section_parts(&mut md, cat);
    section_amortized(&mut md, cat);
    section_size_table(&mut md, "1.5 Scale: blob", cat, "scale.blob");
    section_size_table(&mut md, "1.6 Scale: list", cat, "scale.list");
    section_size_table(&mut md, "1.7 Scale: fields", cat, "scale.fields");

    if cat.host.is_some() || cat.in_section("latency").next().is_some() {
        md.push_str("## 2. Host-dependent\n\n");
        if let Some(h) = &cat.host {
            md.push_str(&format!("**Host:** {h}\n\nNot portable.\n\n"));
        }
        section_speedup(&mut md, cat);
        section_latency(&mut md, cat);
        section_concurrent(&mut md, cat);
        section_alloc(&mut md, cat);
    }

    md.push_str("## 3. Regime conclusions\n\n");
    md.push_str("1. Tiny cold FullSchema loses to CBOR on **schema tax**.\n");
    md.push_str("2. SchemaRef / amortization win as structure or n grows.\n");
    md.push_str("3. Prepared FullSchema ≫ naive `encode_message`.\n");
    md.push_str("4. Large blobs converge; check MiB/s before CPU micro-opts.\n");
    md.push_str("5. Warm encode is **not zero-alloc** yet (value path / `BigInt`).\n");
    md.push_str("6. JSON can win tiny encode latency; TPACK’s case is structural density.\n\n");

    md.push_str("## 4. Methodology\n\n");
    md.push_str("- Objects: `Workload`, `Endpoint`, `Observation`, `Catalog`.\n");
    md.push_str("- Suite = list of experiments; view = pure projection.\n");
    md.push_str("- Competitors: named-map twins. Golden: `flat_record` FullSchema = **76**.\n");
    md
}

fn section_size_table(md: &mut String, title: &str, cat: &Catalog, section: &'static str) {
    let rows: Vec<_> = cat.in_section(section).collect();
    if rows.is_empty() {
        return;
    }
    md.push_str(&format!("## {title}\n\n"));
    md.push_str("| Case | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|------|--------|------:|--------:|--------:|\n");
    let mut cases: Vec<&str> = rows.iter().map(|o| o.dim("case")).collect();
    cases.sort_unstable();
    cases.dedup();
    for case in cases {
        let json = bytes_for(&rows, case, "json");
        let cbor = bytes_for(&rows, case, "cbor");
        let mut fmts: Vec<_> = rows.iter().filter(|o| o.dim("case") == case).collect();
        fmts.sort_by_key(|o| o.dim("format"));
        for o in fmts {
            if let Quantity::Bytes(n) = o.quantity {
                md.push_str(&format!(
                    "| `{}` | `{}` | {} | {} | {} |\n",
                    case,
                    o.dim("format"),
                    n,
                    ratio(n, json),
                    ratio(n, cbor)
                ));
            }
        }
    }
    md.push('\n');
}

fn bytes_for(rows: &[&crate::obs::Observation], case: &str, format: &str) -> usize {
    rows.iter()
        .find(|o| o.dim("case") == case && o.dim("format") == format)
        .and_then(|o| match o.quantity {
            Quantity::Bytes(n) => Some(n),
            _ => None,
        })
        .unwrap_or(1)
}

fn section_parts(md: &mut String, cat: &Catalog) {
    let rows: Vec<_> = cat.in_section("size.parts").collect();
    if rows.is_empty() {
        return;
    }
    md.push_str("## 1.3 Component breakdown\n\n");
    md.push_str("| Case | Format | Total | Header | SchemaId | Schema | Data |\n");
    md.push_str("|------|--------|------:|-------:|---------:|-------:|-----:|\n");
    for o in rows {
        if let Quantity::Parts {
            total,
            header,
            schema_id,
            schema,
            data,
        } = o.quantity
        {
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {} | {} |\n",
                o.dim("case"),
                o.dim("format"),
                total,
                header,
                schema_id,
                schema,
                data
            ));
        }
    }
    md.push('\n');
}

fn section_amortized(md: &mut String, cat: &Catalog) {
    let rows: Vec<_> = cat.in_section("amortized").collect();
    if rows.is_empty() {
        return;
    }
    md.push_str("## 1.4 Amortization (1× WithId + (n−1)× SchemaRef)\n\n");
    let mut cases: Vec<&str> = rows.iter().map(|o| o.dim("case")).collect();
    cases.sort_unstable();
    cases.dedup();
    for case in cases {
        md.push_str(&format!("### `{case}`\n\n"));
        md.push_str("| n | TPACK | JSON | CBOR | vs JSON | vs CBOR |\n");
        md.push_str("|--:|------:|-----:|-----:|--------:|--------:|\n");
        let mut group: Vec<_> = rows.iter().filter(|o| o.dim("case") == case).collect();
        group.sort_by_key(|o| match o.quantity {
            Quantity::Stream { n, .. } => n,
            _ => 0,
        });
        for o in group {
            if let Quantity::Stream {
                n,
                tpack,
                json,
                cbor,
            } = o.quantity
            {
                md.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    n,
                    tpack,
                    json,
                    cbor,
                    ratio(tpack, json),
                    ratio(tpack, cbor)
                ));
            }
        }
        md.push('\n');
    }
}

fn section_speedup(md: &mut String, cat: &Catalog) {
    for o in cat.in_section("speedup") {
        if let Quantity::Speedup {
            naive_ns,
            prepared_ns,
            schemaref_ns,
        } = o.quantity
        {
            md.push_str("### 2.1 PreparedSchema speedup\n\n");
            md.push_str("| Path | ns/op | vs naive |\n|------|------:|---------:|\n");
            md.push_str(&format!("| naive FullSchema | {:.1} | 1.00× |\n", naive_ns));
            md.push_str(&format!(
                "| prepared + reuse | {:.1} | {:.2}× |\n",
                prepared_ns,
                naive_ns / prepared_ns
            ));
            md.push_str(&format!(
                "| SchemaRef warm | {:.1} | {:.2}× |\n\n",
                schemaref_ns,
                naive_ns / schemaref_ns
            ));
        }
    }
}

fn section_latency(md: &mut String, cat: &Catalog) {
    let rows: Vec<_> = cat.in_section("latency").collect();
    if rows.is_empty() {
        return;
    }
    md.push_str("### 2.2 Latency tails\n\n");
    md.push_str(
        "| Case | Format | Op | Path | n | p50 | p90 | p99 | max | mean | B/op | MiB/s@p50 |\n",
    );
    md.push_str(
        "|------|--------|----|------|--:|----:|----:|----:|----:|-----:|-----:|----------:|\n",
    );
    for o in rows {
        if let Quantity::Latency {
            n,
            p50,
            p90,
            p99,
            max,
            mean,
            bytes_per_op,
        } = o.quantity
        {
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.1} |\n",
                o.dim("case"),
                o.dim("format"),
                o.dim("op"),
                o.dim("path"),
                n,
                p50,
                p90,
                p99,
                max,
                mean,
                bytes_per_op,
                mib_s(bytes_per_op, p50)
            ));
        }
    }
    md.push('\n');
}

fn section_concurrent(md: &mut String, cat: &Catalog) {
    let rows: Vec<_> = cat.in_section("concurrent").collect();
    if rows.is_empty() {
        return;
    }
    md.push_str("### 2.3 Concurrent SchemaRef decode\n\n");
    md.push_str("| Mode | threads | total | wall | msgs/s |\n");
    md.push_str("|------|--------:|------:|-----:|-------:|\n");
    for o in rows {
        if let Quantity::Concurrent {
            threads,
            total_messages,
            wall,
            msgs_per_s,
        } = o.quantity
        {
            md.push_str(&format!(
                "| {} | {} | {} | {:.3?} | {:.0} |\n",
                o.dim("mode"),
                threads,
                total_messages,
                wall,
                msgs_per_s
            ));
        }
    }
    md.push('\n');
}

fn section_alloc(md: &mut String, cat: &Catalog) {
    let rows: Vec<_> = cat.in_section("alloc").collect();
    if rows.is_empty() {
        return;
    }
    md.push_str("### 2.4 Allocations\n\n");
    md.push_str("| Case | Format | Op | Path | allocs | bytes | out_B |\n");
    md.push_str("|------|--------|----|------|-------:|------:|------:|\n");
    for o in rows {
        if let Quantity::Alloc {
            allocations,
            bytes_allocated,
            out_bytes,
        } = o.quantity
        {
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {} | {} |\n",
                o.dim("case"),
                o.dim("format"),
                o.dim("op"),
                o.dim("path"),
                allocations,
                bytes_allocated,
                out_bytes
            ));
        }
    }
    md.push_str(
        "\nWarm prepared still allocates on the value path; zero-alloc is a future target.\n\n",
    );
}
