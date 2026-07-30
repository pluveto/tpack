//! Suite: a list of experiments. Each experiment answers with observations.
//!
//! Adding a measurement = adding a small experiment object, not editing a god function.

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use tpack::{Encoder, EnvelopeMode, PreparedSchema, encode_message};

use crate::endpoint::{Endpoint, Error, SerdeFormat, TpackEnvelope, TpackStyle, tpack_parts};
use crate::obs::{Catalog, Observation, Quantity, dims, latency_stats};
use crate::work::{
    self, AMORTIZED_NS, BLOB_SIZES, FIELD_COUNTS, LIST_LENS, Workload, flat_record, narrative,
};

#[derive(Debug, Clone, Copy)]
pub struct Cfg {
    pub host_metrics: bool,
    pub warmup: usize,
    pub samples: usize,
    pub large_blob: bool,
    pub speedup_iters: u32,
    pub threads: usize,
    pub msgs_per_thread: usize,
}

impl Cfg {
    pub fn size_only() -> Self {
        Self {
            host_metrics: false,
            warmup: 0,
            samples: 0,
            large_blob: false,
            speedup_iters: 0,
            threads: 0,
            msgs_per_thread: 0,
        }
    }

    pub fn quick() -> Self {
        Self {
            host_metrics: true,
            warmup: 20,
            samples: 100,
            large_blob: false,
            speedup_iters: 20_000,
            threads: 2,
            msgs_per_thread: 200,
        }
    }

    pub fn report() -> Self {
        Self {
            host_metrics: true,
            warmup: 500,
            samples: 5_000,
            large_blob: true,
            speedup_iters: 100_000,
            threads: std::thread::available_parallelism()
                .map(|n| n.get().min(8))
                .unwrap_or(4),
            msgs_per_thread: 10_000,
        }
    }
}

/// Run every experiment into one catalog.
pub fn run(cfg: Cfg) -> Result<Catalog, Error> {
    let mut cat = Catalog::default();
    SizeGrid.record(&mut cat)?;
    ScaleGrid.record(&mut cat)?;
    AmortizationGrid.record(&mut cat)?;

    if cfg.host_metrics {
        cat.host = Some(host_label());
        LatencyGrid { cfg }.record(&mut cat)?;
        ConcurrentProbe { cfg }.record(&mut cat)?;
        SpeedupProbe {
            iters: cfg.speedup_iters,
        }
        .record(&mut cat)?;
    }
    Ok(cat)
}

/// Attach alloc observations (needs a counting global allocator in the example).
pub fn record_allocs(
    cat: &mut Catalog,
    work: &Workload,
    mut count: impl FnMut(&mut dyn FnMut()) -> (usize, usize),
) -> Result<(), Error> {
    let cases: [(&str, TpackEnvelope, TpackStyle, bool); 3] = [
        ("encode", TpackEnvelope::SchemaRef, TpackStyle::Warm, true),
        (
            "encode",
            TpackEnvelope::FullSchema,
            TpackStyle::Naive,
            false,
        ),
        ("encode", TpackEnvelope::FullSchema, TpackStyle::Warm, true),
    ];
    for (op, env, style, warm_prime) in cases {
        let mut ep = match style {
            TpackStyle::Warm => Endpoint::tpack_warm(work, env)?,
            TpackStyle::Naive => Endpoint::tpack_naive(work, env),
        };
        if warm_prime {
            let _ = ep.encode()?;
        }
        let mut out = 0usize;
        let (allocations, bytes_allocated) = count(&mut || {
            out = ep.encode().expect("e").len();
        });
        cat.push(Observation::new(
            "alloc",
            dims(&[
                ("case", work.name()),
                ("format", env.label()),
                ("op", op),
                ("path", ep.path_label()),
            ]),
            Quantity::Alloc {
                allocations,
                bytes_allocated,
                out_bytes: out,
            },
        ));
    }

    // decode
    let mut ep = Endpoint::tpack_warm(work, TpackEnvelope::SchemaRef)?;
    let bytes = ep.encode()?;
    let (allocations, bytes_allocated) = count(&mut || {
        ep.decode(&bytes).expect("d");
    });
    cat.push(Observation::new(
        "alloc",
        dims(&[
            ("case", work.name()),
            ("format", "tpack-schemaref"),
            ("op", "decode"),
            ("path", "warm-prepared"),
        ]),
        Quantity::Alloc {
            allocations,
            bytes_allocated,
            out_bytes: bytes.len(),
        },
    ));

    // json
    let mut json = Endpoint::serde(work, SerdeFormat::Json);
    let mut out = 0usize;
    let (allocations, bytes_allocated) = count(&mut || {
        out = json.encode().expect("j").len();
    });
    cat.push(Observation::new(
        "alloc",
        dims(&[
            ("case", work.name()),
            ("format", "json"),
            ("op", "encode"),
            ("path", "oneshot"),
        ]),
        Quantity::Alloc {
            allocations,
            bytes_allocated,
            out_bytes: out,
        },
    ));
    Ok(())
}

// --- experiments -------------------------------------------------------------

trait Experiment {
    fn record(&self, cat: &mut Catalog) -> Result<(), Error>;
}

struct SizeGrid;
impl Experiment for SizeGrid {
    fn record(&self, cat: &mut Catalog) -> Result<(), Error> {
        for work in narrative() {
            for mut ep in all_endpoints(&work)? {
                let bytes = ep.encode()?;
                let section = if ep.label().starts_with("tpack-fullschema") {
                    "size.cold"
                } else {
                    "size.steady"
                };
                cat.push(Observation::new(
                    section,
                    dims(&[("case", work.name()), ("format", ep.label())]),
                    Quantity::Bytes(bytes.len()),
                ));
                if let Some(env) = envelope_of(ep.label()) {
                    let (header, schema_id, schema, data) = tpack_parts(&bytes, env)?;
                    cat.push(Observation::new(
                        "size.parts",
                        dims(&[("case", work.name()), ("format", ep.label())]),
                        Quantity::Parts {
                            total: bytes.len(),
                            header,
                            schema_id,
                            schema,
                            data,
                        },
                    ));
                }
            }
        }
        Ok(())
    }
}

struct ScaleGrid;
impl Experiment for ScaleGrid {
    fn record(&self, cat: &mut Catalog) -> Result<(), Error> {
        for &n in BLOB_SIZES {
            let work = work::blob(n);
            let formats = if n <= 1024 {
                scale_all(&work)?
            } else {
                scale_steady(&work)?
            };
            push_sizes(cat, "scale.blob", &work, formats)?;
        }
        for &n in LIST_LENS {
            let work = work::list(n);
            let formats = if n <= 100 {
                scale_all(&work)?
            } else {
                scale_steady(&work)?
            };
            push_sizes(cat, "scale.list", &work, formats)?;
        }
        for &n in FIELD_COUNTS {
            let work = work::wide(n);
            push_sizes(cat, "scale.fields", &work, scale_all(&work)?)?;
        }
        Ok(())
    }
}

struct AmortizationGrid;
impl Experiment for AmortizationGrid {
    fn record(&self, cat: &mut Catalog) -> Result<(), Error> {
        for work in narrative() {
            let mut cold = Endpoint::tpack_warm(&work, TpackEnvelope::FullSchemaWithId)?;
            let mut hot = Endpoint::tpack_warm(&work, TpackEnvelope::SchemaRef)?;
            let mut json = Endpoint::serde(&work, SerdeFormat::Json);
            let mut cbor = Endpoint::serde(&work, SerdeFormat::Cbor);
            let cold_n = cold.encode()?.len();
            let hot_n = hot.encode()?.len();
            let json_n = json.encode()?.len();
            let cbor_n = cbor.encode()?.len();
            for &n in AMORTIZED_NS {
                cat.push(Observation::new(
                    "amortized",
                    dims(&[("case", work.name())]),
                    Quantity::Stream {
                        n,
                        tpack: cold_n + hot_n * n.saturating_sub(1),
                        json: json_n * n,
                        cbor: cbor_n * n,
                    },
                ));
            }
        }
        Ok(())
    }
}

struct LatencyGrid {
    cfg: Cfg,
}
impl Experiment for LatencyGrid {
    fn record(&self, cat: &mut Catalog) -> Result<(), Error> {
        for work in narrative() {
            // warm schemaref encode/decode/rt
            let mut warm = Endpoint::tpack_warm(&work, TpackEnvelope::SchemaRef)?;
            let encoded = warm.encode()?;
            record_latency(
                cat,
                &work,
                &mut warm,
                "encode",
                encoded.len(),
                self.cfg,
                |ep| {
                    let _ = ep.encode()?;
                    Ok(())
                },
            )?;
            record_latency(cat, &work, &mut warm, "decode", encoded.len(), self.cfg, {
                let encoded = encoded.clone();
                move |ep| {
                    ep.decode(&encoded)?;
                    Ok(())
                }
            })?;
            record_latency(
                cat,
                &work,
                &mut warm,
                "roundtrip",
                encoded.len(),
                self.cfg,
                |ep| {
                    let b = ep.encode()?;
                    ep.decode(&b)?;
                    Ok(())
                },
            )?;

            // naive vs warm fullschema encode
            let mut naive = Endpoint::tpack_naive(&work, TpackEnvelope::FullSchema);
            let n = naive.encode()?.len();
            record_latency(cat, &work, &mut naive, "encode", n, self.cfg, |ep| {
                let _ = ep.encode()?;
                Ok(())
            })?;
            let mut prepared = Endpoint::tpack_warm(&work, TpackEnvelope::FullSchema)?;
            record_latency(cat, &work, &mut prepared, "encode", n, self.cfg, |ep| {
                let _ = ep.encode()?;
                Ok(())
            })?;

            // json encode (+ decode for flat)
            let mut json = Endpoint::serde(&work, SerdeFormat::Json);
            let jb = json.encode()?;
            record_latency(cat, &work, &mut json, "encode", jb.len(), self.cfg, |ep| {
                let _ = ep.encode()?;
                Ok(())
            })?;
            if work.name() == "flat_record" {
                record_latency(cat, &work, &mut json, "decode", jb.len(), self.cfg, {
                    let jb = jb.clone();
                    move |ep| {
                        ep.decode(&jb)?;
                        Ok(())
                    }
                })?;
            }
            let mut cbor = Endpoint::serde(&work, SerdeFormat::Cbor);
            let cb = cbor.encode()?.len();
            record_latency(cat, &work, &mut cbor, "encode", cb, self.cfg, |ep| {
                let _ = ep.encode()?;
                Ok(())
            })?;
        }

        let blobs: &[usize] = if self.cfg.large_blob {
            BLOB_SIZES
        } else {
            &[0, 64, 1024, 16_384]
        };
        for &n in blobs {
            let work = work::blob(n);
            let mut ep = Endpoint::tpack_warm(&work, TpackEnvelope::SchemaRef)?;
            let bytes = ep.encode()?.len();
            let mut cfg = self.cfg;
            if n >= 1_048_576 {
                cfg.samples = cfg.samples.min(200);
                cfg.warmup = cfg.warmup.min(20);
            }
            record_latency(cat, &work, &mut ep, "encode", bytes, cfg, |ep| {
                let _ = ep.encode()?;
                Ok(())
            })?;
        }
        Ok(())
    }
}

struct ConcurrentProbe {
    cfg: Cfg,
}
impl Experiment for ConcurrentProbe {
    fn record(&self, cat: &mut Catalog) -> Result<(), Error> {
        let work = flat_record();
        let mut ep = Endpoint::tpack_warm(&work, TpackEnvelope::SchemaRef)?;
        let bytes = Arc::new(ep.encode()?);
        let threads = self.cfg.threads.max(1);
        let per = self.cfg.msgs_per_thread.max(1);

        // shared
        {
            let reg = Endpoint::shared_registry(&work);
            let t0 = Instant::now();
            thread::scope(|s| {
                for _ in 0..threads {
                    let reg = Arc::clone(&reg);
                    let bytes = Arc::clone(&bytes);
                    s.spawn(move || {
                        for _ in 0..per {
                            Endpoint::decode_with_registry(bytes.as_slice(), reg.as_ref())
                                .expect("d");
                        }
                    });
                }
            });
            let wall = t0.elapsed();
            let total = threads * per;
            cat.push(Observation::new(
                "concurrent",
                dims(&[("mode", "shared-registry")]),
                Quantity::Concurrent {
                    threads,
                    total_messages: total,
                    wall,
                    msgs_per_s: total as f64 / wall.as_secs_f64().max(1e-12),
                },
            ));
        }
        // sharded
        {
            let schema = work.schema().clone();
            let id = work.schema_id();
            let t0 = Instant::now();
            thread::scope(|s| {
                for _ in 0..threads {
                    let schema = schema.clone();
                    let bytes = Arc::clone(&bytes);
                    s.spawn(move || {
                        let reg = tpack::StdSchemaRegistry::new();
                        reg.insert(id, schema).expect("i");
                        for _ in 0..per {
                            Endpoint::decode_with_registry(bytes.as_slice(), &reg).expect("d");
                        }
                    });
                }
            });
            let wall = t0.elapsed();
            let total = threads * per;
            cat.push(Observation::new(
                "concurrent",
                dims(&[("mode", "per-thread-registry")]),
                Quantity::Concurrent {
                    threads,
                    total_messages: total,
                    wall,
                    msgs_per_s: total as f64 / wall.as_secs_f64().max(1e-12),
                },
            ));
        }
        Ok(())
    }
}

struct SpeedupProbe {
    iters: u32,
}
impl Experiment for SpeedupProbe {
    fn record(&self, cat: &mut Catalog) -> Result<(), Error> {
        if self.iters == 0 {
            return Ok(());
        }
        let work = flat_record();
        let t0 = Instant::now();
        for _ in 0..self.iters {
            let _ = encode_message(work.schema(), work.value(), EnvelopeMode::FullSchema, None)?;
        }
        let naive = t0.elapsed().as_secs_f64();

        let prepared = PreparedSchema::prepare_default(work.schema().clone())?;
        let mut enc = Encoder::new();
        let t1 = Instant::now();
        for _ in 0..self.iters {
            enc.clear();
            enc.encode_prepared_message(&prepared, work.value(), EnvelopeMode::FullSchema, None)?;
            std::hint::black_box(enc.as_slice());
        }
        let prep = t1.elapsed().as_secs_f64();

        let mut warm = Endpoint::tpack_warm(&work, TpackEnvelope::SchemaRef)?;
        let t2 = Instant::now();
        for _ in 0..self.iters {
            std::hint::black_box(warm.encode()?);
        }
        let sref = t2.elapsed().as_secs_f64();
        let inv = 1e9 / f64::from(self.iters);
        cat.push(Observation::new(
            "speedup",
            dims(&[("case", "flat_record")]),
            Quantity::Speedup {
                naive_ns: naive * inv,
                prepared_ns: prep * inv,
                schemaref_ns: sref * inv,
            },
        ));
        Ok(())
    }
}

// --- helpers -----------------------------------------------------------------

fn all_endpoints(work: &Workload) -> Result<Vec<Endpoint>, Error> {
    Ok(vec![
        Endpoint::tpack_warm(work, TpackEnvelope::FullSchema)?,
        Endpoint::tpack_warm(work, TpackEnvelope::FullSchemaWithId)?,
        Endpoint::tpack_warm(work, TpackEnvelope::SchemaRef)?,
        Endpoint::serde(work, SerdeFormat::Json),
        Endpoint::serde(work, SerdeFormat::Cbor),
        Endpoint::serde(work, SerdeFormat::MsgPack),
    ])
}

fn scale_all(work: &Workload) -> Result<Vec<Endpoint>, Error> {
    all_endpoints(work)
}

fn scale_steady(work: &Workload) -> Result<Vec<Endpoint>, Error> {
    Ok(vec![
        Endpoint::tpack_warm(work, TpackEnvelope::SchemaRef)?,
        Endpoint::serde(work, SerdeFormat::Json),
        Endpoint::serde(work, SerdeFormat::Cbor),
        Endpoint::serde(work, SerdeFormat::MsgPack),
    ])
}

fn push_sizes(
    cat: &mut Catalog,
    section: &'static str,
    work: &Workload,
    endpoints: Vec<Endpoint>,
) -> Result<(), Error> {
    for mut ep in endpoints {
        let n = ep.encode()?.len();
        cat.push(Observation::new(
            section,
            dims(&[("case", work.name()), ("format", ep.label())]),
            Quantity::Bytes(n),
        ));
    }
    Ok(())
}

fn envelope_of(label: &str) -> Option<TpackEnvelope> {
    match label {
        "tpack-fullschema" => Some(TpackEnvelope::FullSchema),
        "tpack-fullschema-with-id" => Some(TpackEnvelope::FullSchemaWithId),
        "tpack-schemaref" => Some(TpackEnvelope::SchemaRef),
        _ => None,
    }
}

fn record_latency<F>(
    cat: &mut Catalog,
    work: &Workload,
    ep: &mut Endpoint,
    op: &str,
    bytes_per_op: usize,
    cfg: Cfg,
    mut body: F,
) -> Result<(), Error>
where
    F: FnMut(&mut Endpoint) -> Result<(), Error>,
{
    for _ in 0..cfg.warmup {
        body(ep)?;
    }
    let mut xs = Vec::with_capacity(cfg.samples);
    for _ in 0..cfg.samples {
        let t0 = Instant::now();
        body(ep)?;
        xs.push(t0.elapsed().as_nanos() as u64);
    }
    let (n, p50, p90, p99, max, mean) = latency_stats(xs);
    cat.push(Observation::new(
        "latency",
        dims(&[
            ("case", work.name()),
            ("format", ep.label()),
            ("op", op),
            ("path", ep.path_label()),
        ]),
        Quantity::Latency {
            n,
            p50,
            p90,
            p99,
            max,
            mean,
            bytes_per_op,
        },
    ));
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
        "{}/{}, {cpus} CPUs, {cpu}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
