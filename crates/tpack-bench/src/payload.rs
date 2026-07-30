//! Named scenarios and scaling payloads (TPACK + serde semantic twins).

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use tpack::{
    BigInt, Decimal, Field, Schema, TpackValue, TypeDescriptor, recommended_schema_id_xxh64_v1,
};

/// Fixed bulk list length used by [`Scenario::BulkList`].
pub const BULK_LIST_LEN: usize = 100;

/// Named fixed scenarios (product narratives + golden guards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scenario {
    /// Draft flat-record example (`id`, `price`, `tax`, `qty`, `ts`).
    FlatRecord,
    /// Small order with three nested line items.
    NestedStruct,
    /// List of [`BULK_LIST_LEN`] flat-like items.
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
            Scenario::BulkList => "List of 100 flat-like product rows (amortized schema cost)",
        }
    }
}

// ---------------------------------------------------------------------------
// Serde semantic twins
// ---------------------------------------------------------------------------

/// Competitor twin of the draft flat-record example.
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

/// Blob twin: id + opaque payload (for payload-size scaling).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlobSerde {
    pub id: String,
    pub blob: Vec<u8>,
}

/// Wide struct twin: N integer fields (for field-count scaling).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WideSerde {
    pub fields: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SerdePayload {
    Flat(FlatRecordSerde),
    Nested(NestedOrderSerde),
    Bulk(Vec<FlatRecordSerde>),
    Blob(BlobSerde),
    Wide(WideSerde),
    /// Homogeneous list of flat records with variable length.
    FlatList(Vec<FlatRecordSerde>),
}

// ---------------------------------------------------------------------------
// Fixed scenarios
// ---------------------------------------------------------------------------

pub fn tpack_schema(scenario: Scenario) -> Schema {
    match scenario {
        Scenario::FlatRecord => flat_record_schema(),
        Scenario::NestedStruct => nested_struct_schema(),
        Scenario::BulkList => bulk_list_schema(BULK_LIST_LEN),
    }
}

pub fn tpack_value(scenario: Scenario) -> TpackValue<'static> {
    match scenario {
        Scenario::FlatRecord => flat_record_value(),
        Scenario::NestedStruct => nested_struct_value(),
        Scenario::BulkList => bulk_list_value(BULK_LIST_LEN),
    }
}

pub fn schema_id_bytes(scenario: Scenario) -> [u8; 8] {
    recommended_schema_id_xxh64_v1(&tpack_schema(scenario)).expect("scenario schema is valid")
}

pub fn serde_payload(scenario: Scenario) -> SerdePayload {
    match scenario {
        Scenario::FlatRecord => SerdePayload::Flat(flat_record_serde()),
        Scenario::NestedStruct => SerdePayload::Nested(nested_struct_serde()),
        Scenario::BulkList => SerdePayload::Bulk(bulk_list_serde(BULK_LIST_LEN)),
    }
}

pub fn flat_record_schema() -> Schema {
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

pub fn flat_record_value() -> TpackValue<'static> {
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
        price: 2_999_900,
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

fn bulk_list_schema(len: usize) -> Schema {
    Schema::new(TypeDescriptor::List {
        max_count: Some(len as u64),
        element: Box::new(flat_record_schema().root),
    })
}

fn bulk_list_value(len: usize) -> TpackValue<'static> {
    let mut items = Vec::with_capacity(len);
    for i in 0..len {
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

fn bulk_list_serde(len: usize) -> Vec<FlatRecordSerde> {
    (0..len)
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
// Scaling axes (deterministic, parameterized)
// ---------------------------------------------------------------------------

/// Payload-size scale points (blob body length in bytes).
pub const BLOB_SIZES: [usize; 5] = [0, 64, 1024, 16_384, 1_048_576];

/// List-length scale points.
pub const LIST_LENGTHS: [usize; 5] = [1, 10, 100, 1_000, 10_000];

/// Field-count scale points (I64 fields in a flat struct).
pub const FIELD_COUNTS: [usize; 4] = [4, 16, 64, 256];

/// Message counts for amortization scans.
pub const AMORTIZED_NS: [usize; 5] = [1, 10, 100, 1_000, 10_000];

/// Schema/value for `{ id: string, blob: bytes }` with a fixed blob length.
pub fn blob_schema() -> Schema {
    Schema::new(TypeDescriptor::Struct(vec![
        Field::new(1, "id", TypeDescriptor::String { max_len: Some(32) }),
        Field::new(2, "blob", TypeDescriptor::Bytes { max_len: None }),
    ]))
}

pub fn blob_value(blob_len: usize) -> TpackValue<'static> {
    // Deterministic fill so size and content are stable.
    let mut blob = vec![0u8; blob_len];
    for (i, b) in blob.iter_mut().enumerate() {
        *b = (i.wrapping_mul(131) + 17) as u8;
    }
    TpackValue::Struct(vec![
        (1, TpackValue::String(Cow::Borrowed("blob-id"))),
        (2, TpackValue::Bytes(Cow::Owned(blob))),
    ])
}

pub fn blob_serde(blob_len: usize) -> BlobSerde {
    let mut blob = vec![0u8; blob_len];
    for (i, b) in blob.iter_mut().enumerate() {
        *b = (i.wrapping_mul(131) + 17) as u8;
    }
    BlobSerde {
        id: "blob-id".to_string(),
        blob,
    }
}

/// Schema/value for a struct of `field_count` I64 fields named f0..fN.
pub fn wide_schema(field_count: usize) -> Schema {
    let fields = (0..field_count)
        .map(|i| Field::new(i as u64 + 1, format!("f{i}"), TypeDescriptor::I64))
        .collect();
    Schema::new(TypeDescriptor::Struct(fields))
}

pub fn wide_value(field_count: usize) -> TpackValue<'static> {
    let fields = (0..field_count)
        .map(|i| (i as u64 + 1, TpackValue::I64(i as i64 * 17 + 3)))
        .collect();
    TpackValue::Struct(fields)
}

pub fn wide_serde(field_count: usize) -> WideSerde {
    WideSerde {
        fields: (0..field_count).map(|i| i as i64 * 17 + 3).collect(),
    }
}

pub fn list_schema(len: usize) -> Schema {
    bulk_list_schema(len)
}

pub fn list_value(len: usize) -> TpackValue<'static> {
    bulk_list_value(len)
}

pub fn list_serde(len: usize) -> Vec<FlatRecordSerde> {
    bulk_list_serde(len)
}

pub fn schema_id_for(schema: &Schema) -> [u8; 8] {
    recommended_schema_id_xxh64_v1(schema).expect("schema is valid")
}
