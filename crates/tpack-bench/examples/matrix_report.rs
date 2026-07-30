//! Full benchmark matrix: sizes + host-labeled latency/alloc/concurrency.
//!
//! ```bash
//! cargo run -p tpack-bench --release --example matrix_report
//! cargo run -p tpack-bench --release --example matrix_report -- --quick
//! ```

use std::{alloc::System, env, fs, path::PathBuf, process};

use stats_alloc::{Region, StatsAlloc};
use tpack_bench::{AllocDelta, ReportOptions, measure_alloc_rows, render_full_report};

#[global_allocator]
static GLOBAL: StatsAlloc<System> = StatsAlloc::system();

fn main() {
    if let Err(err) = run() {
        eprintln!("matrix_report: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let quick = env::args().any(|a| a == "--quick");
    let opts = if quick {
        ReportOptions::quick()
    } else {
        ReportOptions::default()
    };

    eprintln!(
        "matrix_report: building full report (quick={quick}, samples={})…",
        opts.perf.samples
    );
    let mut markdown = render_full_report(&opts)?;

    // Fill allocation table under the stats_alloc global.
    let alloc_rows = measure_alloc_rows(|_label, op| {
        let region = Region::new(&GLOBAL);
        op();
        let change = region.change();
        AllocDelta {
            allocations: change.allocations,
            deallocations: change.deallocations,
            bytes_allocated: change.bytes_allocated,
            bytes_deallocated: change.bytes_deallocated,
            bytes_reallocated: change.bytes_reallocated.unsigned_abs(),
        }
    })?;

    let mut alloc_md = String::from(
        "| Label | path | allocs | deallocs | bytes_alloc | bytes_dealloc | realloc_bytes | out_B |\n",
    );
    alloc_md.push_str(
        "|-------|------|-------:|---------:|------------:|--------------:|--------------:|------:|\n",
    );
    for row in &alloc_rows {
        alloc_md.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |\n",
            row.label,
            row.path,
            row.delta.allocations,
            row.delta.deallocations,
            row.delta.bytes_allocated,
            row.delta.bytes_deallocated,
            row.delta.bytes_reallocated,
            row.bytes_out,
        ));
    }
    alloc_md.push('\n');
    alloc_md.push_str(
        "Notes: **warm prepared encode still allocates today** (see numbers) — mostly value-path heap traffic (`BigInt` coefficients, temporary buffers), not schema re-encoding. ",
    );
    alloc_md.push_str(
        "Naive FullSchema pays additional schema serialization + a fresh output `Vec` every call. Decode allocates AST/values on every message. Zero-alloc hot paths are a future optimization target, not a current claim.\n",
    );

    markdown = markdown.replace("<!-- ALLOC_TABLE -->\n", &alloc_md);

    let output = env::args()
        .filter(|a| a != "--quick")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/benchmarks.md"));
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&output, &markdown)?;
    print!("{markdown}");
    eprintln!("wrote {}", output.display());
    Ok(())
}
