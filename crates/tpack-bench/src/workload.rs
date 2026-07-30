//! One abstraction: a named payload that can be spoken by every format.
//!
//! Factories produce workloads; the runner never knows about "flat vs nested"
//! special cases.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use tpack::{
    BigInt, Decimal, Field, Schema, TpackValue, TypeDescriptor, recommended_schema_id_xxh64_v1,
};

/// Application payload + schema, independent of wire format.
#[derive(Debug, Clone)]
pub struct Workload {
    pub name: String,
    pub schema: Schema,
    pub value: TpackValue<'static>,
    pub twin: Twin,
}

impl Workload {
    pub fn schema_id(&self) -> [u8; 8] {
        recommended_schema_id_xxh64_v1(&self.schema).expect("valid schema")
    }
}

/// Serde-facing twin used by JSON / CBOR / MessagePack (named maps).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Twin {
    Flat(Flat),
    Nested(Nested),
    List(Vec<Flat>),
    Blob(Blob),
    Wide(Wide),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Flat {
    pub id: String,
    pub price: i64,
    pub tax: String,
    pub qty: i32,
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub sku: String,
    pub qty: i32,
    pub unit_price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Nested {
    pub order_id: String,
    pub customer: String,
    pub items: Vec<Line>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Blob {
    pub id: String,
    pub blob: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Wide {
    /// Stable map-like encoding via serde: field names f0..fN on a struct would
    /// need macros; a list of values still repeats less key tax than JSON objects
    /// with names. For field-count tax we serialize as a map via `serde_json::Value`
    /// in the runner when needed — see `Wide::as_named_map`.
    pub fields: Vec<i64>,
}

impl Wide {
    /// Named-map twin so JSON/CBOR/MessagePack pay field-name tax like a real struct.
    pub fn as_named_map(&self) -> serde_json::Map<String, serde_json::Value> {
        self.fields
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("f{i}"), serde_json::json!(v)))
            .collect()
    }
}

// --- factories ----------------------------------------------------------------

pub const BULK_LEN: usize = 100;
pub const BLOB_SIZES: [usize; 5] = [0, 64, 1024, 16_384, 1_048_576];
pub const LIST_LENS: [usize; 5] = [1, 10, 100, 1_000, 10_000];
pub const FIELD_COUNTS: [usize; 4] = [4, 16, 64, 256];
pub const AMORTIZED_NS: [usize; 5] = [1, 10, 100, 1_000, 10_000];

pub fn flat_record() -> Workload {
    Workload {
        name: "flat_record".into(),
        schema: flat_schema(),
        value: flat_value(0),
        twin: Twin::Flat(flat_twin(0)),
    }
}

pub fn nested_struct() -> Workload {
    Workload {
        name: "nested_struct".into(),
        schema: Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "order_id", TypeDescriptor::String { max_len: Some(32) }),
            Field::new(2, "customer", TypeDescriptor::String { max_len: Some(64) }),
            Field::new(
                3,
                "items",
                TypeDescriptor::List {
                    max_count: Some(64),
                    element: Box::new(line_schema()),
                },
            ),
        ])),
        value: TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("ord-10042"))),
            (2, TpackValue::String(Cow::Borrowed("alice@example.com"))),
            (
                3,
                TpackValue::List(vec![
                    line_value("SKU-A", 2, 1_250_000),
                    line_value("SKU-B", 1, 499_900),
                    line_value("SKU-C", 5, 99_500),
                ]),
            ),
        ]),
        twin: Twin::Nested(Nested {
            order_id: "ord-10042".into(),
            customer: "alice@example.com".into(),
            items: vec![
                Line {
                    sku: "SKU-A".into(),
                    qty: 2,
                    unit_price: 1_250_000,
                },
                Line {
                    sku: "SKU-B".into(),
                    qty: 1,
                    unit_price: 499_900,
                },
                Line {
                    sku: "SKU-C".into(),
                    qty: 5,
                    unit_price: 99_500,
                },
            ],
        }),
    }
}

pub fn bulk_list() -> Workload {
    list_of(BULK_LEN)
}

pub fn list_of(len: usize) -> Workload {
    Workload {
        name: format!("list_{len}"),
        schema: Schema::new(TypeDescriptor::List {
            max_count: Some(len as u64),
            element: Box::new(flat_schema().root),
        }),
        value: TpackValue::List((0..len).map(flat_value).collect()),
        twin: Twin::List((0..len).map(flat_twin).collect()),
    }
}

pub fn blob(len: usize) -> Workload {
    let mut body = vec![0u8; len];
    for (i, b) in body.iter_mut().enumerate() {
        *b = (i.wrapping_mul(131) + 17) as u8;
    }
    Workload {
        name: format!("blob_{len}"),
        schema: Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "id", TypeDescriptor::String { max_len: Some(32) }),
            Field::new(2, "blob", TypeDescriptor::Bytes { max_len: None }),
        ])),
        value: TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("blob-id"))),
            (2, TpackValue::Bytes(Cow::Owned(body.clone()))),
        ]),
        twin: Twin::Blob(Blob {
            id: "blob-id".into(),
            blob: body,
        }),
    }
}

pub fn wide(fields: usize) -> Workload {
    let schema_fields = (0..fields)
        .map(|i| Field::new(i as u64 + 1, format!("f{i}"), TypeDescriptor::I64))
        .collect();
    let values: Vec<_> = (0..fields)
        .map(|i| (i as u64 + 1, TpackValue::I64(i as i64 * 17 + 3)))
        .collect();
    let twin_fields: Vec<_> = (0..fields).map(|i| i as i64 * 17 + 3).collect();
    Workload {
        name: format!("fields_{fields}"),
        schema: Schema::new(TypeDescriptor::Struct(schema_fields)),
        value: TpackValue::Struct(values),
        twin: Twin::Wide(Wide {
            fields: twin_fields,
        }),
    }
}

pub fn narrative() -> [Workload; 3] {
    [flat_record(), nested_struct(), bulk_list()]
}

// --- private helpers ----------------------------------------------------------

fn flat_schema() -> Schema {
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

fn flat_value(i: usize) -> TpackValue<'static> {
    let id = if i == 0 {
        Cow::Borrowed("prod_001")
    } else {
        Cow::Owned(format!("prod_{i:03}"))
    };
    TpackValue::Struct(vec![
        (1, TpackValue::String(id)),
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
    ])
}

fn flat_twin(i: usize) -> Flat {
    Flat {
        id: if i == 0 {
            "prod_001".into()
        } else {
            format!("prod_{i:03}")
        },
        price: 2_999_900 + i as i64,
        tax: format!("{:.3}", (13_725 + i as i64) as f64 / 1000.0),
        qty: 10 + (i as i32 % 7),
        ts: 1_715_000_000 + i as i64,
    }
}

fn line_schema() -> TypeDescriptor {
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

fn line_value(sku: &'static str, qty: i32, price: i64) -> TpackValue<'static> {
    TpackValue::Struct(vec![
        (1, TpackValue::String(Cow::Borrowed(sku))),
        (2, TpackValue::I32(qty)),
        (3, TpackValue::DecimalFixed(BigInt::from(price))),
    ])
}
