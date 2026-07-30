//! Host-dependent performance metrics: latency tails, cold/warm, concurrency.
//!
//! Numbers are **not** portable across machines. Always pair with [`HostInfo`].

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tpack::{Decoder, Encoder, EnvelopeMode, PreparedSchema, encode_message};

use crate::codec_ops::{
    EncodeError, SteadyEncoder, decode_tpack_with_registry, encode, encode_tpack_naive,
    preload_registry, preload_registry_from_schema,
};
use crate::format::Format;
use crate::payload::{Scenario, tpack_schema, tpack_value};

/// Host label for checked-in perf sections.
#[derive(Debug, Clone)]
pub struct HostInfo {
    pub summary: String,
}

impl HostInfo {
    pub fn detect() -> Self {
        let arch = std::env::consts::ARCH;
        let os = std::env::consts::OS;
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        // Best-effort CPU brand on Linux.
        let cpu = std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("model name"))
                    .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            })
            .unwrap_or_else(|| "unknown-cpu".into());
        Self {
            summary: format!("{os}/{arch}, {cpus} logical CPUs, {cpu}"),
        }
    }
}

/// Latency percentiles in nanoseconds (single-thread, warm unless noted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatencyStats {
    pub samples: usize,
    pub p50_ns: u64,
    pub p90_ns: u64,
    pub p99_ns: u64,
    pub max_ns: u64,
    pub mean_ns: u64,
}

impl LatencyStats {
    pub fn from_samples(mut samples: Vec<u64>) -> Self {
        assert!(!samples.is_empty());
        samples.sort_unstable();
        let n = samples.len();
        let sum: u128 = samples.iter().map(|&x| u128::from(x)).sum();
        Self {
            samples: n,
            p50_ns: percentile(&samples, 50),
            p90_ns: percentile(&samples, 90),
            p99_ns: percentile(&samples, 99),
            max_ns: samples[n - 1],
            mean_ns: (sum / n as u128) as u64,
        }
    }
}

fn percentile(sorted: &[u64], pct: usize) -> u64 {
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    // Nearest-rank method.
    let rank = ((pct as f64 / 100.0) * n as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    sorted[idx]
}

#[derive(Debug, Clone)]
pub struct LatencyRow {
    pub label: String,
    pub path: &'static str,
    pub stats: LatencyStats,
    pub bytes_per_op: usize,
    pub mib_per_s_at_p50: f64,
}

fn mib_s(bytes: usize, ns: u64) -> f64 {
    if ns == 0 {
        return 0.0;
    }
    (bytes as f64 / (1024.0 * 1024.0)) / (ns as f64 / 1e9)
}

/// Configuration for a matrix timing run.
#[derive(Debug, Clone)]
pub struct PerfConfig {
    pub warmup: usize,
    pub samples: usize,
    /// Skip 1MiB blob latency by default in "quick" mode.
    pub include_large_blob: bool,
    pub concurrent_threads: usize,
    pub concurrent_messages_per_thread: usize,
}

impl Default for PerfConfig {
    fn default() -> Self {
        Self {
            warmup: 200,
            samples: 2_000,
            include_large_blob: false,
            concurrent_threads: 4,
            concurrent_messages_per_thread: 5_000,
        }
    }
}

impl PerfConfig {
    /// Faster CI-safe smoke (still exercises paths).
    pub fn quick() -> Self {
        Self {
            warmup: 20,
            samples: 100,
            include_large_blob: false,
            concurrent_threads: 2,
            concurrent_messages_per_thread: 200,
        }
    }

    /// Fuller local report for docs.
    pub fn report() -> Self {
        Self {
            warmup: 500,
            samples: 5_000,
            include_large_blob: true,
            concurrent_threads: std::thread::available_parallelism()
                .map(|n| n.get().min(8))
                .unwrap_or(4),
            concurrent_messages_per_thread: 10_000,
        }
    }
}

fn time_ns(mut op: impl FnMut()) -> u64 {
    let t0 = Instant::now();
    op();
    t0.elapsed().as_nanos() as u64
}

fn collect_latency(warmup: usize, samples: usize, mut op: impl FnMut()) -> LatencyStats {
    for _ in 0..warmup {
        op();
    }
    let mut xs = Vec::with_capacity(samples);
    for _ in 0..samples {
        xs.push(time_ns(&mut op));
    }
    LatencyStats::from_samples(xs)
}

/// Core latency matrix for fixed scenarios (warm prepared paths unless labeled cold/naive).
pub fn measure_latency_matrix(cfg: &PerfConfig) -> Result<Vec<LatencyRow>, EncodeError> {
    let mut rows = Vec::new();

    for scenario in Scenario::ALL {
        // --- Warm SchemaRef encode/decode ---
        {
            let mut steady = SteadyEncoder::for_scenario(scenario, Format::TpackSchemaRef)?;
            let encoded = steady.encode_once()?;
            let bytes = encoded.len();
            let (registry, _, _) = preload_registry(scenario);

            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let _ = steady.encode_in_place().expect("encode");
            });
            rows.push(LatencyRow {
                label: format!("{}/tpack-schemaref/encode/warm", scenario.name()),
                path: "warm-prepared",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });

            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let mut decoder = Decoder::new(&encoded);
                let _ = decoder
                    .decode_message_with_registry(&registry)
                    .expect("decode");
            });
            rows.push(LatencyRow {
                label: format!("{}/tpack-schemaref/decode/warm", scenario.name()),
                path: "warm-registry",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });

            // Round-trip warm
            let mut steady = SteadyEncoder::for_scenario(scenario, Format::TpackSchemaRef)?;
            let (registry, _, _) = preload_registry(scenario);
            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let bytes = steady.encode_in_place().expect("encode");
                let mut decoder = Decoder::new(bytes);
                let _ = decoder
                    .decode_message_with_registry(&registry)
                    .expect("decode");
            });
            rows.push(LatencyRow {
                label: format!("{}/tpack-schemaref/roundtrip/warm", scenario.name()),
                path: "warm-prepared+registry",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });
        }

        // --- Cold/naive FullSchema (re-encode schema every time) ---
        {
            let schema = tpack_schema(scenario);
            let value = tpack_value(scenario);
            let sample = encode_tpack_naive(&schema, &value, EnvelopeMode::FullSchema, None)?;
            let bytes = sample.len();
            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let _ =
                    encode_message(&schema, &value, EnvelopeMode::FullSchema, None).expect("naive");
            });
            rows.push(LatencyRow {
                label: format!(
                    "{}/tpack-fullschema/encode/naive-cold-schema",
                    scenario.name()
                ),
                path: "naive-reencode-schema",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });

            // Prepared FullSchema warm (fair steady-state for self-contained)
            let mut steady = SteadyEncoder::for_scenario(scenario, Format::TpackFullSchema)?;
            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let _ = steady.encode_in_place().expect("encode");
            });
            rows.push(LatencyRow {
                label: format!("{}/tpack-fullschema/encode/warm-prepared", scenario.name()),
                path: "warm-prepared",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });
        }

        // --- JSON encode/decode baseline ---
        {
            let payload = crate::payload::serde_payload(scenario);
            let encoded = serde_json::to_vec(&payload).map_err(EncodeError::Json)?;
            let bytes = encoded.len();
            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let _ = serde_json::to_vec(&payload).expect("json");
            });
            rows.push(LatencyRow {
                label: format!("{}/json/encode", scenario.name()),
                path: "oneshot",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });
            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let _: crate::payload::SerdePayload =
                    serde_json::from_slice(&encoded).expect("json decode");
            });
            rows.push(LatencyRow {
                label: format!("{}/json/decode", scenario.name()),
                path: "oneshot",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });
        }

        // --- CBOR encode baseline ---
        {
            let payload = crate::payload::serde_payload(scenario);
            let mut buf = Vec::new();
            ciborium::into_writer(&payload, &mut buf)
                .map_err(|e| EncodeError::Cbor(e.to_string()))?;
            let bytes = buf.len();
            let stats = collect_latency(cfg.warmup, cfg.samples, || {
                let mut b = Vec::new();
                ciborium::into_writer(&payload, &mut b).expect("cbor");
                std::hint::black_box(b);
            });
            rows.push(LatencyRow {
                label: format!("{}/cbor/encode", scenario.name()),
                path: "oneshot",
                stats: stats.clone(),
                bytes_per_op: bytes,
                mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
            });
        }
    }

    // Blob scaling latency (SchemaRef warm encode) — shows bandwidth-ish regime.
    let blob_points: &[usize] = if cfg.include_large_blob {
        &[0, 64, 1024, 16_384, 1_048_576]
    } else {
        &[0, 64, 1024, 16_384]
    };
    for &len in blob_points {
        let mut steady = SteadyEncoder::for_blob(len, Format::TpackSchemaRef)?;
        let encoded = steady.encode_once()?;
        let bytes = encoded.len();
        let samples = if len >= 1_048_576 {
            cfg.samples.min(200)
        } else {
            cfg.samples
        };
        let warmup = if len >= 1_048_576 {
            cfg.warmup.min(20)
        } else {
            cfg.warmup
        };
        let stats = collect_latency(warmup, samples, || {
            let _ = steady.encode_in_place().expect("encode");
        });
        rows.push(LatencyRow {
            label: format!("blob_{len}/tpack-schemaref/encode/warm"),
            path: "warm-prepared",
            stats: stats.clone(),
            bytes_per_op: bytes,
            mib_per_s_at_p50: mib_s(bytes, stats.p50_ns),
        });
    }

    Ok(rows)
}

#[derive(Debug, Clone)]
pub struct ConcurrentRow {
    pub threads: usize,
    pub messages_per_thread: usize,
    pub total_messages: usize,
    pub wall: Duration,
    pub msgs_per_s: f64,
    pub note: &'static str,
}

/// Multi-thread SchemaRef decode against a **shared** registry (lock contention probe).
pub fn measure_concurrent_schemaref_decode(cfg: &PerfConfig) -> Result<ConcurrentRow, EncodeError> {
    let scenario = Scenario::FlatRecord;
    let encoded = encode(scenario, Format::TpackSchemaRef)?;
    let (registry, _, _) = preload_registry(scenario);
    let registry = Arc::new(registry);
    let encoded = Arc::new(encoded);
    let threads = cfg.concurrent_threads.max(1);
    let per = cfg.concurrent_messages_per_thread.max(1);

    let t0 = Instant::now();
    thread::scope(|scope| {
        for _ in 0..threads {
            let registry = Arc::clone(&registry);
            let encoded = Arc::clone(&encoded);
            scope.spawn(move || {
                for _ in 0..per {
                    decode_tpack_with_registry(
                        encoded.as_slice(),
                        registry.as_ref(),
                        Format::TpackSchemaRef,
                    )
                    .expect("decode");
                }
            });
        }
    });
    let wall = t0.elapsed();
    let total = threads * per;
    let msgs_per_s = total as f64 / wall.as_secs_f64();
    Ok(ConcurrentRow {
        threads,
        messages_per_thread: per,
        total_messages: total,
        wall,
        msgs_per_s,
        note: "shared StdSchemaRegistry (RwLock) SchemaRef decode",
    })
}

/// Same work with **per-thread** registries (no lock sharing).
pub fn measure_concurrent_schemaref_decode_sharded(
    cfg: &PerfConfig,
) -> Result<ConcurrentRow, EncodeError> {
    let scenario = Scenario::FlatRecord;
    let encoded = encode(scenario, Format::TpackSchemaRef)?;
    let schema = tpack_schema(scenario);
    let encoded = Arc::new(encoded);
    let threads = cfg.concurrent_threads.max(1);
    let per = cfg.concurrent_messages_per_thread.max(1);

    let t0 = Instant::now();
    thread::scope(|scope| {
        for _ in 0..threads {
            let schema = schema.clone();
            let encoded = Arc::clone(&encoded);
            scope.spawn(move || {
                let (registry, _, _) = preload_registry_from_schema(schema);
                for _ in 0..per {
                    decode_tpack_with_registry(
                        encoded.as_slice(),
                        &registry,
                        Format::TpackSchemaRef,
                    )
                    .expect("decode");
                }
            });
        }
    });
    let wall = t0.elapsed();
    let total = threads * per;
    Ok(ConcurrentRow {
        threads,
        messages_per_thread: per,
        total_messages: total,
        wall,
        msgs_per_s: total as f64 / wall.as_secs_f64(),
        note: "per-thread registry (no shared lock)",
    })
}

/// Allocation delta measured by the caller (e.g. stats_alloc Region).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllocDelta {
    pub allocations: usize,
    pub deallocations: usize,
    pub bytes_allocated: usize,
    pub bytes_deallocated: usize,
    pub bytes_reallocated: usize,
}

#[derive(Debug, Clone)]
pub struct AllocRow {
    pub label: String,
    pub path: &'static str,
    pub delta: AllocDelta,
    pub bytes_out: usize,
}

/// Run `op` under an external alloc observer. The observer returns a delta.
pub fn measure_alloc_rows<F>(mut observe: F) -> Result<Vec<AllocRow>, EncodeError>
where
    F: FnMut(&str, &mut dyn FnMut()) -> AllocDelta,
{
    let mut rows = Vec::new();
    let scenario = Scenario::FlatRecord;

    // Warm prepared SchemaRef encode (steadyEncoder first encode builds nothing extra after prepare)
    {
        let mut steady = SteadyEncoder::for_scenario(scenario, Format::TpackSchemaRef)?;
        // Drop prepare cost outside measurement: one throwaway encode.
        let _ = steady.encode_once()?;
        let mut out_len = 0usize;
        let delta = observe("schemaref-encode-warm", &mut || {
            let b = steady.encode_in_place().expect("encode");
            out_len = b.len();
        });
        rows.push(AllocRow {
            label: "flat_record/tpack-schemaref/encode/warm".into(),
            path: "warm-prepared",
            delta,
            bytes_out: out_len,
        });
    }

    // Naive FullSchema encode_message (allocates new Vec + schema bytes every time)
    {
        let schema = tpack_schema(scenario);
        let value = tpack_value(scenario);
        let mut out_len = 0usize;
        let delta = observe("fullschema-encode-naive", &mut || {
            let b = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).expect("enc");
            out_len = b.len();
            std::hint::black_box(b);
        });
        rows.push(AllocRow {
            label: "flat_record/tpack-fullschema/encode/naive".into(),
            path: "naive-reencode-schema",
            delta,
            bytes_out: out_len,
        });
    }

    // Prepared FullSchema with buffer reuse
    {
        let mut steady = SteadyEncoder::for_scenario(scenario, Format::TpackFullSchema)?;
        let _ = steady.encode_once()?;
        let mut out_len = 0usize;
        let delta = observe("fullschema-encode-warm", &mut || {
            let b = steady.encode_in_place().expect("encode");
            out_len = b.len();
        });
        rows.push(AllocRow {
            label: "flat_record/tpack-fullschema/encode/warm-prepared".into(),
            path: "warm-prepared",
            delta,
            bytes_out: out_len,
        });
    }

    // SchemaRef decode
    {
        let encoded = encode(scenario, Format::TpackSchemaRef)?;
        let (registry, _, _) = preload_registry(scenario);
        let delta = observe("schemaref-decode-warm", &mut || {
            let mut decoder = Decoder::new(&encoded);
            let msg = decoder
                .decode_message_with_registry(&registry)
                .expect("decode");
            std::hint::black_box(msg);
        });
        rows.push(AllocRow {
            label: "flat_record/tpack-schemaref/decode/warm".into(),
            path: "warm-registry",
            delta,
            bytes_out: encoded.len(),
        });
    }

    // JSON encode baseline
    {
        let payload = crate::payload::serde_payload(scenario);
        let mut out_len = 0usize;
        let delta = observe("json-encode", &mut || {
            let b = serde_json::to_vec(&payload).expect("json");
            out_len = b.len();
            std::hint::black_box(b);
        });
        rows.push(AllocRow {
            label: "flat_record/json/encode".into(),
            path: "oneshot",
            delta,
            bytes_out: out_len,
        });
    }

    Ok(rows)
}

/// Speedup of prepared FullSchema vs naive (same flat_record).
pub fn measure_prepared_speedup(iters: u32) -> Result<(f64, f64, f64), EncodeError> {
    let scenario = Scenario::FlatRecord;
    let schema = tpack_schema(scenario);
    let value = tpack_value(scenario);

    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).unwrap();
    }
    let naive = t0.elapsed().as_secs_f64();

    let prepared = PreparedSchema::prepare_default(schema.clone()).map_err(EncodeError::Tpack)?;
    let mut enc = Encoder::new();
    let t1 = Instant::now();
    for _ in 0..iters {
        enc.clear();
        enc.encode_prepared_message(&prepared, &value, EnvelopeMode::FullSchema, None)
            .unwrap();
        std::hint::black_box(enc.as_slice());
    }
    let prep = t1.elapsed().as_secs_f64();

    let mut steady = SteadyEncoder::for_scenario(scenario, Format::TpackSchemaRef)?;
    let t2 = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(steady.encode_in_place().unwrap());
    }
    let sref = t2.elapsed().as_secs_f64();

    Ok((
        naive / iters as f64 * 1e9,
        prep / iters as f64 * 1e9,
        sref / iters as f64 * 1e9,
    ))
}
