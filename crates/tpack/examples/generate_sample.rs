use std::{borrow::Cow, fs, path::PathBuf, process};

use tpack::{Decimal, EnvelopeMode, Field, Schema, TpackValue, TypeDescriptor, encode_message};

fn main() {
    if let Err(err) = run() {
        eprintln!("generate_sample: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/flat-record-full-schema.tpack"));

    // Keep the sample aligned with the draft flat-record example.
    let schema = Schema::new(TypeDescriptor::Struct(vec![
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
    ]));

    let value = TpackValue::Struct(vec![
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
    ]);

    let bytes = encode_message(&schema, &value, EnvelopeMode::FullSchema, None)?;
    fs::write(&output, bytes)?;
    eprintln!("wrote {}", output.display());
    Ok(())
}
