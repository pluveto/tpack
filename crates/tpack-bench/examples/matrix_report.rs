use std::{alloc::System, env, fs, path::PathBuf, process};

use stats_alloc::{Region, StatsAlloc};
use tpack_bench::{Cfg, fill_allocs, flat_record, markdown, run};

#[global_allocator]
static GLOBAL: StatsAlloc<System> = StatsAlloc::system();

fn main() {
    if let Err(e) = run_main() {
        eprintln!("matrix_report: {e}");
        process::exit(1);
    }
}

fn run_main() -> Result<(), Box<dyn std::error::Error>> {
    let quick = env::args().any(|a| a == "--quick");
    let cfg = if quick { Cfg::quick() } else { Cfg::report() };
    eprintln!("suite running (quick={quick})…");
    let mut cat = run(cfg)?;
    fill_allocs(&mut cat, &flat_record(), |_label, op| {
        let region = Region::new(&GLOBAL);
        op();
        let c = region.change();
        (c.allocations, c.bytes_allocated)
    })?;
    let md = markdown(&cat);
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
