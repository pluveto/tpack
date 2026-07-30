//! Measurements: (Workload, Format, Path) → structured samples.
//!
//! No markdown. Callers compose results into a [`crate::model::Report`].

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use tpack::{Decoder, Encoder, EnvelopeMode, PreparedSchema, encode_message};

use crate::codec::{self, Error, WarmTpack, decode_competitor, encode, shared_registry};
use crate::model::{
    AllocSample, AmortizedSample, Breakdown, ConcurrentSample, ExecPath, Format, LatencySample,
    LatencyStats, Op, SizeSample, SpeedupSample,
};
use crate::workload::Workload;

// --- size --------------------------------------------------------------------

pub fn size_of(work: &Workload, format: Format) -> Result<SizeSample, Error> {
    let bytes = encode(work, format, ExecPath::WarmPrepared)?;
    Ok(SizeSample {
        case: work.name.clone(),
        format,
        bytes: bytes.len(),
    })
}

pub fn sizes(work: &Workload, formats: &[Format]) -> Result<Vec<SizeSample>, Error> {
    formats.iter().copied().map(|f| size_of(work, f)).collect()
}

pub fn breakdown(work: &Workload, format: Format) -> Result<Breakdown, Error> {
    let bytes = encode(work, format, ExecPath::WarmPrepared)?;
    let (header, schema_id, schema, data) = codec::parse_breakdown(&bytes, format)?;
    Ok(Breakdown {
        case: work.name.clone(),
        format,
        total: bytes.len(),
        header,
        schema_id,
        schema,
        data,
    })
}

pub fn amortized(work: &Workload, n: usize) -> Result<AmortizedSample, Error> {
    assert!(n >= 1);
    let cold = encode(work, Format::TpackFullSchemaWithId, ExecPath::WarmPrepared)?;
    let hot = encode(work, Format::TpackSchemaRef, ExecPath::WarmPrepared)?;
    let json = encode(work, Format::Json, ExecPath::WarmPrepared)?;
    let cbor = encode(work, Format::Cbor, ExecPath::WarmPrepared)?;
    Ok(AmortizedSample {
        case: work.name.clone(),
        n,
        tpack_total: cold.len() + hot.len() * n.saturating_sub(1),
        json_total: json.len() * n,
        cbor_total: cbor.len() * n,
    })
}

// --- latency -----------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct LatencyCfg {
    pub warmup: usize,
    pub samples: usize,
}

impl LatencyCfg {
    pub fn report() -> Self {
        Self {
            warmup: 500,
            samples: 5_000,
        }
    }

    pub fn quick() -> Self {
        Self {
            warmup: 20,
            samples: 100,
        }
    }
}

fn timed_ns(mut op: impl FnMut()) -> u64 {
    let t0 = Instant::now();
    op();
    t0.elapsed().as_nanos() as u64
}

fn collect(cfg: LatencyCfg, mut op: impl FnMut()) -> LatencyStats {
    for _ in 0..cfg.warmup {
        op();
    }
    let xs: Vec<u64> = (0..cfg.samples).map(|_| timed_ns(&mut op)).collect();
    LatencyStats::from_sorted(xs)
}

pub fn latency(
    work: &Workload,
    format: Format,
    op: Op,
    path: ExecPath,
    cfg: LatencyCfg,
) -> Result<LatencySample, Error> {
    let encoded = encode(work, format, path)?;
    let bytes_per_op = encoded.len();

    let stats = match (format.is_tpack(), op, path) {
        (true, Op::Encode, ExecPath::WarmPrepared) => {
            let mut warm = WarmTpack::new(work, format)?;
            collect(cfg, || {
                let _ = warm.encode().expect("encode");
            })
        }
        (true, Op::Encode, ExecPath::NaiveMessage) => {
            let (mode, with_id) = match format {
                Format::TpackFullSchema => (EnvelopeMode::FullSchema, false),
                Format::TpackFullSchemaWithId => (EnvelopeMode::FullSchemaWithId, true),
                Format::TpackSchemaRef => (EnvelopeMode::SchemaRef, true),
                _ => unreachable!(),
            };
            let id = work.schema_id();
            collect(cfg, || {
                let id_ref = with_id.then_some(id.as_slice());
                let _ = encode_message(&work.schema, &work.value, mode, id_ref).expect("enc");
            })
        }
        (true, Op::Decode, _) => {
            let warm = WarmTpack::new(work, format)?;
            collect(cfg, || {
                warm.decode(&encoded).expect("dec");
            })
        }
        (true, Op::RoundTrip, ExecPath::WarmPrepared) => {
            let mut warm = WarmTpack::new(work, format)?;
            collect(cfg, || {
                let owned = warm.encode().expect("e").to_vec();
                warm.decode(&owned).expect("d");
            })
        }
        (false, Op::Encode, _) => collect(cfg, || {
            let _ = encode(work, format, ExecPath::WarmPrepared).expect("e");
        }),
        (false, Op::Decode, _) => collect(cfg, || {
            decode_competitor(&encoded, format).expect("d");
        }),
        _ => {
            return Err(Error::Msg(format!(
                "unsupported latency combo {} / {} / {}",
                format.label(),
                op.label(),
                path.label()
            )));
        }
    };

    Ok(LatencySample {
        case: work.name.clone(),
        format,
        op,
        path,
        stats,
        bytes_per_op,
    })
}

// --- alloc (observer supplied by example / global allocator) -----------------

pub fn run_alloc_cases(
    work: &Workload,
    mut observe: impl FnMut(&str, &mut dyn FnMut()) -> (usize, usize),
) -> Result<Vec<AllocSample>, Error> {
    let mut rows = Vec::new();

    // warm schemaref encode
    {
        let mut warm = WarmTpack::new(work, Format::TpackSchemaRef)?;
        let _ = warm.encode_vec()?;
        let mut out = 0usize;
        let (allocations, bytes_allocated) = observe("schemaref-encode-warm", &mut || {
            out = warm.encode().expect("e").len();
        });
        rows.push(AllocSample {
            case: work.name.clone(),
            format: Format::TpackSchemaRef,
            op: Op::Encode,
            path: ExecPath::WarmPrepared,
            allocations,
            bytes_allocated,
            out_bytes: out,
        });
    }

    // naive fullschema
    {
        let mut out = 0usize;
        let (allocations, bytes_allocated) = observe("fullschema-naive", &mut || {
            let b = encode_message(&work.schema, &work.value, EnvelopeMode::FullSchema, None)
                .expect("e");
            out = b.len();
            std::hint::black_box(b);
        });
        rows.push(AllocSample {
            case: work.name.clone(),
            format: Format::TpackFullSchema,
            op: Op::Encode,
            path: ExecPath::NaiveMessage,
            allocations,
            bytes_allocated,
            out_bytes: out,
        });
    }

    // warm fullschema prepared
    {
        let mut warm = WarmTpack::new(work, Format::TpackFullSchema)?;
        let _ = warm.encode_vec()?;
        let mut out = 0usize;
        let (allocations, bytes_allocated) = observe("fullschema-warm", &mut || {
            out = warm.encode().expect("e").len();
        });
        rows.push(AllocSample {
            case: work.name.clone(),
            format: Format::TpackFullSchema,
            op: Op::Encode,
            path: ExecPath::WarmPrepared,
            allocations,
            bytes_allocated,
            out_bytes: out,
        });
    }

    // schemaref decode
    {
        let encoded = encode(work, Format::TpackSchemaRef, ExecPath::WarmPrepared)?;
        let warm = WarmTpack::new(work, Format::TpackSchemaRef)?;
        let (allocations, bytes_allocated) = observe("schemaref-decode", &mut || {
            warm.decode(&encoded).expect("d");
        });
        rows.push(AllocSample {
            case: work.name.clone(),
            format: Format::TpackSchemaRef,
            op: Op::Decode,
            path: ExecPath::WarmPrepared,
            allocations,
            bytes_allocated,
            out_bytes: encoded.len(),
        });
    }

    // json encode
    {
        let mut out = 0usize;
        let (allocations, bytes_allocated) = observe("json-encode", &mut || {
            let b = encode(work, Format::Json, ExecPath::WarmPrepared).expect("j");
            out = b.len();
            std::hint::black_box(b);
        });
        rows.push(AllocSample {
            case: work.name.clone(),
            format: Format::Json,
            op: Op::Encode,
            path: ExecPath::NaiveMessage,
            allocations,
            bytes_allocated,
            out_bytes: out,
        });
    }

    Ok(rows)
}

// --- concurrent --------------------------------------------------------------

pub fn concurrent_schemaref_decode(
    work: &Workload,
    threads: usize,
    msgs_per_thread: usize,
    use_shared_registry: bool,
) -> Result<ConcurrentSample, Error> {
    let encoded = Arc::new(encode(
        work,
        Format::TpackSchemaRef,
        ExecPath::WarmPrepared,
    )?);
    let threads = threads.max(1);
    let per = msgs_per_thread.max(1);

    let wall = if use_shared_registry {
        let (reg, _, _) = shared_registry(work);
        let t0 = Instant::now();
        thread::scope(|scope| {
            for _ in 0..threads {
                let reg = Arc::clone(&reg);
                let encoded = Arc::clone(&encoded);
                scope.spawn(move || {
                    for _ in 0..per {
                        let mut d = Decoder::new(encoded.as_slice());
                        d.decode_message_with_registry(reg.as_ref()).expect("d");
                    }
                });
            }
        });
        t0.elapsed()
    } else {
        let schema = work.schema.clone();
        let t0 = Instant::now();
        thread::scope(|scope| {
            for _ in 0..threads {
                let schema = schema.clone();
                let encoded = Arc::clone(&encoded);
                scope.spawn(move || {
                    let reg = tpack::StdSchemaRegistry::new();
                    let id = recommended_id(&schema);
                    reg.insert(id, schema).expect("i");
                    for _ in 0..per {
                        let mut d = Decoder::new(encoded.as_slice());
                        d.decode_message_with_registry(&reg).expect("d");
                    }
                });
            }
        });
        t0.elapsed()
    };

    let total = threads * per;
    Ok(ConcurrentSample {
        label: if use_shared_registry {
            "shared-registry".into()
        } else {
            "per-thread-registry".into()
        },
        threads,
        total_messages: total,
        wall,
        msgs_per_s: total as f64 / wall.as_secs_f64().max(1e-12),
        note: if use_shared_registry {
            "shared StdSchemaRegistry (RwLock)".into()
        } else {
            "per-thread registry (no shared lock)".into()
        },
    })
}

fn recommended_id(schema: &tpack::Schema) -> [u8; 8] {
    tpack::recommended_schema_id_xxh64_v1(schema).expect("id")
}

// --- speedup -----------------------------------------------------------------

pub fn prepared_speedup(work: &Workload, iters: u32) -> Result<SpeedupSample, Error> {
    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = encode_message(&work.schema, &work.value, EnvelopeMode::FullSchema, None)?;
    }
    let naive = t0.elapsed().as_secs_f64();

    let prepared = PreparedSchema::prepare_default(work.schema.clone())?;
    let mut enc = Encoder::new();
    let t1 = Instant::now();
    for _ in 0..iters {
        enc.clear();
        enc.encode_prepared_message(&prepared, &work.value, EnvelopeMode::FullSchema, None)?;
        std::hint::black_box(enc.as_slice());
    }
    let prep = t1.elapsed().as_secs_f64();

    let mut warm = WarmTpack::new(work, Format::TpackSchemaRef)?;
    let t2 = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(warm.encode()?);
    }
    let sref = t2.elapsed().as_secs_f64();

    let inv = 1e9 / f64::from(iters);
    Ok(SpeedupSample {
        naive_ns: naive * inv,
        prepared_ns: prep * inv,
        schemaref_ns: sref * inv,
    })
}
