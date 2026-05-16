use tpack::Message;

use crate::cli::InspectSection;

mod json;
mod shared;
mod tree;

pub fn print_tree(message: &Message<'_>, section: InspectSection) {
    let mut out = String::new();
    tree::TreeFormatter::write(message, section, &mut out);
    print!("{out}");
}

pub fn print_json(message: &Message<'_>, section: InspectSection) {
    let mut out = String::new();
    json::JsonFormatter::write(message, section, &mut out);
    print!("{out}");
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, sync::Arc};

    use tpack::{
        Envelope, EnvelopeMode, Field, Message, Schema, SchemaId, TpackValue, TypeDescriptor,
        ValueMapEntry, Variant,
    };

    use super::*;

    fn render_json(message: &Message<'_>, section: InspectSection) -> String {
        let mut out = String::new();
        json::JsonFormatter::write(message, section, &mut out);
        out
    }

    fn sample_message() -> Message<'static> {
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

        Message {
            envelope: Envelope {
                mode: EnvelopeMode::FullSchema,
                schema_id: None,
                used_cached_schema: false,
            },
            schema: Arc::new(schema),
            value,
        }
    }

    fn json_edge_case_message() -> Message<'static> {
        let schema = Schema::new(TypeDescriptor::Struct(vec![
            Field::new(1, "label", TypeDescriptor::String { max_len: None }),
            Field::new(2, "score", TypeDescriptor::F64),
            Field::new(
                3,
                "attrs",
                TypeDescriptor::Map {
                    max_count: None,
                    key: Box::new(TypeDescriptor::String { max_len: None }),
                    value: Box::new(TypeDescriptor::Optional(Box::new(TypeDescriptor::I32))),
                },
            ),
            Field::new(
                4,
                "choice",
                TypeDescriptor::Union(vec![
                    Variant::new("count", TypeDescriptor::I32),
                    Variant::new(
                        "state",
                        TypeDescriptor::Enum(vec!["new".to_string(), "done".to_string()]),
                    ),
                ]),
            ),
            Field::new(
                5,
                "payload",
                TypeDescriptor::Extension {
                    authority: "acme.io".to_string(),
                    type_name: "widget".to_string(),
                    schema_params: vec![1, 2, 3],
                },
            ),
        ]));
        let value = TpackValue::Struct(vec![
            (
                1,
                TpackValue::String(Cow::Borrowed("line\n\"quoted\"\ttext")),
            ),
            (2, TpackValue::F64(f64::NAN)),
            (
                3,
                TpackValue::Map(vec![
                    ValueMapEntry {
                        key: TpackValue::String(Cow::Borrowed("x")),
                        value: TpackValue::Optional(Some(Box::new(TpackValue::I32(7)))),
                    },
                    ValueMapEntry {
                        key: TpackValue::String(Cow::Borrowed("y")),
                        value: TpackValue::Optional(None),
                    },
                ]),
            ),
            (
                4,
                TpackValue::Union {
                    index: 1,
                    value: Box::new(TpackValue::Enum(1)),
                },
            ),
            (5, TpackValue::Extension(Cow::Borrowed(&[0x0a, 0x0b]))),
        ]);

        Message {
            envelope: Envelope {
                mode: EnvelopeMode::FullSchema,
                schema_id: None,
                used_cached_schema: false,
            },
            schema: Arc::new(schema),
            value,
        }
    }

    #[test]
    fn tree_inspect_formats_full_message() {
        let mut out = String::new();
        tree::TreeFormatter::write(&sample_message(), InspectSection::All, &mut out);

        assert_eq!(
            out,
            r#"message
  envelope
    mode: FullSchema
    schema_id: null
    used_cached_schema: false
  schema
    struct
      id#1: string(max_len=64)
      price#2: decimal_fixed(precision=10, scale=2)
      qty#3: i32
      active#4: bool
  value
    struct
      id#1: "prod_001"
      price#2: 2999900
      qty#3: 10
      active#4: true
"#
        );
    }

    #[test]
    fn json_inspect_can_format_value_only() {
        assert_eq!(
            render_json(&sample_message(), InspectSection::Value),
            r#"{
  "id#1": "prod_001",
  "price#2": 2999900,
  "qty#3": 10,
  "active#4": true
}
"#
        );
    }

    #[test]
    fn json_inspect_all_wraps_envelope_schema_and_value() {
        let out = render_json(&sample_message(), InspectSection::All);

        assert!(out.starts_with("{\n  \"envelope\": {\n    \"mode\": \"FullSchema\","));
        assert!(
            out.contains("  },\n  \"schema\": {\n    \"root\": {\n      \"type\": \"struct\",")
        );
        assert!(out.contains("  },\n  \"value\": {\n    \"id#1\": \"prod_001\","));
        assert!(out.ends_with("  }\n}\n"));
    }

    #[test]
    fn json_inspect_formats_envelope_with_schema_id() {
        let mut message = sample_message();
        message.envelope.mode = EnvelopeMode::SchemaRef;
        message.envelope.schema_id = Some(SchemaId::borrowed(&[0xde, 0xad, 0xbe, 0xef]));
        message.envelope.used_cached_schema = true;

        assert_eq!(
            render_json(&message, InspectSection::Envelope),
            r#"{
  "mode": "SchemaRef",
  "schema_id": "0xdeadbeef",
  "used_cached_schema": true
}
"#
        );
    }

    #[test]
    fn json_inspect_formats_nested_schema_types() {
        assert_eq!(
            render_json(&json_edge_case_message(), InspectSection::Schema),
            r#"{
  "type": "struct",
  "fields": [
    {
      "id": 1,
      "name": "label",
      "type": {
        "type": "string",
        "max_len": null
      }
    },
    {
      "id": 2,
      "name": "score",
      "type": {
        "type": "f64"
      }
    },
    {
      "id": 3,
      "name": "attrs",
      "type": {
        "type": "map",
        "max_count": null,
        "key": {
          "type": "string",
          "max_len": null
        },
        "value": {
          "type": "optional",
          "inner": {
            "type": "i32"
          }
        }
      }
    },
    {
      "id": 4,
      "name": "choice",
      "type": {
        "type": "union",
        "variants": [
          {
            "name": "count",
            "type": {
              "type": "i32"
            }
          },
          {
            "name": "state",
            "type": {
              "type": "enum",
              "symbols": [
                "new",
                "done"
              ]
            }
          }
        ]
      }
    },
    {
      "id": 5,
      "name": "payload",
      "type": {
        "type": "extension",
        "authority": "acme.io",
        "type_name": "widget",
        "schema_params": "0x010203"
      }
    }
  ]
}
"#
        );
    }

    #[test]
    fn json_inspect_formats_json_specific_value_encodings() {
        assert_eq!(
            render_json(&json_edge_case_message(), InspectSection::Value),
            r#"{
  "label#1": "line\n\"quoted\"\ttext",
  "score#2": "NaN",
  "attrs#3": [
    {
      "key": "x",
      "value": 7
    },
    {
      "key": "y",
      "value": null
    }
  ],
  "choice#4": {
    "variant": 1,
    "value": 1
  },
  "payload#5": "0x0a0b"
}
"#
        );
    }
}
