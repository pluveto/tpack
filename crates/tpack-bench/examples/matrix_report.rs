//! Full matrix: portable sizes + host latency/alloc/concurrency.
use std::{alloc::System, env, fs, path::PathBuf, process};

use stats_alloc::{Region, StatsAlloc};
use tpack_bench::{MatrixCfg, fill_allocs, flat_record, markdown, run_matrix};

#[global_allocator]
static GLOBAL: StatsAlloc<System> = StatsAlloc::system();

fn main() {
    if let Err(e) = run() {
        eprintln!("matrix_report: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let quick = env::args().any(|a| a == "--quick");
    let cfg = if quick {
        MatrixCfg::quick()
    } else {
        MatrixCfg::report()
    };

    eprintln!("running matrix (quick={quick})…");
    let mut report = run_matrix(cfg)?;

    fill_allocs(&mut report, &flat_record(), |_label, op| {
        let region = Region::new(&GLOBAL);
        op();
        let c = region.change();
        (c.allocations, c.bytes_allocated)
    })?;

    let md = markdown(&report);
    let out = env::args()
        .filter(|a| a != "--quick")
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/benchmarks.md"));
    if let Some(p) = out.parent() {
        if !p.as_os_str().is_empty() {
            fs::create_dir_all(p)?;
        }
    }
    fs::write(&out, &md)?;
    print!("{md}");
    eprintln!("wrote {}", out.display());
    Ok(())
}
