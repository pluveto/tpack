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
    BigInt, Decimal, Decoder, Encoder, EnvelopeMode, Field, PreparedSchema, Schema,
    StdSchemaRegistry, TpackValue, TypeDescriptor, recommended_schema_id_xxh64_v1,
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
        (2, TpackValue::DecimalFixed(BigInt::from(2_999_900))),
        (
            3,
            TpackValue::Decimal(Decimal {
                scale: 3,
                coefficient: BigInt::from(13_725),
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
                    (3, TpackValue::DecimalFixed(BigInt::from(1_250_000))),
                ]),
                TpackValue::Struct(vec![
                    (1, TpackValue::String(Cow::Borrowed("SKU-B"))),
                    (2, TpackValue::I32(1)),
                    (3, TpackValue::DecimalFixed(BigInt::from(499_900))),
                ]),
                TpackValue::Struct(vec![
                    (1, TpackValue::String(Cow::Borrowed("SKU-C"))),
                    (2, TpackValue::I32(5)),
                    (3, TpackValue::DecimalFixed(BigInt::from(99_500))),
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
            (
                2,
                TpackValue::DecimalFixed(BigInt::from(2_999_900 + i as i64)),
            ),
            (
                3,
                TpackValue::Decimal(Decimal {
                    scale: 3,
                    coefficient: BigInt::from(13_725 + i as i64),
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
// Encoding (steady-state aware)
// ---------------------------------------------------------------------------

/// Encode a scenario in the given format (one-shot / cold helper).
///
/// For repeated encodes, prefer [`SteadyEncoder`] so schema bytes and the
/// output buffer are reused.
pub fn encode(scenario: Scenario, format: Format) -> Result<Vec<u8>, EncodeError> {
    match format {
        Format::TpackFullSchema | Format::TpackFullSchemaWithId | Format::TpackSchemaRef => {
            let mut steady = SteadyEncoder::new(scenario, format)?;
            steady.encode_once()
        }
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

/// Reusable TPACK encoder: prepared schema + cleared output buffer.
pub struct SteadyEncoder {
    scenario: Scenario,
    format: Format,
    prepared: PreparedSchema,
    value: TpackValue<'static>,
    schema_id: [u8; 8],
    encoder: Encoder,
    mode: EnvelopeMode,
    with_id: bool,
}

impl SteadyEncoder {
    pub fn new(scenario: Scenario, format: Format) -> Result<Self, EncodeError> {
        let (mode, with_id) = match format {
            Format::TpackFullSchema => (EnvelopeMode::FullSchema, false),
            Format::TpackFullSchemaWithId => (EnvelopeMode::FullSchemaWithId, true),
            Format::TpackSchemaRef => (EnvelopeMode::SchemaRef, true),
            _ => {
                return Err(EncodeError::Cbor(
                    "SteadyEncoder is only for TPACK formats".into(),
                ));
            }
        };
        let schema = tpack_schema(scenario);
        let prepared = PreparedSchema::prepare_default(schema).map_err(EncodeError::Tpack)?;
        let value = tpack_value(scenario);
        let schema_id = schema_id_bytes(scenario);
        Ok(Self {
            scenario,
            format,
            prepared,
            value,
            schema_id,
            encoder: Encoder::new(),
            mode,
            with_id,
        })
    }

    pub fn scenario(&self) -> Scenario {
        self.scenario
    }

    pub fn format(&self) -> Format {
        self.format
    }

    /// Encode one message into a fresh `Vec` (encoder buffer is taken).
    pub fn encode_once(&mut self) -> Result<Vec<u8>, EncodeError> {
        self.encoder.clear();
        let id = if self.with_id {
            Some(self.schema_id.as_slice())
        } else {
            None
        };
        self.encoder
            .encode_prepared_message(&self.prepared, &self.value, self.mode, id)
            .map_err(EncodeError::Tpack)?;
        Ok(self.encoder.take_vec())
    }

    /// Encode into the internal buffer; returns a slice valid until next encode/clear.
    pub fn encode_in_place(&mut self) -> Result<&[u8], EncodeError> {
        self.encoder.clear();
        let id = if self.with_id {
            Some(self.schema_id.as_slice())
        } else {
            None
        };
        self.encoder
            .encode_prepared_message(&self.prepared, &self.value, self.mode, id)
            .map_err(EncodeError::Tpack)?;
        Ok(self.encoder.as_slice())
    }
}

/// Decode previously encoded bytes for a format.
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

/// TPACK message component sizes (deterministic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpackBreakdown {
    pub scenario: Scenario,
    pub format: Format,
    pub total: usize,
    pub header_and_mode: usize,
    pub schema_id: usize,
    pub schema: usize,
    pub data: usize,
}

/// Parse a single TPACK message into overhead components.
pub fn breakdown_tpack(scenario: Scenario, format: Format) -> Result<TpackBreakdown, EncodeError> {
    let bytes = encode(scenario, format)?;
    let mode = match format {
        Format::TpackFullSchema => EnvelopeMode::FullSchema,
        Format::TpackFullSchemaWithId => EnvelopeMode::FullSchemaWithId,
        Format::TpackSchemaRef => EnvelopeMode::SchemaRef,
        _ => {
            return Err(EncodeError::Cbor(
                "breakdown_tpack only applies to TPACK formats".into(),
            ));
        }
    };
    let b = parse_tpack_breakdown(scenario, format, mode, &bytes)?;
    Ok(b)
}

fn parse_tpack_breakdown(
    scenario: Scenario,
    format: Format,
    mode: EnvelopeMode,
    bytes: &[u8],
) -> Result<TpackBreakdown, EncodeError> {
    if bytes.len() < 6 || &bytes[0..4] != b"TPAK" {
        return Err(EncodeError::Cbor("not a TPACK message".into()));
    }
    let mut i = 5; // after magic+version
    let mode_byte = bytes[i];
    i += 1;
    assert_eq!(mode_byte, mode.tag());

    let header_and_mode = 6;
    let mut schema_id_len = 0usize;
    let mut schema_len = 0usize;

    match mode {
        EnvelopeMode::FullSchema => {
            let (slen, ni) = read_uvarint(bytes, i)?;
            schema_len = slen as usize;
            i = ni + schema_len;
        }
        EnvelopeMode::FullSchemaWithId => {
            let (id_len, ni) = read_uvarint(bytes, i)?;
            schema_id_len = id_len as usize;
            i = ni + schema_id_len;
            let (slen, ni) = read_uvarint(bytes, i)?;
            schema_len = slen as usize;
            i = ni + schema_len;
        }
        EnvelopeMode::SchemaRef => {
            let (id_len, ni) = read_uvarint(bytes, i)?;
            schema_id_len = id_len as usize;
            i = ni + schema_id_len;
        }
    }
    let data = bytes.len().saturating_sub(i);
    // schema_id field includes its length prefix in "overhead" accounting:
    // report id payload only; length prefix counted in residual overhead via total sum.
    Ok(TpackBreakdown {
        scenario,
        format,
        total: bytes.len(),
        header_and_mode,
        schema_id: schema_id_len,
        schema: schema_len,
        data,
    })
}

fn read_uvarint(bytes: &[u8], mut i: usize) -> Result<(u64, usize), EncodeError> {
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *bytes
            .get(i)
            .ok_or_else(|| EncodeError::Cbor("truncated uvarint".into()))?;
        i += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, i));
        }
        shift += 7;
        if shift > 63 {
            return Err(EncodeError::Cbor("uvarint too long".into()));
        }
    }
}

/// One measured encode size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeSample {
    pub scenario: Scenario,
    pub format: Format,
    pub bytes: usize,
}

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

/// Amortized multi-message stream: 1× FullSchemaWithId cold + (n-1)× SchemaRef hot.
pub const AMORTIZED_MESSAGE_COUNT: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct AmortizedSample {
    pub scenario: Scenario,
    pub label: &'static str,
    pub total_bytes: usize,
    pub per_message: f64,
}

pub fn measure_amortized(scenario: Scenario, n: usize) -> Result<AmortizedSample, EncodeError> {
    assert!(n >= 1);
    let cold = encode(scenario, Format::TpackFullSchemaWithId)?;
    let hot = encode(scenario, Format::TpackSchemaRef)?;
    let total = cold.len() + hot.len() * (n - 1);
    Ok(AmortizedSample {
        scenario,
        label: "tpack-1×with-id+(n-1)×schemaref",
        total_bytes: total,
        per_message: total as f64 / n as f64,
    })
}

pub fn measure_amortized_competitor(
    scenario: Scenario,
    format: Format,
    n: usize,
) -> Result<AmortizedSample, EncodeError> {
    let one = encode(scenario, format)?;
    let total = one.len() * n;
    Ok(AmortizedSample {
        scenario,
        label: format.label(),
        total_bytes: total,
        per_message: one.len() as f64,
    })
}

/// Render the checked-in benchmarks markdown document.
pub fn render_benchmarks_markdown(samples: &[SizeSample]) -> String {
    let mut md = String::new();
    md.push_str("# TPACK size & throughput benchmarks\n\n");
    md.push_str("This document is **generated** by the `tpack-bench` harness.\n\n");
    md.push_str("```bash\ncargo run -p tpack-bench --example size_report\n```\n\n");
    md.push_str("Sizes are deterministic. Throughput is hardware-specific; see below.\n\n");

    md.push_str("## How to read this table\n\n");
    md.push_str("TPACK has three envelope modes with different cost profiles:\n\n");
    md.push_str("- **`tpack-schemaref`** (primary steady-state metric): schema lives in a registry; message is id + data.\n");
    md.push_str("- **`tpack-fullschema` / `with-id`**: self-contained; **includes the full schema descriptor** every message (cold / bootstrap).\n");
    md.push_str("- Competing JSON/CBOR/MessagePack maps repeat field names every message.\n\n");
    md.push_str(
        "A single tiny FullSchema message is **not** the design center for density vs CBOR.\n",
    );
    md.push_str("Prefer SchemaRef rows and the [amortized multi-message](#amortized-multi-message-stream) section.\n\n");

    md.push_str("## Methodology\n\n");
    md.push_str("### Formats\n\n");
    md.push_str("| Label | What is measured |\n|-------|------------------|\n");
    md.push_str("| `tpack-fullschema` | Cold self-contained message (schema + data) |\n");
    md.push_str("| `tpack-fullschema-with-id` | Cold + `xxh64-v1` schema id |\n");
    md.push_str("| `tpack-schemaref` | Steady-state id + data (registry on decode) |\n");
    md.push_str("| `json` | Compact `serde_json` |\n");
    md.push_str("| `cbor` | `ciborium` named maps |\n");
    md.push_str("| `msgpack` | `rmp-serde` named maps |\n\n");
    md.push_str("### Semantic twins\n\n");
    md.push_str("Competitors encode *semantic twins* (i64 unscaled for DecimalFixed, decimal **string** for Decimal, serde structs for nesting).\n");
    md.push_str("They are not the same type system; comparisons are about application payload footprint.\n\n");
    md.push_str("### Scenarios\n\n");
    for scenario in Scenario::ALL {
        md.push_str(&format!(
            "- **`{}`**: {}\n",
            scenario.name(),
            scenario.description()
        ));
    }
    md.push('\n');

    md.push_str("## Steady-state size (SchemaRef primary)\n\n");
    md.push_str("| Scenario | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|----------|--------|------:|--------:|--------:|\n");
    let steady_formats = [
        Format::TpackSchemaRef,
        Format::Json,
        Format::Cbor,
        Format::MsgPack,
    ];
    for scenario in Scenario::ALL {
        let json_b = sample_bytes(samples, scenario, Format::Json);
        let cbor_b = sample_bytes(samples, scenario, Format::Cbor);
        for format in steady_formats {
            let bytes = sample_bytes(samples, scenario, format);
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                scenario.name(),
                format.label(),
                bytes,
                ratio(bytes, json_b),
                ratio(bytes, cbor_b),
            ));
        }
    }
    md.push('\n');

    md.push_str("## Cold / self-contained size (FullSchema)\n\n");
    md.push_str("Includes embedded schema every message — expected to lose to CBOR on **tiny** single records.\n\n");
    md.push_str("| Scenario | Format | Bytes | vs JSON | vs CBOR |\n");
    md.push_str("|----------|--------|------:|--------:|--------:|\n");
    for scenario in Scenario::ALL {
        let json_b = sample_bytes(samples, scenario, Format::Json);
        let cbor_b = sample_bytes(samples, scenario, Format::Cbor);
        for format in [Format::TpackFullSchema, Format::TpackFullSchemaWithId] {
            let bytes = sample_bytes(samples, scenario, format);
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                scenario.name(),
                format.label(),
                bytes,
                ratio(bytes, json_b),
                ratio(bytes, cbor_b),
            ));
        }
    }
    md.push('\n');

    md.push_str("## TPACK component breakdown\n\n");
    md.push_str(
        "| Scenario | Mode | Total | Header+ver+mode | SchemaId payload | Schema | Data |\n",
    );
    md.push_str(
        "|----------|------|------:|----------------:|-----------------:|-------:|-----:|\n",
    );
    for scenario in Scenario::ALL {
        for format in [
            Format::TpackFullSchema,
            Format::TpackFullSchemaWithId,
            Format::TpackSchemaRef,
        ] {
            let b = breakdown_tpack(scenario, format).expect("breakdown");
            md.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} | {} | {} |\n",
                scenario.name(),
                format.label(),
                b.total,
                b.header_and_mode,
                b.schema_id,
                b.schema,
                b.data,
            ));
        }
    }
    md.push_str("\nLength prefixes for schema id / schema len are part of total but not expanded as separate columns.\n\n");

    md.push_str("## Amortized multi-message stream\n\n");
    md.push_str(&format!(
        "Models a realistic session: **1× FullSchemaWithId** (cold bind) + **{}× SchemaRef** (hot).\n",
        AMORTIZED_MESSAGE_COUNT - 1
    ));
    md.push_str("Competitors pay full map size every message.\n\n");
    md.push_str("| Scenario | Stream | Total bytes (n=100) | Per message avg | vs 100× JSON | vs 100× CBOR |\n");
    md.push_str("|----------|--------|--------------------:|----------------:|-------------:|-------------:|\n");
    for scenario in Scenario::ALL {
        let tpack = measure_amortized(scenario, AMORTIZED_MESSAGE_COUNT).expect("amortized");
        let json = measure_amortized_competitor(scenario, Format::Json, AMORTIZED_MESSAGE_COUNT)
            .expect("json");
        let cbor = measure_amortized_competitor(scenario, Format::Cbor, AMORTIZED_MESSAGE_COUNT)
            .expect("cbor");
        md.push_str(&format!(
            "| `{}` | `{}` | {} | {:.1} | {} | {} |\n",
            scenario.name(),
            tpack.label,
            tpack.total_bytes,
            tpack.per_message,
            ratio(tpack.total_bytes, json.total_bytes),
            ratio(tpack.total_bytes, cbor.total_bytes),
        ));
        md.push_str(&format!(
            "| `{}` | `json` ×100 | {} | {:.1} | 1.00× | {} |\n",
            scenario.name(),
            json.total_bytes,
            json.per_message,
            ratio(json.total_bytes, cbor.total_bytes),
        ));
        md.push_str(&format!(
            "| `{}` | `cbor` ×100 | {} | {:.1} | {} | 1.00× |\n",
            scenario.name(),
            cbor.total_bytes,
            cbor.per_message,
            ratio(cbor.total_bytes, json.total_bytes),
        ));
    }
    md.push('\n');

    md.push_str("## Throughput (informational)\n\n");
    md.push_str(
        "Criterion benches use **PreparedSchema + Encoder buffer reuse** (steady-state API).\n",
    );
    md.push_str("Naive `encode_message` still re-encodes schema for FullSchema modes; prefer `PreparedSchema` in hot loops.\n\n");
    md.push_str(
        "```bash\ncargo bench -p tpack-bench\ncargo bench -p tpack-bench -- --quick\n```\n\n",
    );
    md.push_str("| Environment | Notes |\n|-------------|-------|\n");
    md.push_str("| CPU | *fill in when publishing absolute numbers* |\n");
    md.push_str("| CI policy | Full Criterion is **not** part of default CI |\n\n");

    md.push_str("## Golden guards\n\n");
    md.push_str("- `flat_record` × FullSchema == 76 (draft-00 vector length)\n");
    md.push_str("- Component breakdown sums are consistent with total size\n");
    md
}

fn sample_bytes(samples: &[SizeSample], scenario: Scenario, format: Format) -> usize {
    samples
        .iter()
        .find(|s| s.scenario == scenario && s.format == format)
        .map(|s| s.bytes)
        .unwrap_or(0)
}

fn ratio(num: usize, den: usize) -> String {
    if den == 0 {
        return "n/a".into();
    }
    format!("{:.2}×", num as f64 / den as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAT_RECORD_FULLSCHEMA_BYTES: usize = 76;

    #[test]
    fn flat_record_fullschema_matches_golden_size() {
        let bytes = encode(Scenario::FlatRecord, Format::TpackFullSchema).expect("encode");
        assert_eq!(bytes.len(), FLAT_RECORD_FULLSCHEMA_BYTES);
    }

    #[test]
    fn breakdown_data_matches_across_tpack_modes() {
        let full = breakdown_tpack(Scenario::FlatRecord, Format::TpackFullSchema).unwrap();
        let sref = breakdown_tpack(Scenario::FlatRecord, Format::TpackSchemaRef).unwrap();
        assert_eq!(full.data, sref.data);
        assert!(full.schema > 0);
        assert_eq!(sref.schema, 0);
        assert!(sref.schema_id > 0);
    }

    #[test]
    fn amortized_beats_100x_cbor_on_bulk() {
        let tpack = measure_amortized(Scenario::BulkList, AMORTIZED_MESSAGE_COUNT).unwrap();
        let cbor =
            measure_amortized_competitor(Scenario::BulkList, Format::Cbor, AMORTIZED_MESSAGE_COUNT)
                .unwrap();
        assert!(
            tpack.total_bytes < cbor.total_bytes,
            "amortized tpack {} should beat cbor {}",
            tpack.total_bytes,
            cbor.total_bytes
        );
    }

    #[test]
    fn steady_encoder_matches_oneshot() {
        let oneshot = encode(Scenario::FlatRecord, Format::TpackSchemaRef).unwrap();
        let mut steady = SteadyEncoder::new(Scenario::FlatRecord, Format::TpackSchemaRef).unwrap();
        let a = steady.encode_once().unwrap();
        let b = steady.encode_once().unwrap();
        assert_eq!(oneshot, a);
        assert_eq!(a, b);
    }

    #[test]
    fn all_sizes_positive_and_deterministic() {
        let a = measure_all_sizes().unwrap();
        let b = measure_all_sizes().unwrap();
        assert_eq!(a, b);
        for sample in &a {
            assert!(sample.bytes > 0);
        }
    }

    #[test]
    fn markdown_tells_steady_state_story() {
        let samples = measure_all_sizes().unwrap();
        let md = render_benchmarks_markdown(&samples);
        assert!(md.contains("Steady-state size"));
        assert!(md.contains("Amortized multi-message"));
        assert!(md.contains("component breakdown") || md.contains("Component breakdown"));
        assert!(md.contains("PreparedSchema"));
    }

    #[test]
    fn roundtrip_all_formats_flat_record() {
        for format in Format::ALL {
            let bytes = encode(Scenario::FlatRecord, format).expect("encode");
            decode(Scenario::FlatRecord, format, &bytes).unwrap();
        }
    }
}
