use std::{env, fs, path::PathBuf, process};
use tpack_bench::size_report_markdown;

fn main() {
    if let Err(e) = run() {
        eprintln!("size_report: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let md = size_report_markdown()?;
    let out = env::args()
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
