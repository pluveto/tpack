use std::borrow::Cow;

use tpack::{
    CanonicalMode, DecodeOptions, Decoder, EncodeOptions, EnvelopeMode, ErrorKind, Field, Schema,
    TpackValue, TypeDescriptor, ValueMapEntry, Variant, encode_message,
};

mod reference_cases {
    use super::*;

    #[cfg(feature = "std")]
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

    #[cfg(feature = "std")]
    fn flat_value<'a>() -> TpackValue<'a> {
        TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("prod_001"))),
            (2, TpackValue::DecimalFixed(2_999_900)),
            (
                3,
                TpackValue::Decimal(tpack::Decimal {
                    scale: 3,
                    coefficient: 13_725,
                }),
            ),
            (4, TpackValue::I32(10)),
            (5, TpackValue::I64(1_715_000_000)),
        ])
    }

    fn draft_flat_full_schema_hex() -> Vec<u8> {
        vec![
            0x54, 0x50, 0x41, 0x4B, 0x01, 0x00, 0x28, 0x20, 0x05, 0x01, 0x02, 0x69, 0x64, 0x00,
            0x0E, 0x40, 0x02, 0x05, 0x70, 0x72, 0x69, 0x63, 0x65, 0x00, 0x0D, 0x12, 0x04, 0x03,
            0x03, 0x74, 0x61, 0x78, 0x00, 0x0C, 0x04, 0x03, 0x71, 0x74, 0x79, 0x00, 0x04, 0x05,
            0x02, 0x74, 0x73, 0x00, 0x05, 0x08, 0x70, 0x72, 0x6F, 0x64, 0x5F, 0x30, 0x30, 0x31,
            0xB8, 0x99, 0xEE, 0x02, 0x06, 0xBA, 0xD6, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00,
            0x00, 0x00, 0x66, 0x38, 0xD2, 0xC0,
        ]
    }

    #[cfg(feature = "std")]
    fn draft_flat_with_id_hex() -> Vec<u8> {
        let mut bytes = vec![
            0x54, 0x50, 0x41, 0x4B, 0x01, 0x01, 0x11, 0x65, 0x78, 0x61, 0x6D, 0x70, 0x6C, 0x65,
            0x2E, 0x72, 0x65, 0x63, 0x6F, 0x72, 0x64, 0x2E, 0x76, 0x31, 0x28,
        ];
        bytes.extend_from_slice(&draft_flat_full_schema_hex()[7..]);
        bytes
    }

    fn draft_flat_schema_ref_hex() -> Vec<u8> {
        let mut bytes = vec![
            0x54, 0x50, 0x41, 0x4B, 0x01, 0x02, 0x11, 0x65, 0x78, 0x61, 0x6D, 0x70, 0x6C, 0x65,
            0x2E, 0x72, 0x65, 0x63, 0x6F, 0x72, 0x64, 0x2E, 0x76, 0x31,
        ];
        bytes.extend_from_slice(&draft_flat_full_schema_hex()[47..]);
        bytes
    }

    #[cfg(feature = "std")]
    #[test]
    fn draft_section_12_envelopes_decode_and_canonicalize() {
        let schema = flat_schema();
        let value = flat_value();
        assert_eq!(
            encode_message(&schema, &value, EnvelopeMode::FullSchema, None).unwrap(),
            draft_flat_full_schema_hex()
        );
        assert_eq!(
            encode_message(
                &schema,
                &value,
                EnvelopeMode::FullSchemaWithId,
                Some(b"example.record.v1"),
            )
            .unwrap(),
            draft_flat_with_id_hex()
        );
        assert_eq!(
            encode_message(
                &schema,
                &value,
                EnvelopeMode::SchemaRef,
                Some(b"example.record.v1"),
            )
            .unwrap(),
            draft_flat_schema_ref_hex()
        );

        let registry = tpack::StdSchemaRegistry::new();
        registry.insert(b"example.record.v1", schema.clone());

        let with_id_bytes = draft_flat_with_id_hex();
        let mut decoder = Decoder::new(&with_id_bytes);
        let with_id = decoder.decode_message_with_registry(&registry).unwrap();
        assert_eq!(with_id.envelope.mode, EnvelopeMode::FullSchemaWithId);
        assert!(with_id.envelope.used_cached_schema);
        assert_eq!(with_id.schema.as_ref(), &schema);
        assert_eq!(with_id.value, value);

        let schema_ref_bytes = draft_flat_schema_ref_hex();
        let mut decoder = Decoder::new(&schema_ref_bytes);
        let schema_ref = decoder.decode_message_with_registry(&registry).unwrap();
        assert_eq!(schema_ref.envelope.mode, EnvelopeMode::SchemaRef);
        assert!(schema_ref.envelope.used_cached_schema);
        assert_eq!(schema_ref.value, flat_value());

        let full_schema_bytes = draft_flat_full_schema_hex();
        let mut decoder = Decoder::with_options(
            &full_schema_bytes,
            DecodeOptions {
                canonical: CanonicalMode::Strict,
                ..DecodeOptions::default()
            },
        );
        let message = decoder.decode_message().unwrap();
        let mut encoder = tpack::Encoder::with_options(EncodeOptions {
            canonical: CanonicalMode::Strict,
            ..EncodeOptions::default()
        });
        encoder
            .encode_message(
                &message.schema,
                &message.value,
                EnvelopeMode::FullSchema,
                None,
            )
            .unwrap();
        assert_eq!(encoder.into_vec(), draft_flat_full_schema_hex());
    }

    #[test]
    fn schema_ref_requires_registry_hit_and_profile_permission() {
        let bytes = draft_flat_schema_ref_hex();
        let mut decoder = Decoder::new(&bytes);
        assert!(matches!(
            decoder.decode_message().unwrap_err().kind(),
            ErrorKind::UnknownSchemaId
        ));

        let mut decoder = Decoder::with_options(
            &bytes,
            DecodeOptions {
                allow_schema_ref: false,
                ..DecodeOptions::default()
            },
        );
        assert!(matches!(
            decoder.decode_message().unwrap_err().kind(),
            ErrorKind::SchemaRefNotAllowed
        ));
    }

    #[test]
    fn canonical_map_ordering_and_nan_are_enforced() {
        let schema = Schema::new(TypeDescriptor::Map {
            max_count: None,
            key: Box::new(TypeDescriptor::String { max_len: None }),
            value: Box::new(TypeDescriptor::I32),
        });
        let value = TpackValue::Map(vec![
            ValueMapEntry {
                key: TpackValue::String(Cow::Borrowed("b")),
                value: TpackValue::I32(2),
            },
            ValueMapEntry {
                key: TpackValue::String(Cow::Borrowed("a")),
                value: TpackValue::I32(1),
            },
        ]);
        let bytes = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).unwrap();
        let mut decoder = Decoder::with_options(
            &bytes,
            DecodeOptions {
                canonical: CanonicalMode::Strict,
                ..DecodeOptions::default()
            },
        );
        assert!(decoder.decode_message().is_err());

        let nan_schema = Schema::new(TypeDescriptor::Map {
            max_count: None,
            key: Box::new(TypeDescriptor::F32),
            value: Box::new(TypeDescriptor::I32),
        });
        let nan_value = TpackValue::Map(vec![ValueMapEntry {
            key: TpackValue::F32(f32::NAN),
            value: TpackValue::I32(1),
        }]);
        assert!(encode_message(&nan_schema, &nan_value, EnvelopeMode::FullSchema, None).is_err());
    }

    #[test]
    fn map_duplicate_keys_use_canonical_key_bytes_even_outside_strict_mode() {
        let bytes = vec![
            0x54, 0x50, 0x41, 0x4B, 0x01, 0x00, 0x0B, 0x22, 0x00, 0x24, 0x01, 0x01, b'a', 0x04,
            0x02, 0x00, 0x00, 0x00, 0x01, 0x80, 0x00, 0x00, 0x00, 0x02,
        ];
        assert!(Decoder::new(&bytes).decode_message().is_err());
    }

    #[test]
    fn validation_rejects_invalid_schema_and_bounded_values() {
        let invalid_decimal = Schema::new(TypeDescriptor::DecimalFixed {
            precision: 0,
            scale: 0,
        });
        assert!(
            encode_message(
                &invalid_decimal,
                &TpackValue::DecimalFixed(0),
                EnvelopeMode::FullSchema,
                None,
            )
            .is_err()
        );

        let duplicate_id = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "a", TypeDescriptor::I32),
            Field::new(1, "b", TypeDescriptor::I32),
        ]));
        assert!(
            encode_message(
                &duplicate_id,
                &TpackValue::Struct(vec![(1, TpackValue::I32(1)), (1, TpackValue::I32(2))]),
                EnvelopeMode::FullSchema,
                None,
            )
            .is_err()
        );

        let duplicate_name = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "a", TypeDescriptor::I32),
            Field::new(2, "a", TypeDescriptor::I32),
        ]));
        assert!(
            encode_message(
                &duplicate_name,
                &TpackValue::Struct(vec![(1, TpackValue::I32(1)), (2, TpackValue::I32(2))]),
                EnvelopeMode::FullSchema,
                None,
            )
            .is_err()
        );

        let bounded_string = Schema::new(TypeDescriptor::String { max_len: Some(3) });
        assert!(
            encode_message(
                &bounded_string,
                &TpackValue::String(Cow::Borrowed("toolong")),
                EnvelopeMode::FullSchema,
                None,
            )
            .is_err()
        );

        let bounded_list = Schema::new(TypeDescriptor::List {
            max_count: Some(1),
            element: Box::new(TypeDescriptor::I32),
        });
        assert!(
            encode_message(
                &bounded_list,
                &TpackValue::List(vec![TpackValue::I32(1), TpackValue::I32(2)]),
                EnvelopeMode::FullSchema,
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn decoder_rejects_flags_trailing_bytes_noncanonical_nan_and_varint_overflow() {
        let mut nonzero_flags = vec![
            0x54, 0x50, 0x41, 0x4B, 0x01, 0x00, 0x07, 0x20, 0x01, 0x01, 0x01, b'a', 0x01, 0x04,
            0x00, 0x00, 0x00, 0x01,
        ];
        assert!(Decoder::new(&nonzero_flags).decode_message().is_err());

        let schema = Schema::new(TypeDescriptor::I32);
        let mut trailing =
            encode_message(&schema, &TpackValue::I32(1), EnvelopeMode::FullSchema, None).unwrap();
        trailing.push(0);
        assert!(matches!(
            Decoder::new(&trailing).decode_message().unwrap_err().kind(),
            ErrorKind::TrailingBytes
        ));

        let nan_schema = Schema::new(TypeDescriptor::F32);
        let noncanonical_nan = TpackValue::F32(f32::from_bits(0x7FC0_0001));
        let nan_bytes = encode_message(
            &nan_schema,
            &noncanonical_nan,
            EnvelopeMode::FullSchema,
            None,
        )
        .unwrap();
        let mut decoder = Decoder::with_options(
            &nan_bytes,
            DecodeOptions {
                canonical: CanonicalMode::Strict,
                ..DecodeOptions::default()
            },
        );
        assert!(decoder.decode_message().is_err());

        let f64_schema = Schema::new(TypeDescriptor::F64);
        let noncanonical_f64_nan = TpackValue::F64(f64::from_bits(0x7FF8_0000_0000_0001));
        let f64_nan_bytes = encode_message(
            &f64_schema,
            &noncanonical_f64_nan,
            EnvelopeMode::FullSchema,
            None,
        )
        .unwrap();
        let mut decoder = Decoder::with_options(
            &f64_nan_bytes,
            DecodeOptions {
                canonical: CanonicalMode::Strict,
                ..DecodeOptions::default()
            },
        );
        assert!(decoder.decode_message().is_err());

        let overlong_field_id = vec![
            0x54, 0x50, 0x41, 0x4B, 0x01, 0x00, 0x08, 0x20, 0x01, 0x81, 0x00, 0x01, b'a', 0x00,
            0x04, 0x00, 0x00, 0x00, 0x01,
        ];
        let mut decoder = Decoder::with_options(
            &overlong_field_id,
            DecodeOptions {
                canonical: CanonicalMode::Strict,
                ..DecodeOptions::default()
            },
        );
        assert!(matches!(
            decoder.decode_message().unwrap_err().kind(),
            ErrorKind::OverlongVarint
        ));

        nonzero_flags[6] = 0xFF;
        assert!(matches!(
            Decoder::new(&nonzero_flags)
                .decode_message()
                .unwrap_err()
                .kind(),
            ErrorKind::SchemaLengthMismatch | ErrorKind::UnexpectedEof | ErrorKind::VarintOverflow
        ));
    }

    #[cfg(feature = "derive")]
    #[test]
    fn native_derive_struct_unit_enum_and_data_enum_roundtrip() {
        use tpack::{TpackDeserialize, TpackSerialize};

        #[derive(Debug, PartialEq, TpackSerialize, TpackDeserialize)]
        struct Order {
            #[tpack(field_id = 1)]
            id: String,
            #[tpack(field_id = 2, rename = "amount", type = "::tpack::TypeDescriptor::I32")]
            qty: i32,
            #[tpack(field_id = 3)]
            tags: Vec<String>,
            #[tpack(field_id = 4)]
            note: ::std::option::Option<String>,
        }

        #[derive(Debug, PartialEq, TpackSerialize, TpackDeserialize)]
        #[tpack(auto)]
        struct AutoOrder {
            id: String,
            qty: i32,
        }

        #[derive(Debug, PartialEq, TpackSerialize, TpackDeserialize)]
        enum Side {
            Buy,
            Sell,
        }

        #[derive(Debug, PartialEq, TpackSerialize, TpackDeserialize)]
        enum Event {
            Quantity(i32),
            Label(String),
        }

        let order = Order {
            id: "ord-1".to_string(),
            qty: 7,
            tags: vec!["hot".to_string(), "ioc".to_string()],
            note: Some("desk".to_string()),
        };
        let order_value = order.to_value();
        assert_eq!(Order::from_value(order_value).unwrap(), order);
        assert_eq!(
            Order::from_value(TpackValue::Struct(vec![
                (
                    3,
                    TpackValue::List(vec![TpackValue::String(Cow::Borrowed("ioc"))])
                ),
                (99, TpackValue::I32(0)),
                (2, TpackValue::I32(9)),
                (1, TpackValue::String(Cow::Borrowed("ord-2"))),
            ]))
            .unwrap(),
            Order {
                id: "ord-2".to_string(),
                qty: 9,
                tags: vec!["ioc".to_string()],
                note: None,
            }
        );
        assert!(
            Order::from_value(TpackValue::Struct(vec![
                (2, TpackValue::I32(9)),
                (1, TpackValue::String(Cow::Borrowed("ord-2"))),
            ]))
            .is_err()
        );

        let auto = AutoOrder {
            id: "ord-3".to_string(),
            qty: 11,
        };
        assert_eq!(AutoOrder::from_value(auto.to_value()).unwrap(), auto);

        let side = Side::Sell;
        assert_eq!(Side::from_value(side.to_value()).unwrap(), Side::Sell);

        let event = Event::Label("filled".to_string());
        assert_eq!(Event::from_value(event.to_value()).unwrap(), event);
    }

    #[test]
    fn struct_values_are_encoded_by_field_id_not_input_order() {
        let schema = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "timestamp", TypeDescriptor::U64),
            Field::new(2, "message", TypeDescriptor::String { max_len: None }),
        ]));
        let schema_order = TpackValue::Struct(vec![
            (1, TpackValue::U64(123)),
            (2, TpackValue::String(Cow::Borrowed("ok"))),
        ]);
        let wire_order = TpackValue::Struct(vec![
            (2, TpackValue::String(Cow::Borrowed("ok"))),
            (99, TpackValue::I32(0)),
            (1, TpackValue::U64(123)),
        ]);

        let schema_order_bytes =
            encode_message(&schema, &schema_order, EnvelopeMode::FullSchema, None).unwrap();
        let wire_order_bytes =
            encode_message(&schema, &wire_order, EnvelopeMode::FullSchema, None).unwrap();
        assert_eq!(wire_order_bytes, schema_order_bytes);
        assert_eq!(
            tpack::decode_message(&wire_order_bytes).unwrap().value,
            schema_order
        );
    }

    #[test]
    fn decoder_enforces_max_string_len_even_when_max_bytes_len_is_larger() {
        let string_schema = Schema::new(TypeDescriptor::String { max_len: None });
        let string_value = TpackValue::String(Cow::Borrowed("four"));
        let string_bytes = encode_message(
            &string_schema,
            &string_value,
            EnvelopeMode::FullSchema,
            None,
        )
        .unwrap();
        let mut decoder = Decoder::with_options(
            &string_bytes,
            DecodeOptions {
                limits: tpack::Limits {
                    max_string_len: 3,
                    max_bytes_len: 16,
                    ..tpack::Limits::default()
                },
                ..DecodeOptions::default()
            },
        );
        assert!(matches!(
            decoder.decode_message().unwrap_err().kind(),
            ErrorKind::LimitExceeded("string length")
        ));
    }

    #[test]
    fn decoder_enforces_max_extension_len_even_when_max_bytes_len_is_larger() {
        let extension_schema = Schema::new(TypeDescriptor::Extension {
            authority: "example".to_string(),
            type_name: "opaque".to_string(),
            schema_params: Vec::new(),
        });
        let extension_value = TpackValue::Extension(Cow::Borrowed(&[1, 2, 3, 4]));
        let extension_bytes = encode_message(
            &extension_schema,
            &extension_value,
            EnvelopeMode::FullSchema,
            None,
        )
        .unwrap();
        let mut decoder = Decoder::with_options(
            &extension_bytes,
            DecodeOptions {
                limits: tpack::Limits {
                    max_extension_len: 3,
                    max_bytes_len: 16,
                    ..tpack::Limits::default()
                },
                ..DecodeOptions::default()
            },
        );
        assert!(matches!(
            decoder.decode_message().unwrap_err().kind(),
            ErrorKind::LimitExceeded("extension payload size")
                | ErrorKind::LimitExceeded("byte string length")
        ));
    }

    #[cfg(feature = "serde_support")]
    #[test]
    fn serde_support_decodes_struct_list_map_option_and_enum() {
        use std::collections::BTreeMap;

        use serde::Deserialize;

        #[derive(Debug, Deserialize, PartialEq)]
        enum Status {
            New,
            Done,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Payload {
            id: String,
            tags: Vec<String>,
            counts: BTreeMap<String, i32>,
            maybe: Option<i32>,
            status: Status,
        }

        let schema = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "id", TypeDescriptor::String { max_len: None }),
            Field::new(
                2,
                "tags",
                TypeDescriptor::List {
                    max_count: None,
                    element: Box::new(TypeDescriptor::String { max_len: None }),
                },
            ),
            Field::new(
                3,
                "counts",
                TypeDescriptor::Map {
                    max_count: None,
                    key: Box::new(TypeDescriptor::String { max_len: None }),
                    value: Box::new(TypeDescriptor::I32),
                },
            ),
            Field::new(
                4,
                "maybe",
                TypeDescriptor::Optional(Box::new(TypeDescriptor::I32)),
            ),
            Field::new(
                5,
                "status",
                TypeDescriptor::Enum(vec!["New".to_string(), "Done".to_string()]),
            ),
        ]));
        let value = TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("payload-1"))),
            (
                2,
                TpackValue::List(vec![
                    TpackValue::String(Cow::Borrowed("a")),
                    TpackValue::String(Cow::Borrowed("b")),
                ]),
            ),
            (
                3,
                TpackValue::Map(vec![
                    ValueMapEntry {
                        key: TpackValue::String(Cow::Borrowed("x")),
                        value: TpackValue::I32(1),
                    },
                    ValueMapEntry {
                        key: TpackValue::String(Cow::Borrowed("y")),
                        value: TpackValue::I32(2),
                    },
                ]),
            ),
            (4, TpackValue::Optional(Some(Box::new(TpackValue::I32(42))))),
            (5, TpackValue::Enum(1)),
        ]);
        let bytes = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).unwrap();
        let decoded: Payload = tpack::serde_support::from_slice(&bytes).unwrap();

        let mut counts = BTreeMap::new();
        counts.insert("x".to_string(), 1);
        counts.insert("y".to_string(), 2);
        assert_eq!(
            decoded,
            Payload {
                id: "payload-1".to_string(),
                tags: vec!["a".to_string(), "b".to_string()],
                counts,
                maybe: Some(42),
                status: Status::Done,
            }
        );
    }

    #[cfg(feature = "serde_support")]
    #[test]
    fn serde_support_struct_field_matching_keeps_order_unknown_and_duplicate_behavior() {
        use serde::Deserialize;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Payload {
            id: String,
            qty: i32,
        }

        let schema = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "id", TypeDescriptor::String { max_len: None }),
            Field::new(2, "qty", TypeDescriptor::I32),
        ]));

        let decoded: Payload = tpack::serde_support::from_value(
            &schema,
            TpackValue::Struct(vec![
                (2, TpackValue::I32(7)),
                (99, TpackValue::String(Cow::Borrowed("ignored"))),
                (1, TpackValue::String(Cow::Borrowed("ord-1"))),
            ]),
        )
        .unwrap();
        assert_eq!(
            decoded,
            Payload {
                id: "ord-1".to_string(),
                qty: 7,
            }
        );

        let duplicate_error = tpack::serde_support::from_value::<Payload>(
            &schema,
            TpackValue::Struct(vec![
                (1, TpackValue::String(Cow::Borrowed("ord-1"))),
                (2, TpackValue::I32(7)),
                (1, TpackValue::String(Cow::Borrowed("ord-2"))),
            ]),
        )
        .unwrap_err();
        assert!(matches!(
            duplicate_error.kind(),
            ErrorKind::Invalid(message) if message == "duplicate struct field value"
        ));
    }

    #[cfg(all(feature = "serde_support", feature = "std"))]
    #[test]
    fn serde_support_builder_uses_registry_for_schema_ref_messages() {
        use serde::Deserialize;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Payload {
            id: String,
            qty: i32,
        }

        let schema = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "id", TypeDescriptor::String { max_len: None }),
            Field::new(2, "qty", TypeDescriptor::I32),
        ]));
        let value = TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("ord-1"))),
            (2, TpackValue::I32(7)),
        ]);
        let bytes = encode_message(
            &schema,
            &value,
            EnvelopeMode::SchemaRef,
            Some(b"example.payload.v1"),
        )
        .unwrap();

        let registry = tpack::StdSchemaRegistry::new();
        registry.insert(b"example.payload.v1", schema);

        let decoded: Payload = tpack::serde_support::Deserializer::new()
            .registry(&registry)
            .from_slice(&bytes)
            .unwrap();
        assert_eq!(
            decoded,
            Payload {
                id: "ord-1".to_string(),
                qty: 7,
            }
        );
    }

    #[test]
    fn data_type_roundtrips_cover_temporal_duration_interval_and_extension() {
        let schema = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "date", TypeDescriptor::Date),
            Field::new(2, "time", TypeDescriptor::Time),
            Field::new(3, "dt", TypeDescriptor::DateTime),
            Field::new(
                4,
                "ts",
                TypeDescriptor::Timestamp(tpack::TimestampPrecision::Nanoseconds),
            ),
            Field::new(5, "dur", TypeDescriptor::Duration),
            Field::new(6, "big_i", TypeDescriptor::BigInt),
            Field::new(7, "big_u", TypeDescriptor::BigUInt),
            Field::new(8, "cal", TypeDescriptor::CalendarInterval),
            Field::new(
                9,
                "ext",
                TypeDescriptor::Extension {
                    authority: "example".to_string(),
                    type_name: "opaque".to_string(),
                    schema_params: vec![1, 2, 3],
                },
            ),
        ]));
        let value = TpackValue::Struct(vec![
            (1, TpackValue::Date(-1)),
            (2, TpackValue::Time(1_000)),
            (
                3,
                TpackValue::DateTime {
                    days: 10,
                    nanos: 20,
                },
            ),
            (4, TpackValue::Timestamp(123)),
            (
                5,
                TpackValue::Duration(tpack::Duration {
                    seconds: -2,
                    nanos: -3,
                }),
            ),
            (6, TpackValue::BigInt(-9)),
            (7, TpackValue::BigUInt(9)),
            (
                8,
                TpackValue::CalendarInterval(tpack::CalendarInterval {
                    months: 1,
                    days: 2,
                    nanos: 3,
                }),
            ),
            (9, TpackValue::Extension(Cow::Borrowed(&[9, 8, 7]))),
        ]);
        let bytes = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).unwrap();
        assert_eq!(tpack::decode_message(&bytes).unwrap().value, value);
    }

    #[test]
    fn encoder_canonical_mode_sorts_map_entries() {
        let schema = Schema::new(TypeDescriptor::Map {
            max_count: None,
            key: Box::new(TypeDescriptor::String { max_len: None }),
            value: Box::new(TypeDescriptor::I32),
        });
        let value = TpackValue::Map(vec![
            ValueMapEntry {
                key: TpackValue::String(Cow::Borrowed("b")),
                value: TpackValue::I32(2),
            },
            ValueMapEntry {
                key: TpackValue::String(Cow::Borrowed("a")),
                value: TpackValue::I32(1),
            },
        ]);
        let mut encoder = tpack::Encoder::with_options(EncodeOptions {
            canonical: CanonicalMode::Strict,
            ..EncodeOptions::default()
        });
        encoder
            .encode_message(&schema, &value, EnvelopeMode::FullSchema, None)
            .unwrap();
        let bytes = encoder.into_vec();
        let mut decoder = Decoder::with_options(
            &bytes,
            DecodeOptions {
                canonical: CanonicalMode::Strict,
                ..DecodeOptions::default()
            },
        );
        assert_eq!(
            decoder.decode_message().unwrap().value,
            TpackValue::Map(vec![
                ValueMapEntry {
                    key: TpackValue::String(Cow::Borrowed("a")),
                    value: TpackValue::I32(1),
                },
                ValueMapEntry {
                    key: TpackValue::String(Cow::Borrowed("b")),
                    value: TpackValue::I32(2),
                },
            ])
        );
    }

    #[test]
    fn union_full_schema_example_from_draft_decodes() {
        let schema = Schema::new(TypeDescriptor::Union(vec![
            Variant::new(
                "fiat",
                TypeDescriptor::DecimalFixed {
                    precision: 18,
                    scale: 4,
                },
            ),
            Variant::new("label", TypeDescriptor::String { max_len: None }),
            Variant::new("wei", TypeDescriptor::BigUInt),
        ]));
        let value = TpackValue::Union {
            index: 0,
            value: Box::new(TpackValue::DecimalFixed(1_285_000)),
        };
        let bytes = encode_message(&schema, &value, EnvelopeMode::FullSchema, None).unwrap();
        assert_eq!(
            &bytes[..],
            &[
                0x54, 0x50, 0x41, 0x4B, 0x01, 0x00, 0x16, 0x23, 0x03, 0x04, 0x66, 0x69, 0x61, 0x74,
                0x0D, 0x12, 0x04, 0x05, 0x6C, 0x61, 0x62, 0x65, 0x6C, 0x0F, 0x03, 0x77, 0x65, 0x69,
                0x19, 0x00, 0x90, 0xEE, 0x9C, 0x01,
            ]
        );
        assert_eq!(tpack::decode_message(&bytes).unwrap().value, value);
    }
}
