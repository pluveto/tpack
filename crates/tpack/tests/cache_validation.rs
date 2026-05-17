use tpack::{
    DecodeOptions, Decoder, EnvelopeMode, ErrorKind, Field, Schema, StdSchemaRegistry, TpackValue,
    TypeDescriptor, encode_message, encode_schema, recommended_schema_id_sha256,
};

fn cached_schema() -> Schema {
    Schema::new(TypeDescriptor::Struct(vec![Field::new(
        1,
        "qty",
        TypeDescriptor::I32,
    )]))
}

fn cached_value<'a>() -> TpackValue<'a> {
    TpackValue::Struct(vec![(1, TpackValue::I32(7))])
}

fn full_schema_with_id_bytes(schema_id: &[u8]) -> Vec<u8> {
    encode_message(
        &cached_schema(),
        &cached_value(),
        EnvelopeMode::FullSchemaWithId,
        Some(schema_id),
    )
    .expect("encode message")
}

fn embedded_schema_range(
    bytes: &[u8],
    schema_id_len: usize,
    schema_len: usize,
) -> core::ops::Range<usize> {
    let start = 6 + 1 + schema_id_len + 1;
    let end = start + schema_len;
    assert!(
        end <= bytes.len(),
        "embedded schema range must stay in-bounds"
    );
    start..end
}

#[test]
fn cache_hit_validates_embedded_schema_bytes_by_default() {
    let schema = cached_schema();
    let value = cached_value();
    let schema_bytes = encode_schema(&schema).expect("encode schema");
    let mut bytes = full_schema_with_id_bytes(b"cached.v1");
    let schema_range = embedded_schema_range(&bytes, b"cached.v1".len(), schema_bytes.len());
    bytes[schema_range.end - 1] = 0xFF;

    let registry = StdSchemaRegistry::new();
    registry.insert(b"cached.v1", schema.clone());

    let mut decoder = Decoder::new(&bytes);
    assert!(matches!(
        decoder
            .decode_message_with_registry(&registry)
            .unwrap_err()
            .kind(),
        ErrorKind::UnknownTypeTag(0xFF)
    ));

    let mut decoder = Decoder::with_options(
        &bytes,
        DecodeOptions {
            validate_embedded_schema_on_cache_hit: false,
            ..DecodeOptions::default()
        },
    );
    let decoded = decoder.decode_message_with_registry(&registry).unwrap();
    assert!(decoded.envelope.used_cached_schema);
    assert_eq!(decoded.schema.as_ref(), &schema);
    assert_eq!(decoded.value, value);
}

#[test]
fn cache_hit_rejects_mismatched_embedded_schema_even_when_it_is_well_formed() {
    let schema = cached_schema();
    let schema_bytes = encode_schema(&schema).expect("encode schema");
    let mut bytes = full_schema_with_id_bytes(b"cached.v1");
    let schema_range = embedded_schema_range(&bytes, b"cached.v1".len(), schema_bytes.len());
    bytes[schema_range.end - 1] = 0x05;

    let registry = StdSchemaRegistry::new();
    registry.insert(b"cached.v1", schema);

    let mut decoder = Decoder::new(&bytes);
    assert!(matches!(
        decoder
            .decode_message_with_registry(&registry)
            .unwrap_err()
            .kind(),
        ErrorKind::EmbeddedSchemaMismatch
    ));
}

#[test]
fn recommended_sha256_schema_id_still_requires_embedded_schema_validation() {
    let schema = cached_schema();
    let value = cached_value();
    let schema_bytes = encode_schema(&schema).expect("encode schema");
    let schema_id = recommended_schema_id_sha256(&schema).expect("derive schema id");
    let mut bytes = full_schema_with_id_bytes(&schema_id);
    let schema_range = embedded_schema_range(&bytes, schema_id.len(), schema_bytes.len());
    bytes[schema_range.end - 1] = 0x05;

    let registry = StdSchemaRegistry::new();
    registry.insert(schema_id.to_vec(), schema.clone());

    let mut decoder = Decoder::new(&bytes);
    assert!(matches!(
        decoder
            .decode_message_with_registry(&registry)
            .unwrap_err()
            .kind(),
        ErrorKind::EmbeddedSchemaMismatch
    ));

    let mut decoder = Decoder::with_options(
        &bytes,
        DecodeOptions {
            validate_embedded_schema_on_cache_hit: false,
            ..DecodeOptions::default()
        },
    );
    let decoded = decoder.decode_message_with_registry(&registry).unwrap();
    assert!(decoded.envelope.used_cached_schema);
    assert_eq!(decoded.schema.as_ref(), &schema);
    assert_eq!(decoded.value, value);
}
