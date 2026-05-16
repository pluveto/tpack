use tpack::{Message, TpackValue, TypeDescriptor};

use crate::cli::InspectSection;

use super::shared::{
    bytes_hex, calendar_interval_json, comma, decimal_json, duration_json, envelope_mode_name,
    find_struct_field, float_json, json_escape, line, list_element_type, map_types,
    optional_inner_type, timestamp_precision_name, trim_last_newline, type_inline, type_limit_json,
    union_variant_type, write_json_property, write_optional_u64_json,
};

pub(super) fn write(message: &Message<'_>, section: InspectSection, out: &mut String) {
    JsonFormatter { out }.write_message(message, section);
}

struct JsonFormatter<'a> {
    out: &'a mut String,
}

impl JsonFormatter<'_> {
    fn write_message(&mut self, message: &Message<'_>, section: InspectSection) {
        match section {
            InspectSection::All => {
                line(self.out, 0, "{");
                write_json_property(self.out, 1, "envelope", |out, indent| {
                    Self::write_envelope(message, out, indent);
                });
                trim_last_newline(self.out);
                self.out.push_str(",\n");
                line(self.out, 1, "\"schema\": {");
                write_json_property(self.out, 2, "root", |out, indent| {
                    Self::write_type(&message.schema.root, out, indent);
                });
                trim_last_newline(self.out);
                self.out.push_str("\n  },\n");
                write_json_property(self.out, 1, "value", |out, indent| {
                    Self::write_value(&message.value, Some(&message.schema.root), out, indent);
                });
                trim_last_newline(self.out);
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
        line(out, indent, "{");
        line(
            out,
            indent + 1,
            &format!(
                "\"mode\": \"{}\",",
                json_escape(envelope_mode_name(message.envelope.mode))
            ),
        );
        match &message.envelope.schema_id {
            Some(schema_id) => line(
                out,
                indent + 1,
                &format!("\"schema_id\": \"{}\",", bytes_hex(schema_id.as_bytes())),
            ),
            None => line(out, indent + 1, "\"schema_id\": null,"),
        }
        line(
            out,
            indent + 1,
            &format!(
                "\"used_cached_schema\": {}",
                message.envelope.used_cached_schema
            ),
        );
        line(out, indent, "}");
    }

    fn write_type(ty: &TypeDescriptor, out: &mut String, indent: usize) {
        match ty {
            TypeDescriptor::Struct(fields) => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"struct\",");
                line(out, indent + 1, "\"fields\": [");
                for (index, field) in fields.iter().enumerate() {
                    line(out, indent + 2, "{");
                    line(out, indent + 3, &format!("\"id\": {},", field.id));
                    line(
                        out,
                        indent + 3,
                        &format!("\"name\": \"{}\",", json_escape(&field.name)),
                    );
                    write_json_property(out, indent + 3, "type", |out, indent| {
                        Self::write_type(&field.ty, out, indent);
                    });
                    trim_last_newline(out);
                    out.push('\n');
                    line(
                        out,
                        indent + 2,
                        if index + 1 == fields.len() { "}" } else { "}," },
                    );
                }
                line(out, indent + 1, "]");
                line(out, indent, "}");
            }
            TypeDescriptor::List { max_count, element } => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"list\",");
                write_optional_u64_json(out, indent + 1, "max_count", *max_count, true);
                write_json_property(out, indent + 1, "element", |out, indent| {
                    Self::write_type(element, out, indent);
                });
                trim_last_newline(out);
                out.push('\n');
                line(out, indent, "}");
            }
            TypeDescriptor::Map {
                max_count,
                key,
                value,
            } => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"map\",");
                write_optional_u64_json(out, indent + 1, "max_count", *max_count, true);
                write_json_property(out, indent + 1, "key", |out, indent| {
                    Self::write_type(key, out, indent);
                });
                trim_last_newline(out);
                out.push_str(",\n");
                write_json_property(out, indent + 1, "value", |out, indent| {
                    Self::write_type(value, out, indent);
                });
                trim_last_newline(out);
                out.push('\n');
                line(out, indent, "}");
            }
            TypeDescriptor::Union(variants) => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"union\",");
                line(out, indent + 1, "\"variants\": [");
                for (index, variant) in variants.iter().enumerate() {
                    line(out, indent + 2, "{");
                    line(
                        out,
                        indent + 3,
                        &format!("\"name\": \"{}\",", json_escape(&variant.name)),
                    );
                    write_json_property(out, indent + 3, "type", |out, indent| {
                        Self::write_type(&variant.ty, out, indent);
                    });
                    trim_last_newline(out);
                    out.push('\n');
                    line(
                        out,
                        indent + 2,
                        if index + 1 == variants.len() {
                            "}"
                        } else {
                            "},"
                        },
                    );
                }
                line(out, indent + 1, "]");
                line(out, indent, "}");
            }
            TypeDescriptor::Enum(symbols) => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"enum\",");
                line(out, indent + 1, "\"symbols\": [");
                for (index, symbol) in symbols.iter().enumerate() {
                    line(
                        out,
                        indent + 2,
                        &format!(
                            "\"{}\"{}",
                            json_escape(symbol),
                            comma(index + 1, symbols.len())
                        ),
                    );
                }
                line(out, indent + 1, "]");
                line(out, indent, "}");
            }
            TypeDescriptor::Optional(inner) => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"optional\",");
                write_json_property(out, indent + 1, "inner", |out, indent| {
                    Self::write_type(inner, out, indent);
                });
                trim_last_newline(out);
                out.push('\n');
                line(out, indent, "}");
            }
            TypeDescriptor::DecimalFixed { precision, scale } => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"decimal_fixed\",");
                line(out, indent + 1, &format!("\"precision\": {precision},"));
                line(out, indent + 1, &format!("\"scale\": {scale}"));
                line(out, indent, "}");
            }
            TypeDescriptor::String { max_len } => {
                type_limit_json(out, indent, "string", "max_len", *max_len)
            }
            TypeDescriptor::Bytes { max_len } => {
                type_limit_json(out, indent, "bytes", "max_len", *max_len)
            }
            TypeDescriptor::Timestamp(precision) => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"timestamp\",");
                line(
                    out,
                    indent + 1,
                    &format!(
                        "\"precision\": \"{}\"",
                        timestamp_precision_name(*precision)
                    ),
                );
                line(out, indent, "}");
            }
            TypeDescriptor::Extension {
                authority,
                type_name,
                schema_params,
            } => {
                line(out, indent, "{");
                line(out, indent + 1, "\"type\": \"extension\",");
                line(
                    out,
                    indent + 1,
                    &format!("\"authority\": \"{}\",", json_escape(authority)),
                );
                line(
                    out,
                    indent + 1,
                    &format!("\"type_name\": \"{}\",", json_escape(type_name)),
                );
                line(
                    out,
                    indent + 1,
                    &format!("\"schema_params\": \"{}\"", bytes_hex(schema_params)),
                );
                line(out, indent, "}");
            }
            _ => {
                line(out, indent, "{");
                line(
                    out,
                    indent + 1,
                    &format!("\"type\": \"{}\"", type_inline(ty)),
                );
                line(out, indent, "}");
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
                line(out, indent, "{");
                for (index, (field_id, field_value)) in values.iter().enumerate() {
                    let field = find_struct_field(ty, *field_id);
                    let field_ty = field.map(|field| &field.ty);
                    let name = field
                        .map(|field| field.name.as_str())
                        .unwrap_or("<unknown>");
                    let key = format!("{name}#{field_id}");
                    write_json_property(out, indent + 1, &key, |out, indent| {
                        Self::write_value(field_value, field_ty, out, indent);
                    });
                    trim_last_newline(out);
                    out.push_str(comma(index + 1, values.len()));
                    out.push('\n');
                }
                line(out, indent, "}");
            }
            TpackValue::List(values) => {
                line(out, indent, "[");
                let element_ty = list_element_type(ty);
                for (index, item) in values.iter().enumerate() {
                    Self::write_value(item, element_ty, out, indent + 1);
                    trim_last_newline(out);
                    out.push_str(comma(index + 1, values.len()));
                    out.push('\n');
                }
                line(out, indent, "]");
            }
            TpackValue::Map(entries) => {
                line(out, indent, "[");
                let (key_ty, value_ty) = map_types(ty);
                for (index, entry) in entries.iter().enumerate() {
                    line(out, indent + 1, "{");
                    write_json_property(out, indent + 2, "key", |out, indent| {
                        Self::write_value(&entry.key, key_ty, out, indent);
                    });
                    trim_last_newline(out);
                    out.push_str(",\n");
                    write_json_property(out, indent + 2, "value", |out, indent| {
                        Self::write_value(&entry.value, value_ty, out, indent);
                    });
                    trim_last_newline(out);
                    out.push('\n');
                    line(
                        out,
                        indent + 1,
                        if index + 1 == entries.len() {
                            "}"
                        } else {
                            "},"
                        },
                    );
                }
                line(out, indent, "]");
            }
            TpackValue::Union { index, value } => {
                line(out, indent, "{");
                line(out, indent + 1, &format!("\"variant\": {index},"));
                write_json_property(out, indent + 1, "value", |out, indent| {
                    Self::write_value(value, union_variant_type(ty, *index), out, indent);
                });
                trim_last_newline(out);
                out.push('\n');
                line(out, indent, "}");
            }
            TpackValue::Optional(Some(value)) => {
                Self::write_value(value, optional_inner_type(ty), out, indent);
            }
            TpackValue::Optional(None) | TpackValue::Null => line(out, indent, "null"),
            TpackValue::Bool(value) => line(out, indent, &value.to_string()),
            TpackValue::I8(value) => line(out, indent, &value.to_string()),
            TpackValue::I16(value) => line(out, indent, &value.to_string()),
            TpackValue::I32(value) => line(out, indent, &value.to_string()),
            TpackValue::I64(value) => line(out, indent, &value.to_string()),
            TpackValue::U8(value) => line(out, indent, &value.to_string()),
            TpackValue::U16(value) => line(out, indent, &value.to_string()),
            TpackValue::U32(value) => line(out, indent, &value.to_string()),
            TpackValue::U64(value) => line(out, indent, &value.to_string()),
            TpackValue::F32(value) => line(out, indent, &float_json(f64::from(*value))),
            TpackValue::F64(value) => line(out, indent, &float_json(*value)),
            TpackValue::Decimal(value) => decimal_json(out, indent, *value),
            TpackValue::DecimalFixed(value) => line(out, indent, &value.to_string()),
            TpackValue::String(value) => line(out, indent, &format!("\"{}\"", json_escape(value))),
            TpackValue::Bytes(value) => line(out, indent, &format!("\"{}\"", bytes_hex(value))),
            TpackValue::Date(value) => line(out, indent, &value.to_string()),
            TpackValue::Time(value) => line(out, indent, &value.to_string()),
            TpackValue::DateTime { days, nanos } => {
                line(out, indent, "{");
                line(out, indent + 1, &format!("\"days\": {days},"));
                line(out, indent + 1, &format!("\"nanos\": {nanos}"));
                line(out, indent, "}");
            }
            TpackValue::DateTimeTz {
                days,
                nanos,
                timezone,
            } => {
                line(out, indent, "{");
                line(out, indent + 1, &format!("\"days\": {days},"));
                line(out, indent + 1, &format!("\"nanos\": {nanos},"));
                line(
                    out,
                    indent + 1,
                    &format!("\"timezone\": \"{}\"", json_escape(timezone)),
                );
                line(out, indent, "}");
            }
            TpackValue::Timestamp(value) => line(out, indent, &value.to_string()),
            TpackValue::Duration(value) => duration_json(out, indent, *value),
            TpackValue::BigInt(value) => line(out, indent, &value.to_string()),
            TpackValue::BigUInt(value) => line(out, indent, &value.to_string()),
            TpackValue::CalendarInterval(value) => calendar_interval_json(out, indent, *value),
            TpackValue::Enum(index) => line(out, indent, &index.to_string()),
            TpackValue::Extension(value) => line(out, indent, &format!("\"{}\"", bytes_hex(value))),
        }
    }
}
