use std::{borrow::Cow, fs, path::PathBuf, process};

use tpack::{EnvelopeMode, Field, Schema, TpackValue, TypeDescriptor, encode_message};

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
        .unwrap_or_else(|| PathBuf::from("target/sample.tpack"));

    let schema = Schema::new(TypeDescriptor::Struct(vec![
        Field::new(1, "id", TypeDescriptor::String { max_len: Some(64) }),
        Field::new(
            2,
            "price",
            TypeDescriptor::DecimalFixed {
                precision: 10,
                scale: 2,
            },
        ),
        Field::new(3, "qty", TypeDescriptor::I32),
        Field::new(4, "active", TypeDescriptor::Bool),
    ]));

    let value = TpackValue::Struct(vec![
        (1, TpackValue::String(Cow::Borrowed("prod_001"))),
        (2, TpackValue::DecimalFixed(2_999_900)),
        (3, TpackValue::I32(10)),
        (4, TpackValue::Bool(true)),
    ]);

    let bytes = encode_message(&schema, &value, EnvelopeMode::FullSchema, None)?;
    fs::write(&output, bytes)?;
    eprintln!("wrote {}", output.display());
    Ok(())
}
