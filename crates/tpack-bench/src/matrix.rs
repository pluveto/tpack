//! The experiment list is *data*. Running it produces a [`Report`].

use crate::codec::Error;
use crate::measure::{self, LatencyCfg};
use crate::model::{ExecPath, Format, Op, Report};
use crate::workload::{
    self, AMORTIZED_NS, BLOB_SIZES, FIELD_COUNTS, LIST_LENS, Workload, narrative,
};

#[derive(Debug, Clone, Copy)]
pub struct MatrixCfg {
    pub latency: LatencyCfg,
    pub include_host: bool,
    pub include_large_blob: bool,
    pub speedup_iters: u32,
    pub conc_threads: usize,
    pub conc_msgs: usize,
}

impl MatrixCfg {
    pub fn size_only() -> Self {
        Self {
            latency: LatencyCfg::quick(),
            include_host: false,
            include_large_blob: false,
            speedup_iters: 0,
            conc_threads: 0,
            conc_msgs: 0,
        }
    }

    pub fn quick() -> Self {
        Self {
            latency: LatencyCfg::quick(),
            include_host: true,
            include_large_blob: false,
            speedup_iters: 20_000,
            conc_threads: 2,
            conc_msgs: 200,
        }
    }

    pub fn report() -> Self {
        Self {
            latency: LatencyCfg::report(),
            include_host: true,
            include_large_blob: true,
            speedup_iters: 100_000,
            conc_threads: std::thread::available_parallelism()
                .map(|n| n.get().min(8))
                .unwrap_or(4),
            conc_msgs: 10_000,
        }
    }
}

/// Run the full declared matrix into a structured [`Report`].
pub fn run(cfg: MatrixCfg) -> Result<Report, Error> {
    let mut report = Report::empty();
    let stories = narrative();

    // 1) narrative sizes + breakdown + amortization
    for work in &stories {
        report.sizes.extend(measure::sizes(work, &Format::ALL)?);
        for &f in &[
            Format::TpackFullSchema,
            Format::TpackFullSchemaWithId,
            Format::TpackSchemaRef,
        ] {
            report.breakdowns.push(measure::breakdown(work, f)?);
        }
        for &n in &AMORTIZED_NS {
            report.amortized.push(measure::amortized(work, n)?);
        }
    }

    // 2) scaling axes (declared loops, same measure::size_of)
    for &len in &BLOB_SIZES {
        let work = workload::blob(len);
        let formats: &[Format] = if len <= 1024 {
            &Format::ALL
        } else {
            &Format::STEADY
        };
        for s in measure::sizes(&work, formats)? {
            report.scale_blob.push(s);
        }
    }
    for &len in &LIST_LENS {
        let work = workload::list_of(len);
        let formats: &[Format] = if len <= 100 {
            &Format::ALL
        } else {
            &Format::STEADY
        };
        for s in measure::sizes(&work, formats)? {
            report.scale_list.push(s);
        }
    }
    for &n in &FIELD_COUNTS {
        let work = workload::wide(n);
        for s in measure::sizes(&work, &Format::ALL)? {
            report.scale_fields.push(s);
        }
    }

    if !cfg.include_host {
        return Ok(report);
    }

    report.host = Some(host_label());

    // 3) latency: fixed story × ops, plus blob encode scale
    let flat = &stories[0];
    for work in &stories {
        // warm schemaref encode/decode/rt
        for op in [Op::Encode, Op::Decode, Op::RoundTrip] {
            report.latency.push(measure::latency(
                work,
                Format::TpackSchemaRef,
                op,
                ExecPath::WarmPrepared,
                cfg.latency,
            )?);
        }
        // naive vs warm fullschema encode
        report.latency.push(measure::latency(
            work,
            Format::TpackFullSchema,
            Op::Encode,
            ExecPath::NaiveMessage,
            cfg.latency,
        )?);
        report.latency.push(measure::latency(
            work,
            Format::TpackFullSchema,
            Op::Encode,
            ExecPath::WarmPrepared,
            cfg.latency,
        )?);
        // json/cbor encode
        for format in [Format::Json, Format::Cbor] {
            report.latency.push(measure::latency(
                work,
                format,
                Op::Encode,
                ExecPath::NaiveMessage,
                cfg.latency,
            )?);
        }
        if work.name == "flat_record" {
            report.latency.push(measure::latency(
                work,
                Format::Json,
                Op::Decode,
                ExecPath::NaiveMessage,
                cfg.latency,
            )?);
        }
    }

    let blob_lens: &[usize] = if cfg.include_large_blob {
        &BLOB_SIZES
    } else {
        &[0, 64, 1024, 16_384]
    };
    for &len in blob_lens {
        let work = workload::blob(len);
        let mut lat_cfg = cfg.latency;
        if len >= 1_048_576 {
            lat_cfg.samples = lat_cfg.samples.min(200);
            lat_cfg.warmup = lat_cfg.warmup.min(20);
        }
        report.latency.push(measure::latency(
            &work,
            Format::TpackSchemaRef,
            Op::Encode,
            ExecPath::WarmPrepared,
            lat_cfg,
        )?);
    }

    // 4) concurrency
    report.concurrent.push(measure::concurrent_schemaref_decode(
        flat,
        cfg.conc_threads,
        cfg.conc_msgs,
        true,
    )?);
    report.concurrent.push(measure::concurrent_schemaref_decode(
        flat,
        cfg.conc_threads,
        cfg.conc_msgs,
        false,
    )?);

    // 5) speedup
    if cfg.speedup_iters > 0 {
        report.speedup = Some(measure::prepared_speedup(flat, cfg.speedup_iters)?);
    }

    Ok(report)
}

/// Fill alloc section using an external observer (stats_alloc in example).
pub fn fill_allocs(
    report: &mut Report,
    work: &Workload,
    observe: impl FnMut(&str, &mut dyn FnMut()) -> (usize, usize),
) -> Result<(), Error> {
    report.allocs = measure::run_alloc_cases(work, observe)?;
    Ok(())
}

fn host_label() -> String {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        })
        .unwrap_or_else(|| "unknown-cpu".into());
    format!(
        "{}/{}, {cpus} logical CPUs, {cpu}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
