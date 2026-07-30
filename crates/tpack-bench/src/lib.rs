//! Shared payloads and encoders for TPACK size and throughput benchmarks.
//!
//! Competitor formats (JSON / CBOR / MessagePack) encode *semantic twins* of
//! the TPACK values: they use ordinary `serde` types (`String`, `i64`, `f64`,
//! nested structs) rather than TPACK's native `Decimal` / `DecimalFixed`
//! model. Size comparisons are therefore fair for wire footprint of equivalent
//! application data, not a claim of identical type systems.

use std::borrow::Cow;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tpack::{
    Decimal, Decoder, EnvelopeMode, Field, Schema, StdSchemaRegistry, TpackValue, TypeDescriptor,
    encode_message, recommended_schema_id_xxh64_v1,
};

/// Named benchmark scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scenario {
    /// Draft flat-record example (`id`, `price`, `tax`, `qty`, `ts`).
    FlatRecord,
    /// Small order with three nested line items.
    NestedStruct,
    /// List of [`BULK_LIST_LEN`] flat-like items (amortized schema cost).
    BulkList,
}

impl Scenario {
    pub const ALL: [Scenario; 3] = [
        Scenario::FlatRecord,
        Scenario::NestedStruct,
        Scenario::BulkList,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Scenario::FlatRecord => "flat_record",
            Scenario::NestedStruct => "nested_struct",
            Scenario::BulkList => "bulk_list",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Scenario::FlatRecord => {
                "Draft flat-record example: id, DecimalFixed price, Decimal tax, qty, ts"
            }
            Scenario::NestedStruct => {
                "Order header plus three line items (sku, qty, unit_price) to stress key/schema overhead"
            }
            Scenario::BulkList => "List of 100 flat-like product rows (amortized FullSchema cost)",
        }
    }
}

/// Number of elements in the [`Scenario::BulkList`] payload.
pub const BULK_LIST_LEN: usize = 100;

/// Wire formats measured by the harness.
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
    pub const ALL: [Format; 6] = [
        Format::TpackFullSchema,
        Format::TpackFullSchemaWithId,
        Format::TpackSchemaRef,
        Format::Json,
        Format::Cbor,
        Format::MsgPack,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Format::TpackFullSchema => "tpack-fullschema",
            Format::TpackFullSchemaWithId => "tpack-fullschema-with-id",
            Format::TpackSchemaRef => "tpack-schemaref",
            Format::Json => "json",
            Format::Cbor => "cbor",
            Format::MsgPack => "msgpack",
        }
    }

    pub fn is_tpack(self) -> bool {
        matches!(
            self,
            Format::TpackFullSchema | Format::TpackFullSchemaWithId | Format::TpackSchemaRef
        )
    }
}

/// One measured encode size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeSample {
    pub scenario: Scenario,
    pub format: Format,
    pub bytes: usize,
}

// ---------------------------------------------------------------------------
// Serde semantic twins (JSON / CBOR / MessagePack)
// ---------------------------------------------------------------------------

/// Competitor twin of the draft flat-record example.
///
/// `price` is the unscaled `DecimalFixed` coefficient (scale 4). `tax` is a
/// decimal string so JSON/CBOR/MessagePack do not need a decimal type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlatRecordSerde {
    pub id: String,
    pub price: i64,
    pub tax: String,
    pub qty: i32,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineItemSerde {
    pub sku: String,
    pub qty: i32,
    pub unit_price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NestedOrderSerde {
    pub order_id: String,
    pub customer: String,
    pub items: Vec<LineItemSerde>,
}

// ---------------------------------------------------------------------------
// Payload construction
// ---------------------------------------------------------------------------

/// Build the TPACK schema for a scenario.
pub fn tpack_schema(scenario: Scenario) -> Schema {
    match scenario {
        Scenario::FlatRecord => flat_record_schema(),
        Scenario::NestedStruct => nested_struct_schema(),
        Scenario::BulkList => bulk_list_schema(),
    }
}

/// Build the TPACK value for a scenario (owned strings via `'static` borrows
/// where possible; bulk list allocates).
pub fn tpack_value(scenario: Scenario) -> TpackValue<'static> {
    match scenario {
        Scenario::FlatRecord => flat_record_value(),
        Scenario::NestedStruct => nested_struct_value(),
        Scenario::BulkList => bulk_list_value(),
    }
}

/// Recommended `xxh64-v1` schema id for a scenario (fixed 8-byte BE digest).
pub fn schema_id_bytes(scenario: Scenario) -> [u8; 8] {
    recommended_schema_id_xxh64_v1(&tpack_schema(scenario)).expect("scenario schema is valid")
}

/// Serde twin for JSON / CBOR / MessagePack.
pub fn serde_payload(scenario: Scenario) -> SerdePayload {
    match scenario {
        Scenario::FlatRecord => SerdePayload::Flat(flat_record_serde()),
        Scenario::NestedStruct => SerdePayload::Nested(nested_struct_serde()),
        Scenario::BulkList => SerdePayload::Bulk(bulk_list_serde()),
    }
}

/// Enum wrapper so encode helpers share one entry point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SerdePayload {
    Flat(FlatRecordSerde),
    Nested(NestedOrderSerde),
    Bulk(Vec<FlatRecordSerde>),
}

fn flat_record_schema() -> Schema {
    Schema::new(TypeDescriptor::Struct(vec![
        Field::new(1, "id", TypeDescriptor::String { max_len: Some(64) }),
        Field::new(
            2,
            "price",
            TypeDescriptor::DecimalFixed {
                precision: 18,
                scale: 4,
            },
        ),
        Field::new(3, "tax", TypeDescriptor::Decimal),
        Field::new(4, "qty", TypeDescriptor::I32),
        Field::new(5, "ts", TypeDescriptor::I64),
    ]))
}

fn flat_record_value() -> TpackValue<'static> {
    TpackValue::Struct(vec![
        (1, TpackValue::String(Cow::Borrowed("prod_001"))),
        (2, TpackValue::DecimalFixed(2_999_900)),
        (
            3,
            TpackValue::Decimal(Decimal {
                scale: 3,
                coefficient: 13_725,
            }),
        ),
        (4, TpackValue::I32(10)),
        (5, TpackValue::I64(1_715_000_000)),
    ])
}

fn flat_record_serde() -> FlatRecordSerde {
    FlatRecordSerde {
        id: "prod_001".to_string(),
        // DecimalFixed scale 4 coefficient 2_999_900 → application value 299.9900
        price: 2_999_900,
        // Decimal scale 3 coefficient 13_725 → "13.725"
        tax: "13.725".to_string(),
        qty: 10,
        ts: 1_715_000_000,
    }
}

fn line_item_fields() -> TypeDescriptor {
    TypeDescriptor::Struct(vec![
        Field::new(1, "sku", TypeDescriptor::String { max_len: Some(32) }),
        Field::new(2, "qty", TypeDescriptor::I32),
        Field::new(
            3,
            "unit_price",
            TypeDescriptor::DecimalFixed {
                precision: 18,
                scale: 4,
            },
        ),
    ])
}

fn nested_struct_schema() -> Schema {
    Schema::new(TypeDescriptor::Struct(vec![
        Field::new(1, "order_id", TypeDescriptor::String { max_len: Some(32) }),
        Field::new(2, "customer", TypeDescriptor::String { max_len: Some(64) }),
        Field::new(
            3,
            "items",
            TypeDescriptor::List {
                max_count: Some(64),
                element: Box::new(line_item_fields()),
            },
        ),
    ]))
}

fn nested_struct_value() -> TpackValue<'static> {
    TpackValue::Struct(vec![
        (1, TpackValue::String(Cow::Borrowed("ord-10042"))),
        (2, TpackValue::String(Cow::Borrowed("alice@example.com"))),
        (
            3,
            TpackValue::List(vec![
                TpackValue::Struct(vec![
                    (1, TpackValue::String(Cow::Borrowed("SKU-A"))),
                    (2, TpackValue::I32(2)),
                    (3, TpackValue::DecimalFixed(1_250_000)),
                ]),
                TpackValue::Struct(vec![
                    (1, TpackValue::String(Cow::Borrowed("SKU-B"))),
                    (2, TpackValue::I32(1)),
                    (3, TpackValue::DecimalFixed(499_900)),
                ]),
                TpackValue::Struct(vec![
                    (1, TpackValue::String(Cow::Borrowed("SKU-C"))),
                    (2, TpackValue::I32(5)),
                    (3, TpackValue::DecimalFixed(99_500)),
                ]),
            ]),
        ),
    ])
}

fn nested_struct_serde() -> NestedOrderSerde {
    NestedOrderSerde {
        order_id: "ord-10042".to_string(),
        customer: "alice@example.com".to_string(),
        items: vec![
            LineItemSerde {
                sku: "SKU-A".to_string(),
                qty: 2,
                unit_price: 1_250_000,
            },
            LineItemSerde {
                sku: "SKU-B".to_string(),
                qty: 1,
                unit_price: 499_900,
            },
            LineItemSerde {
                sku: "SKU-C".to_string(),
                qty: 5,
                unit_price: 99_500,
            },
        ],
    }
}

fn bulk_list_schema() -> Schema {
    Schema::new(TypeDescriptor::List {
        max_count: Some(BULK_LIST_LEN as u64),
        element: Box::new(flat_record_schema().root),
    })
}

fn bulk_list_value() -> TpackValue<'static> {
    let mut items = Vec::with_capacity(BULK_LIST_LEN);
    for i in 0..BULK_LIST_LEN {
        let id = format!("prod_{i:03}");
        items.push(TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Owned(id))),
            (2, TpackValue::DecimalFixed(2_999_900 + i as i64)),
            (
                3,
                TpackValue::Decimal(Decimal {
                    scale: 3,
                    coefficient: 13_725 + i as i64,
                }),
            ),
            (4, TpackValue::I32(10 + (i as i32 % 7))),
            (5, TpackValue::I64(1_715_000_000 + i as i64)),
        ]));
    }
    TpackValue::List(items)
}

fn bulk_list_serde() -> Vec<FlatRecordSerde> {
    (0..BULK_LIST_LEN)
        .map(|i| FlatRecordSerde {
            id: format!("prod_{i:03}"),
            price: 2_999_900 + i as i64,
            tax: format!("{:.3}", (13_725 + i as i64) as f64 / 1000.0),
            qty: 10 + (i as i32 % 7),
            ts: 1_715_000_000 + i as i64,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Encode a scenario in the given format. Returns owned wire bytes.
pub fn encode(scenario: Scenario, format: Format) -> Result<Vec<u8>, EncodeError> {
    match format {
        Format::TpackFullSchema => encode_tpack(scenario, EnvelopeMode::FullSchema, false),
        Format::TpackFullSchemaWithId => {
            encode_tpack(scenario, EnvelopeMode::FullSchemaWithId, true)
        }
        Format::TpackSchemaRef => encode_tpack(scenario, EnvelopeMode::SchemaRef, true),
        Format::Json => {
            let payload = serde_payload(scenario);
            serde_json::to_vec(&payload).map_err(EncodeError::Json)
        }
        Format::Cbor => {
            let payload = serde_payload(scenario);
            let mut buf = Vec::new();
            ciborium::into_writer(&payload, &mut buf)
                .map_err(|e| EncodeError::Cbor(e.to_string()))?;
            Ok(buf)
        }
        Format::MsgPack => {
            let payload = serde_payload(scenario);
            rmp_serde::to_vec_named(&payload).map_err(EncodeError::MsgPack)
        }
    }
}

fn encode_tpack(
    scenario: Scenario,
    mode: EnvelopeMode,
    with_id: bool,
) -> Result<Vec<u8>, EncodeError> {
    let schema = tpack_schema(scenario);
    let value = tpack_value(scenario);
    let id = if with_id {
        Some(schema_id_bytes(scenario))
    } else {
        None
    };
    let id_ref = id.as_ref().map(|b| b.as_slice());
    encode_message(&schema, &value, mode, id_ref).map_err(EncodeError::Tpack)
}

/// Decode previously encoded bytes for a format.
///
/// For `tpack-schemaref`, the registry is preloaded with the scenario schema.
/// For competitor formats, decode returns an opaque success (type-erased length
/// check via re-parse into `SerdePayload`).
pub fn decode(scenario: Scenario, format: Format, bytes: &[u8]) -> Result<(), EncodeError> {
    match format {
        Format::TpackFullSchema | Format::TpackFullSchemaWithId => {
            let mut decoder = Decoder::new(bytes);
            decoder.decode_message().map_err(EncodeError::Tpack)?;
            Ok(())
        }
        Format::TpackSchemaRef => {
            let schema = tpack_schema(scenario);
            let id = schema_id_bytes(scenario);
            let registry = StdSchemaRegistry::new();
            registry
                .insert(id, schema)
                .expect("fresh registry has no conflicts");
            let mut decoder = Decoder::new(bytes);
            decoder
                .decode_message_with_registry(&registry)
                .map_err(EncodeError::Tpack)?;
            Ok(())
        }
        Format::Json => {
            let _: SerdePayload = serde_json::from_slice(bytes).map_err(EncodeError::Json)?;
            Ok(())
        }
        Format::Cbor => {
            let _: SerdePayload =
                ciborium::from_reader(bytes).map_err(|e| EncodeError::Cbor(e.to_string()))?;
            Ok(())
        }
        Format::MsgPack => {
            let _: SerdePayload =
                rmp_serde::from_slice(bytes).map_err(EncodeError::MsgPackDecode)?;
            Ok(())
        }
    }
}

/// Preload a registry for SchemaRef decode benches (shared setup cost).
pub fn preload_registry(scenario: Scenario) -> (StdSchemaRegistry, Arc<Schema>, [u8; 8]) {
    let schema = Arc::new(tpack_schema(scenario));
    let id = schema_id_bytes(scenario);
    let registry = StdSchemaRegistry::new();
    registry
        .insert_shared(id, Arc::clone(&schema))
        .expect("fresh registry has no conflicts");
    (registry, schema, id)
}

/// Errors from the harness encoders (not part of the public TPACK API).
#[derive(Debug)]
pub enum EncodeError {
    Tpack(tpack::Error),
    Json(serde_json::Error),
    Cbor(String),
    MsgPack(rmp_serde::encode::Error),
    MsgPackDecode(rmp_serde::decode::Error),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::Tpack(e) => write!(f, "tpack: {e}"),
            EncodeError::Json(e) => write!(f, "json: {e}"),
            EncodeError::Cbor(e) => write!(f, "cbor: {e}"),
            EncodeError::MsgPack(e) => write!(f, "msgpack: {e}"),
            EncodeError::MsgPackDecode(e) => write!(f, "msgpack decode: {e}"),
        }
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EncodeError::Tpack(e) => Some(e),
            EncodeError::Json(e) => Some(e),
            EncodeError::Cbor(_) => None,
            EncodeError::MsgPack(e) => Some(e),
            EncodeError::MsgPackDecode(e) => Some(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Size measurement + markdown
// ---------------------------------------------------------------------------

/// Measure encoded sizes for every scenario × format pair.
pub fn measure_all_sizes() -> Result<Vec<SizeSample>, EncodeError> {
    let mut out = Vec::with_capacity(Scenario::ALL.len() * Format::ALL.len());
    for scenario in Scenario::ALL {
        for format in Format::ALL {
            let bytes = encode(scenario, format)?;
            out.push(SizeSample {
                scenario,
                format,
                bytes: bytes.len(),
            });
        }
    }
    Ok(out)
}

/// Render the checked-in size comparison markdown document.
pub fn render_benchmarks_markdown(samples: &[SizeSample]) -> String {
    let mut md = String::new();
    md.push_str("# TPACK size & throughput benchmarks\n\n");
    md.push_str("This document is **generated** by the `tpack-bench` harness.\n\n");
    md.push_str("Regenerate the size table with:\n\n");
    md.push_str("```bash\n");
    md.push_str("cargo run -p tpack-bench --example size_report\n");
    md.push_str("```\n\n");
    md.push_str("Sizes are deterministic (same on every machine). Throughput numbers\n");
    md.push_str("depend on hardware and are **not** checked into this table as absolute\n");
    md.push_str("truth; see [Example throughput](#example-throughput-informational).\n\n");

    md.push_str("## Methodology\n\n");
    md.push_str("### Formats\n\n");
    md.push_str("| Label | What is measured |\n");
    md.push_str("|-------|------------------|\n");
    md.push_str("| `tpack-fullschema` | `EnvelopeMode::FullSchema` (schema + data) |\n");
    md.push_str("| `tpack-fullschema-with-id` | `FullSchemaWithId` + official `xxh64-v1` 8-byte schema id |\n");
    md.push_str(
        "| `tpack-schemaref` | `SchemaRef` (id + data only; decode uses a preloaded registry) |\n",
    );
    md.push_str("| `json` | Compact `serde_json` (no pretty-print) |\n");
    md.push_str("| `cbor` | `ciborium` (CBOR via serde) |\n");
    md.push_str("| `msgpack` | `rmp-serde` named maps |\n\n");

    md.push_str("### Semantic twins\n\n");
    md.push_str("JSON / CBOR / MessagePack do not share TPACK's native type system.\n");
    md.push_str("Competitor payloads are *semantic twins*:\n\n");
    md.push_str("- String and integer fields map directly.\n");
    md.push_str(
        "- `DecimalFixed` becomes an `i64` unscaled coefficient (scale is schema-side in TPACK).\n",
    );
    md.push_str("- `Decimal` becomes a decimal **string** (e.g. `\"13.725\"`) so competitors are not forced into binary floats.\n");
    md.push_str("- Nested structs and lists use ordinary serde structs / `Vec`.\n\n");
    md.push_str(
        "TPACK numbers therefore include typed schema descriptors; competitor numbers include\n",
    );
    md.push_str("repeated field names (JSON/MessagePack maps) or CBOR map keys. Neither side is\n");
    md.push_str("claimed to \"win\" every column.\n\n");

    md.push_str("### Scenarios\n\n");
    for scenario in Scenario::ALL {
        md.push_str(&format!(
            "- **`{}`**: {}\n",
            scenario.name(),
            scenario.description()
        ));
    }
    md.push('\n');

    md.push_str("## Size comparison\n\n");
    md.push_str("| Scenario | Format | Bytes | vs JSON |\n");
    md.push_str("|----------|--------|------:|--------:|\n");

    for scenario in Scenario::ALL {
        let json_bytes = samples
            .iter()
            .find(|s| s.scenario == scenario && s.format == Format::Json)
            .map(|s| s.bytes)
            .unwrap_or(1);
        for format in Format::ALL {
            let bytes = samples
                .iter()
                .find(|s| s.scenario == scenario && s.format == format)
                .map(|s| s.bytes)
                .unwrap_or(0);
            let vs = if format == Format::Json {
                "1.00×".to_string()
            } else {
                format!("{:.2}×", bytes as f64 / json_bytes as f64)
            };
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} |\n",
                scenario.name(),
                format.label(),
                bytes,
                vs
            ));
        }
    }
    md.push('\n');
    md.push_str(
        "`vs JSON` is `format_bytes / json_bytes` for the same scenario (lower is smaller).\n\n",
    );

    md.push_str("## Example throughput (informational)\n\n");
    md.push_str(
        "Throughput is measured with [Criterion](https://github.com/bheisler/criterion.rs).\n",
    );
    md.push_str(
        "**Do not** treat absolute ns/iter numbers as portable; re-run on your hardware.\n\n",
    );
    md.push_str("```bash\n");
    md.push_str("# Full Criterion run (writes HTML under target/criterion/)\n");
    md.push_str("cargo bench -p tpack-bench\n");
    md.push('\n');
    md.push_str("# Faster smoke run\n");
    md.push_str("cargo bench -p tpack-bench -- --quick\n");
    md.push_str("```\n\n");
    md.push_str("Benches cover encode and decode for each scenario × format pair.\n");
    md.push_str("`tpack-schemaref` decode uses a registry preloaded outside the timed loop.\n\n");
    md.push_str("| Environment | Notes |\n");
    md.push_str("|-------------|-------|\n");
    md.push_str("| CPU | *run locally / on CI runner; fill in when publishing numbers* |\n");
    md.push_str("| Command | `cargo bench -p tpack-bench` |\n");
    md.push_str("| CI policy | Full Criterion is **not** part of default CI |\n\n");

    md.push_str("## Golden size guard\n\n");
    md.push_str(
        "Unit tests in `tpack-bench` pin the `flat_record` × `tpack-fullschema` size to the\n",
    );
    md.push_str(
        "draft-00 test vector length so accidental encoding drift fails CI without running\n",
    );
    md.push_str("multi-minute benches.\n");

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draft-00 flat-record FullSchema vector is 76 bytes (header + envelope + schema + data).
    const FLAT_RECORD_FULLSCHEMA_BYTES: usize = 76;

    #[test]
    fn flat_record_fullschema_matches_golden_size() {
        let bytes = encode(Scenario::FlatRecord, Format::TpackFullSchema).expect("encode");
        assert_eq!(
            bytes.len(),
            FLAT_RECORD_FULLSCHEMA_BYTES,
            "flat_record FullSchema size drifted; update golden only with intentional wire changes"
        );
    }

    #[test]
    fn all_sizes_positive_and_deterministic() {
        let a = measure_all_sizes().expect("measure");
        let b = measure_all_sizes().expect("measure again");
        assert_eq!(a, b);
        for sample in &a {
            assert!(
                sample.bytes > 0,
                "{} / {} produced empty payload",
                sample.scenario.name(),
                sample.format.label()
            );
        }
    }

    #[test]
    fn schemaref_smaller_than_fullschema_on_bulk() {
        let full = encode(Scenario::BulkList, Format::TpackFullSchema)
            .expect("full")
            .len();
        let sref = encode(Scenario::BulkList, Format::TpackSchemaRef)
            .expect("ref")
            .len();
        assert!(
            sref < full,
            "SchemaRef ({sref}) should beat FullSchema ({full}) when schema is large relative to data amortisation"
        );
    }

    #[test]
    fn roundtrip_all_formats_flat_record() {
        for format in Format::ALL {
            let bytes = encode(Scenario::FlatRecord, format).expect("encode");
            decode(Scenario::FlatRecord, format, &bytes).unwrap_or_else(|e| {
                panic!("decode {} failed: {e}", format.label());
            });
        }
    }

    #[test]
    fn markdown_contains_size_table_header() {
        let samples = measure_all_sizes().expect("measure");
        let md = render_benchmarks_markdown(&samples);
        assert!(md.contains("| Scenario | Format | Bytes | vs JSON |"));
        assert!(md.contains("tpack-fullschema"));
        assert!(md.contains("flat_record"));
        assert!(md.contains("nested_struct"));
        assert!(md.contains("bulk_list"));
    }
}
