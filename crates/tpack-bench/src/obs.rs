//! Uniform observations: the only thing the suite produces and the view consumes.
//!
//! Kay: one message shape. Tables are just different projections.

use std::collections::BTreeMap;
use std::time::Duration;

/// Dimension bag (sorted keys for stable grouping).
pub type Dims = BTreeMap<String, String>;

pub fn dims(pairs: &[(&str, &str)]) -> Dims {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// What was measured — a sum type, not parallel report fields.
#[derive(Debug, Clone)]
pub enum Quantity {
    Bytes(usize),
    /// header, schema_id, schema, data
    Parts {
        total: usize,
        header: usize,
        schema_id: usize,
        schema: usize,
        data: usize,
    },
    /// Amortized stream totals for one n.
    Stream {
        n: usize,
        tpack: usize,
        json: usize,
        cbor: usize,
    },
    Latency {
        n: usize,
        p50: u64,
        p90: u64,
        p99: u64,
        max: u64,
        mean: u64,
        bytes_per_op: usize,
    },
    Alloc {
        allocations: usize,
        bytes_allocated: usize,
        out_bytes: usize,
    },
    Concurrent {
        threads: usize,
        total_messages: usize,
        wall: Duration,
        msgs_per_s: f64,
    },
    Speedup {
        naive_ns: f64,
        prepared_ns: f64,
        schemaref_ns: f64,
    },
}

#[derive(Debug, Clone)]
pub struct Observation {
    /// Section key for the view (e.g. "size.steady", "latency").
    pub section: &'static str,
    pub dims: Dims,
    pub quantity: Quantity,
}

impl Observation {
    pub fn new(section: &'static str, dims: Dims, quantity: Quantity) -> Self {
        Self {
            section,
            dims,
            quantity,
        }
    }

    pub fn dim(&self, key: &str) -> &str {
        self.dims.get(key).map(String::as_str).unwrap_or("")
    }
}

/// Entire run: host label + flat observation list.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    pub host: Option<String>,
    pub items: Vec<Observation>,
}

impl Catalog {
    pub fn push(&mut self, o: Observation) {
        self.items.push(o);
    }

    pub fn extend<I: IntoIterator<Item = Observation>>(&mut self, it: I) {
        self.items.extend(it);
    }

    pub fn in_section<'a>(
        &'a self,
        section: &'static str,
    ) -> impl Iterator<Item = &'a Observation> {
        self.items.iter().filter(move |o| o.section == section)
    }
}

pub fn ratio(num: usize, den: usize) -> String {
    if den == 0 {
        "n/a".into()
    } else {
        format!("{:.2}×", num as f64 / den as f64)
    }
}

pub fn latency_stats(mut xs: Vec<u64>) -> (usize, u64, u64, u64, u64, u64) {
    assert!(!xs.is_empty());
    xs.sort_unstable();
    let n = xs.len();
    let sum: u128 = xs.iter().map(|&x| u128::from(x)).sum();
    let rank = |pct: u8| {
        let r = ((f64::from(pct) / 100.0) * n as f64).ceil() as usize;
        xs[r.saturating_sub(1).min(n - 1)]
    };
    (
        n,
        rank(50),
        rank(90),
        rank(99),
        xs[n - 1],
        (sum / n as u128) as u64,
    )
}

pub fn mib_s(bytes: usize, ns: u64) -> f64 {
    if ns == 0 {
        0.0
    } else {
        (bytes as f64 / (1024.0 * 1024.0)) / (ns as f64 / 1e9)
    }
}
