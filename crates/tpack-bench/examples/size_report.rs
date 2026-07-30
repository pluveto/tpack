//! Print the size comparison markdown and write `docs/benchmarks.md`.
//!
//! ```bash
//! cargo run -p tpack-bench --example size_report
//! ```
//!
//! Optional path override:
//!
//! ```bash
//! cargo run -p tpack-bench --example size_report -- path/to/benchmarks.md
//! ```

use std::{env, fs, path::PathBuf, process};

use tpack_bench::{measure_all_sizes, render_benchmarks_markdown};

fn main() {
    if let Err(err) = run() {
        eprintln!("size_report: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let samples = measure_all_sizes()?;
    let markdown = render_benchmarks_markdown(&samples);

    let output = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        // examples run with CWD = workspace root when invoked via `cargo run -p …`
        PathBuf::from("docs/benchmarks.md")
    });

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
