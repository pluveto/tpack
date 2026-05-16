use tpack::{Message, TpackValue, TypeDescriptor};

use crate::cli::InspectSection;

use super::shared;

pub(super) struct JsonFormatter<'a> {
    out: &'a mut String,
}

impl<'a> JsonFormatter<'a> {
    pub(super) fn write(message: &Message<'_>, section: InspectSection, out: &'a mut String) {
        Self { out }.write_message(message, section);
    }

    fn write_message(&mut self, message: &Message<'_>, section: InspectSection) {
        match section {
            InspectSection::All => {
                shared::line(self.out, 0, "{");
                shared::write_json_property(self.out, 1, "envelope", |out, indent| {
                    Self::write_envelope(message, out, indent);
                });
                shared::trim_last_newline(self.out);
                self.out.push_str(",\n");
                shared::line(self.out, 1, "\"schema\": {");
                shared::write_json_property(self.out, 2, "root", |out, indent| {
                    Self::write_type(&message.schema.root, out, indent);
                });
                shared::trim_last_newline(self.out);
                self.out.push_str("\n  },\n");
                shared::write_json_property(self.out, 1, "value", |out, indent| {
                    Self::write_value(&message.value, Some(&message.schema.root), out, indent);
                });
                shared::trim_last_newline(self.out);
                self.out.push_str("\n}\n");
            }
            InspectSection::Envelope => Self::write_envelope(message, self.out, 0),
            InspectSection::Schema => Self::write_type(&message.schema.root, self.out, 0),
            InspectSection::Value => {
                Self::write_value(&message.value, Some(&message.schema.root), self.out, 0)
            }
        }
    }

    fn write_envelope(message: &Message<'_>, out: &mut String, indent: usize) {
        shared::line(out, indent, "{");
        shared::line(
            out,
            indent + 1,
            &format!(
                "\"mode\": \"{}\",",
                shared::json_escape(shared::envelope_mode_name(message.envelope.mode))
            ),
        );
        match &message.envelope.schema_id {
            Some(schema_id) => shared::line(
                out,
                indent + 1,
                &format!(
                    "\"schema_id\": \"{}\",",
                    shared::bytes_hex(schema_id.as_bytes())
                ),
            ),
            None => shared::line(out, indent + 1, "\"schema_id\": null,"),
        }
        shared::line(
            out,
            indent + 1,
            &format!(
                "\"used_cached_schema\": {}",
                message.envelope.used_cached_schema
            ),
        );
        shared::line(out, indent, "}");
    }

    fn write_type(ty: &TypeDescriptor, out: &mut String, indent: usize) {
        match ty {
            TypeDescriptor::Struct(fields) => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"struct\",");
                shared::line(out, indent + 1, "\"fields\": [");
                for (index, field) in fields.iter().enumerate() {
                    shared::line(out, indent + 2, "{");
                    shared::line(out, indent + 3, &format!("\"id\": {},", field.id));
                    shared::line(
                        out,
                        indent + 3,
                        &format!("\"name\": \"{}\",", shared::json_escape(&field.name)),
                    );
                    shared::write_json_property(out, indent + 3, "type", |out, indent| {
                        Self::write_type(&field.ty, out, indent);
                    });
                    shared::trim_last_newline(out);
                    out.push('\n');
                    shared::line(
                        out,
                        indent + 2,
                        if index + 1 == fields.len() { "}" } else { "}," },
                    );
                }
                shared::line(out, indent + 1, "]");
                shared::line(out, indent, "}");
            }
            TypeDescriptor::List { max_count, element } => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"list\",");
                shared::write_optional_u64_json(out, indent + 1, "max_count", *max_count, true);
                shared::write_json_property(out, indent + 1, "element", |out, indent| {
                    Self::write_type(element, out, indent);
                });
                shared::trim_last_newline(out);
                out.push('\n');
                shared::line(out, indent, "}");
            }
            TypeDescriptor::Map {
                max_count,
                key,
                value,
            } => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"map\",");
                shared::write_optional_u64_json(out, indent + 1, "max_count", *max_count, true);
                shared::write_json_property(out, indent + 1, "key", |out, indent| {
                    Self::write_type(key, out, indent);
                });
                shared::trim_last_newline(out);
                out.push_str(",\n");
                shared::write_json_property(out, indent + 1, "value", |out, indent| {
                    Self::write_type(value, out, indent);
                });
                shared::trim_last_newline(out);
                out.push('\n');
                shared::line(out, indent, "}");
            }
            TypeDescriptor::Union(variants) => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"union\",");
                shared::line(out, indent + 1, "\"variants\": [");
                for (index, variant) in variants.iter().enumerate() {
                    shared::line(out, indent + 2, "{");
                    shared::line(
                        out,
                        indent + 3,
                        &format!("\"name\": \"{}\",", shared::json_escape(&variant.name)),
                    );
                    shared::write_json_property(out, indent + 3, "type", |out, indent| {
                        Self::write_type(&variant.ty, out, indent);
                    });
                    shared::trim_last_newline(out);
                    out.push('\n');
                    shared::line(
                        out,
                        indent + 2,
                        if index + 1 == variants.len() {
                            "}"
                        } else {
                            "},"
                        },
                    );
                }
                shared::line(out, indent + 1, "]");
                shared::line(out, indent, "}");
            }
            TypeDescriptor::Enum(symbols) => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"enum\",");
                shared::line(out, indent + 1, "\"symbols\": [");
                for (index, symbol) in symbols.iter().enumerate() {
                    shared::line(
                        out,
                        indent + 2,
                        &format!(
                            "\"{}\"{}",
                            shared::json_escape(symbol),
                            shared::comma(index + 1, symbols.len())
                        ),
                    );
                }
                shared::line(out, indent + 1, "]");
                shared::line(out, indent, "}");
            }
            TypeDescriptor::Optional(inner) => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"optional\",");
                shared::write_json_property(out, indent + 1, "inner", |out, indent| {
                    Self::write_type(inner, out, indent);
                });
                shared::trim_last_newline(out);
                out.push('\n');
                shared::line(out, indent, "}");
            }
            TypeDescriptor::DecimalFixed { precision, scale } => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"decimal_fixed\",");
                shared::line(out, indent + 1, &format!("\"precision\": {precision},"));
                shared::line(out, indent + 1, &format!("\"scale\": {scale}"));
                shared::line(out, indent, "}");
            }
            TypeDescriptor::String { max_len } => {
                shared::type_limit_json(out, indent, "string", "max_len", *max_len)
            }
            TypeDescriptor::Bytes { max_len } => {
                shared::type_limit_json(out, indent, "bytes", "max_len", *max_len)
            }
            TypeDescriptor::Timestamp(precision) => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"timestamp\",");
                shared::line(
                    out,
                    indent + 1,
                    &format!(
                        "\"precision\": \"{}\"",
                        shared::timestamp_precision_name(*precision)
                    ),
                );
                shared::line(out, indent, "}");
            }
            TypeDescriptor::Extension {
                authority,
                type_name,
                schema_params,
            } => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, "\"type\": \"extension\",");
                shared::line(
                    out,
                    indent + 1,
                    &format!("\"authority\": \"{}\",", shared::json_escape(authority)),
                );
                shared::line(
                    out,
                    indent + 1,
                    &format!("\"type_name\": \"{}\",", shared::json_escape(type_name)),
                );
                shared::line(
                    out,
                    indent + 1,
                    &format!(
                        "\"schema_params\": \"{}\"",
                        shared::bytes_hex(schema_params)
                    ),
                );
                shared::line(out, indent, "}");
            }
            _ => {
                shared::line(out, indent, "{");
                shared::line(
                    out,
                    indent + 1,
                    &format!("\"type\": \"{}\"", shared::type_inline(ty)),
                );
                shared::line(out, indent, "}");
            }
        }
    }

    fn write_value(
        value: &TpackValue<'_>,
        ty: Option<&TypeDescriptor>,
        out: &mut String,
        indent: usize,
    ) {
        match value {
            TpackValue::Struct(values) => {
                shared::line(out, indent, "{");
                for (index, (field_id, field_value)) in values.iter().enumerate() {
                    let field = shared::find_struct_field(ty, *field_id);
                    let field_ty = field.map(|field| &field.ty);
                    let name = field
                        .map(|field| field.name.as_str())
                        .unwrap_or("<unknown>");
                    let key = format!("{name}#{field_id}");
                    shared::write_json_property(out, indent + 1, &key, |out, indent| {
                        Self::write_value(field_value, field_ty, out, indent);
                    });
                    shared::trim_last_newline(out);
                    out.push_str(shared::comma(index + 1, values.len()));
                    out.push('\n');
                }
                shared::line(out, indent, "}");
            }
            TpackValue::List(values) => {
                shared::line(out, indent, "[");
                let element_ty = shared::list_element_type(ty);
                for (index, item) in values.iter().enumerate() {
                    Self::write_value(item, element_ty, out, indent + 1);
                    shared::trim_last_newline(out);
                    out.push_str(shared::comma(index + 1, values.len()));
                    out.push('\n');
                }
                shared::line(out, indent, "]");
            }
            TpackValue::Map(entries) => {
                shared::line(out, indent, "[");
                let (key_ty, value_ty) = shared::map_types(ty);
                for (index, entry) in entries.iter().enumerate() {
                    shared::line(out, indent + 1, "{");
                    shared::write_json_property(out, indent + 2, "key", |out, indent| {
                        Self::write_value(&entry.key, key_ty, out, indent);
                    });
                    shared::trim_last_newline(out);
                    out.push_str(",\n");
                    shared::write_json_property(out, indent + 2, "value", |out, indent| {
                        Self::write_value(&entry.value, value_ty, out, indent);
                    });
                    shared::trim_last_newline(out);
                    out.push('\n');
                    shared::line(
                        out,
                        indent + 1,
                        if index + 1 == entries.len() {
                            "}"
                        } else {
                            "},"
                        },
                    );
                }
                shared::line(out, indent, "]");
            }
            TpackValue::Union { index, value } => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, &format!("\"variant\": {index},"));
                shared::write_json_property(out, indent + 1, "value", |out, indent| {
                    Self::write_value(value, shared::union_variant_type(ty, *index), out, indent);
                });
                shared::trim_last_newline(out);
                out.push('\n');
                shared::line(out, indent, "}");
            }
            TpackValue::Optional(Some(value)) => {
                Self::write_value(value, shared::optional_inner_type(ty), out, indent);
            }
            TpackValue::Optional(None) | TpackValue::Null => shared::line(out, indent, "null"),
            TpackValue::Bool(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::I8(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::I16(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::I32(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::I64(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::U8(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::U16(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::U32(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::U64(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::F32(value) => {
                shared::line(out, indent, &shared::float_json(f64::from(*value)))
            }
            TpackValue::F64(value) => shared::line(out, indent, &shared::float_json(*value)),
            TpackValue::Decimal(value) => shared::decimal_json(out, indent, *value),
            TpackValue::DecimalFixed(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::String(value) => {
                shared::line(out, indent, &format!("\"{}\"", shared::json_escape(value)))
            }
            TpackValue::Bytes(value) => {
                shared::line(out, indent, &format!("\"{}\"", shared::bytes_hex(value)))
            }
            TpackValue::Date(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::Time(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::DateTime { days, nanos } => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, &format!("\"days\": {days},"));
                shared::line(out, indent + 1, &format!("\"nanos\": {nanos}"));
                shared::line(out, indent, "}");
            }
            TpackValue::DateTimeTz {
                days,
                nanos,
                timezone,
            } => {
                shared::line(out, indent, "{");
                shared::line(out, indent + 1, &format!("\"days\": {days},"));
                shared::line(out, indent + 1, &format!("\"nanos\": {nanos},"));
                shared::line(
                    out,
                    indent + 1,
                    &format!("\"timezone\": \"{}\"", shared::json_escape(timezone)),
                );
                shared::line(out, indent, "}");
            }
            TpackValue::Timestamp(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::Duration(value) => shared::duration_json(out, indent, *value),
            TpackValue::BigInt(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::BigUInt(value) => shared::line(out, indent, &value.to_string()),
            TpackValue::CalendarInterval(value) => {
                shared::calendar_interval_json(out, indent, *value)
            }
            TpackValue::Enum(index) => shared::line(out, indent, &index.to_string()),
            TpackValue::Extension(value) => {
                shared::line(out, indent, &format!("\"{}\"", shared::bytes_hex(value)))
            }
        }
    }
}
