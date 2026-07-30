//! Deterministic size matrix only (fast, CI-safe).
//!
//! ```bash
//! cargo run -p tpack-bench --example size_report
//! ```

use std::{env, fs, path::PathBuf, process};

use tpack_bench::render_size_report;

fn main() {
    if let Err(err) = run() {
        eprintln!("size_report: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = render_size_report()?;
    let output = env::args()
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
    eprintln!(
        "wrote {} (size-only; host metrics omitted)",
        output.display()
    );
    Ok(())
}
