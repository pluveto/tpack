use tpack::{
    EncodeOptions, Encoder, EnvelopeMode, ErrorKind, Field, Limits, Schema, TpackValue,
    TypeDescriptor, encode_schema,
};

fn oversized_schema() -> Schema {
    Schema::new(TypeDescriptor::Struct(vec![Field::new(
        1,
        "schema_name",
        TypeDescriptor::Null,
    )]))
}

#[test]
fn encoder_rejects_schema_bytes_over_max_schema_len() {
    let schema = oversized_schema();
    let schema_len = encode_schema(&schema).expect("encode schema").len();
    let options = EncodeOptions {
        limits: Limits {
            max_schema_len: schema_len - 1,
            ..Limits::default()
        },
        ..EncodeOptions::default()
    };

    let mut schema_encoder = Encoder::with_options(options);
    assert!(matches!(
        schema_encoder.encode_schema(&schema).unwrap_err().kind(),
        ErrorKind::SchemaLengthExceeded
    ));

    let value = TpackValue::Struct(vec![(1, TpackValue::Null)]);
    let mut message_encoder = Encoder::with_options(options);
    assert!(matches!(
        message_encoder
            .encode_message(&schema, &value, EnvelopeMode::FullSchema, None)
            .unwrap_err()
            .kind(),
        ErrorKind::SchemaLengthExceeded
    ));
}
