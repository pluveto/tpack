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

    use tpack::{Envelope, EnvelopeMode, Field, Schema, TpackValue, TypeDescriptor};

    use super::*;

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
        let mut out = String::new();
        json::JsonFormatter::write(&sample_message(), InspectSection::Value, &mut out);

        assert_eq!(
            out,
            r#"{
  "id#1": "prod_001",
  "price#2": 2999900,
  "qty#3": 10,
  "active#4": true
}
"#
        );
    }
}
