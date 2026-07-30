//! Shared vocabulary: formats, paths, and structured samples.
//!
//! Everything the harness *observes* becomes plain data here.
//! Rendering and CLI only consume these types.

use std::time::Duration;

/// Wire formats under comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    TpackFullSchema,
    TpackFullSchemaWithId,
    TpackSchemaRef,
    Json,
    Cbor,
    MsgPack,
}

impl Format {
    pub const ALL: [Self; 6] = [
        Self::TpackFullSchema,
        Self::TpackFullSchemaWithId,
        Self::TpackSchemaRef,
        Self::Json,
        Self::Cbor,
        Self::MsgPack,
    ];

    pub const STEADY: [Self; 4] = [Self::TpackSchemaRef, Self::Json, Self::Cbor, Self::MsgPack];

    pub const COLD_TPACK: [Self; 2] = [Self::TpackFullSchema, Self::TpackFullSchemaWithId];

    pub fn label(self) -> &'static str {
        match self {
            Self::TpackFullSchema => "tpack-fullschema",
            Self::TpackFullSchemaWithId => "tpack-fullschema-with-id",
            Self::TpackSchemaRef => "tpack-schemaref",
            Self::Json => "json",
            Self::Cbor => "cbor",
            Self::MsgPack => "msgpack",
        }
    }

    pub fn is_tpack(self) -> bool {
        matches!(
            self,
            Self::TpackFullSchema | Self::TpackFullSchemaWithId | Self::TpackSchemaRef
        )
    }
}

/// How an operation is executed (must never be collapsed in reports).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecPath {
    /// `PreparedSchema` + reused `Encoder` / preloaded registry.
    WarmPrepared,
    /// One-shot `encode_message` (re-encodes schema for FullSchema modes).
    NaiveMessage,
}

impl ExecPath {
    pub fn label(self) -> &'static str {
        match self {
            Self::WarmPrepared => "warm-prepared",
            Self::NaiveMessage => "naive-message",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    Encode,
    Decode,
    RoundTrip,
}

impl Op {
    pub fn label(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Decode => "decode",
            Self::RoundTrip => "roundtrip",
        }
    }
}

/// One encoded size observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeSample {
    pub case: String,
    pub format: Format,
    pub bytes: usize,
}

/// TPACK envelope anatomy (portable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breakdown {
    pub case: String,
    pub format: Format,
    pub total: usize,
    pub header: usize,
    pub schema_id: usize,
    pub schema: usize,
    pub data: usize,
}

/// Amortized stream: 1× cold FullSchemaWithId + (n−1)× SchemaRef vs full-price competitors.
#[derive(Debug, Clone, PartialEq)]
pub struct AmortizedSample {
    pub case: String,
    pub n: usize,
    pub tpack_total: usize,
    pub json_total: usize,
    pub cbor_total: usize,
}

/// Latency distribution in nanoseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatencyStats {
    pub n: usize,
    pub p50: u64,
    pub p90: u64,
    pub p99: u64,
    pub max: u64,
    pub mean: u64,
}

impl LatencyStats {
    pub fn from_sorted(mut xs: Vec<u64>) -> Self {
        assert!(!xs.is_empty());
        xs.sort_unstable();
        let n = xs.len();
        let sum: u128 = xs.iter().map(|&v| u128::from(v)).sum();
        Self {
            n,
            p50: nearest_rank(&xs, 50),
            p90: nearest_rank(&xs, 90),
            p99: nearest_rank(&xs, 99),
            max: xs[n - 1],
            mean: (sum / n as u128) as u64,
        }
    }
}

fn nearest_rank(sorted: &[u64], pct: u8) -> u64 {
    let n = sorted.len();
    let rank = ((f64::from(pct) / 100.0) * n as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(n - 1)]
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatencySample {
    pub case: String,
    pub format: Format,
    pub op: Op,
    pub path: ExecPath,
    pub stats: LatencyStats,
    pub bytes_per_op: usize,
}

impl LatencySample {
    pub fn mib_s_at_p50(&self) -> f64 {
        if self.stats.p50 == 0 {
            return 0.0;
        }
        (self.bytes_per_op as f64 / (1024.0 * 1024.0)) / (self.stats.p50 as f64 / 1e9)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocSample {
    pub case: String,
    pub format: Format,
    pub op: Op,
    pub path: ExecPath,
    pub allocations: usize,
    pub bytes_allocated: usize,
    pub out_bytes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConcurrentSample {
    pub label: String,
    pub threads: usize,
    pub total_messages: usize,
    pub wall: Duration,
    pub msgs_per_s: f64,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpeedupSample {
    pub naive_ns: f64,
    pub prepared_ns: f64,
    pub schemaref_ns: f64,
}

/// Complete report object — pure data, no markdown.
#[derive(Debug, Clone)]
pub struct Report {
    pub host: Option<String>,
    pub sizes: Vec<SizeSample>,
    pub breakdowns: Vec<Breakdown>,
    pub amortized: Vec<AmortizedSample>,
    pub scale_blob: Vec<SizeSample>,
    pub scale_list: Vec<SizeSample>,
    pub scale_fields: Vec<SizeSample>,
    pub latency: Vec<LatencySample>,
    pub allocs: Vec<AllocSample>,
    pub concurrent: Vec<ConcurrentSample>,
    pub speedup: Option<SpeedupSample>,
}

impl Report {
    pub fn empty() -> Self {
        Self {
            host: None,
            sizes: Vec::new(),
            breakdowns: Vec::new(),
            amortized: Vec::new(),
            scale_blob: Vec::new(),
            scale_list: Vec::new(),
            scale_fields: Vec::new(),
            latency: Vec::new(),
            allocs: Vec::new(),
            concurrent: Vec::new(),
            speedup: None,
        }
    }
}

pub fn ratio(num: usize, den: usize) -> String {
    if den == 0 {
        "n/a".into()
    } else {
        format!("{:.2}×", num as f64 / den as f64)
    }
}
