//! Workload: the only thing formats talk about.
//!
//! A workload is application meaning (schema + value + named-map twin).
//! It does not know JSON, TPACK, or Markdown.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use tpack::{
    BigInt, Decimal, Field, Schema, TpackValue, TypeDescriptor, recommended_schema_id_xxh64_v1,
};

#[derive(Debug, Clone)]
pub struct Workload {
    name: String,
    schema: Schema,
    value: TpackValue<'static>,
    /// Serde named-map twin (or a JSON map for wide structs).
    twin: Twin,
}

impl Workload {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn value(&self) -> &TpackValue<'static> {
        &self.value
    }

    pub fn twin(&self) -> &Twin {
        &self.twin
    }

    pub fn schema_id(&self) -> [u8; 8] {
        recommended_schema_id_xxh64_v1(&self.schema).expect("schema encodes")
    }
}

/// Competitor view: always something `serde` can turn into a named map/array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Twin {
    Value(serde_json::Value),
}

impl Twin {
    fn from_serializable<T: Serialize>(v: T) -> Self {
        Self::Value(serde_json::to_value(v).expect("twin json value"))
    }
}

// --- scale axes (declared once) ----------------------------------------------

pub const BLOB_SIZES: &[usize] = &[0, 64, 1024, 16_384, 1_048_576];
pub const LIST_LENS: &[usize] = &[1, 10, 100, 1_000, 10_000];
pub const FIELD_COUNTS: &[usize] = &[4, 16, 64, 256];
pub const AMORTIZED_NS: &[usize] = &[1, 10, 100, 1_000, 10_000];
pub const BULK: usize = 100;

pub fn narrative() -> Vec<Workload> {
    vec![flat_record(), nested(), list(BULK)]
}

pub fn flat_record() -> Workload {
    Workload {
        name: "flat_record".into(),
        schema: flat_schema(),
        value: flat_value(0),
        twin: Twin::from_serializable(flat_dto(0)),
    }
}

pub fn nested() -> Workload {
    #[derive(Serialize)]
    struct Line {
        sku: String,
        qty: i32,
        unit_price: i64,
    }
    #[derive(Serialize)]
    struct Order {
        order_id: String,
        customer: String,
        items: Vec<Line>,
    }
    let dto = Order {
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
    };
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
                    element: Box::new(line_ty()),
                },
            ),
        ])),
        value: TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("ord-10042"))),
            (2, TpackValue::String(Cow::Borrowed("alice@example.com"))),
            (
                3,
                TpackValue::List(vec![
                    line_val("SKU-A", 2, 1_250_000),
                    line_val("SKU-B", 1, 499_900),
                    line_val("SKU-C", 5, 99_500),
                ]),
            ),
        ]),
        twin: Twin::from_serializable(dto),
    }
}

pub fn list(n: usize) -> Workload {
    Workload {
        name: format!("list_{n}"),
        schema: Schema::new(TypeDescriptor::List {
            max_count: Some(n as u64),
            element: Box::new(flat_schema().root),
        }),
        value: TpackValue::List((0..n).map(flat_value).collect()),
        twin: Twin::from_serializable((0..n).map(flat_dto).collect::<Vec<_>>()),
    }
}

pub fn blob(n: usize) -> Workload {
    let mut body = vec![0u8; n];
    for (i, b) in body.iter_mut().enumerate() {
        *b = (i.wrapping_mul(131) + 17) as u8;
    }
    #[derive(Serialize)]
    struct B {
        id: String,
        blob: Vec<u8>,
    }
    Workload {
        name: format!("blob_{n}"),
        schema: Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "id", TypeDescriptor::String { max_len: Some(32) }),
            Field::new(2, "blob", TypeDescriptor::Bytes { max_len: None }),
        ])),
        value: TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("blob-id"))),
            (2, TpackValue::Bytes(Cow::Owned(body.clone()))),
        ]),
        twin: Twin::from_serializable(B {
            id: "blob-id".into(),
            blob: body,
        }),
    }
}

pub fn wide(n: usize) -> Workload {
    let mut map = serde_json::Map::new();
    let fields: Vec<_> = (0..n)
        .map(|i| {
            let v = i as i64 * 17 + 3;
            map.insert(format!("f{i}"), serde_json::json!(v));
            (i as u64 + 1, TpackValue::I64(v))
        })
        .collect();
    Workload {
        name: format!("fields_{n}"),
        schema: Schema::new(TypeDescriptor::Struct(
            (0..n)
                .map(|i| Field::new(i as u64 + 1, format!("f{i}"), TypeDescriptor::I64))
                .collect(),
        )),
        value: TpackValue::Struct(fields),
        twin: Twin::Value(serde_json::Value::Object(map)),
    }
}

// --- private -----------------------------------------------------------------

#[derive(Serialize)]
struct FlatDto {
    id: String,
    price: i64,
    tax: String,
    qty: i32,
    ts: i64,
}

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

fn flat_dto(i: usize) -> FlatDto {
    FlatDto {
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

fn line_ty() -> TypeDescriptor {
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

fn line_val(sku: &'static str, qty: i32, price: i64) -> TpackValue<'static> {
    TpackValue::Struct(vec![
        (1, TpackValue::String(Cow::Borrowed(sku))),
        (2, TpackValue::I32(qty)),
        (3, TpackValue::DecimalFixed(BigInt::from(price))),
    ])
}
