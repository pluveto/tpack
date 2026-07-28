//! Generator for `test-vectors/v1/reference/xxh64-v1-flat-record/`.
//!
//! Emits FullSchemaWithId and SchemaRef envelopes for the shared flat-record
//! example using the official `xxh64-v1` SchemaId (8-byte BE digest).
//!
//! Usage (from repo root):
//!   cargo run -p tpack --example gen_xxh64_v1_vectors

use std::{borrow::Cow, fs, path::PathBuf, process};

use tpack::{
    Decimal, EnvelopeMode, Field, Schema, TpackValue, TypeDescriptor, encode_message,
    recommended_schema_id_xxh64_v1,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("gen_xxh64_v1_vectors: {err}");
        process::exit(1);
    }
}

fn format_hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (i, byte) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{byte:02X}"));
    }
    out.push('\n');
    out
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../test-vectors/v1/reference/xxh64-v1-flat-record")
        });
    fs::create_dir_all(&out_dir)?;

    // Same flat-record schema/value as draft-00 examples and reference.rs.
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

    let schema_id = recommended_schema_id_xxh64_v1(&schema)?;
    assert_eq!(
        schema_id,
        [0x23, 0x73, 0x76, 0xF7, 0x21, 0xB6, 0x0A, 0x41],
        "schema id must match documented xxh64-v1 digest for flat-record"
    );

    let with_id = encode_message(
        &schema,
        &value,
        EnvelopeMode::FullSchemaWithId,
        Some(&schema_id),
    )?;
    let schema_ref = encode_message(&schema, &value, EnvelopeMode::SchemaRef, Some(&schema_id))?;

    fs::write(
        out_dir.join("full-schema-with-id.hex"),
        format_hex(&with_id),
    )?;
    fs::write(out_dir.join("schema-ref.hex"), format_hex(&schema_ref))?;

    eprintln!("schema_id = {:02X?}", schema_id);
    eprintln!(
        "wrote {}/full-schema-with-id.hex ({} bytes)",
        out_dir.display(),
        with_id.len()
    );
    eprintln!(
        "wrote {}/schema-ref.hex ({} bytes)",
        out_dir.display(),
        schema_ref.len()
    );
    Ok(())
}
