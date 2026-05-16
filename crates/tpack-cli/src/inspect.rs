use std::fmt::Write;

use tpack::{
    CalendarInterval, Decimal, Duration, EnvelopeMode, Message, TimestampPrecision, TpackValue,
    TypeDescriptor, ValueMapEntry,
};

use crate::cli::InspectSection;

pub fn print_tree(message: &Message<'_>, section: InspectSection) {
    let mut out = String::new();
    write_tree(message, section, &mut out);
    print!("{out}");
}

pub fn print_json(message: &Message<'_>, section: InspectSection) {
    let mut out = String::new();
    write_json(message, section, &mut out);
    print!("{out}");
}

fn write_tree(message: &Message<'_>, section: InspectSection, out: &mut String) {
    match section {
        InspectSection::All => {
            line(out, 0, "message");
            write_envelope_tree(message, out, 1);

            line(out, 1, "schema");
            write_type_tree(&message.schema.root, out, 2);

            line(out, 1, "value");
            write_value_tree(&message.value, Some(&message.schema.root), out, 2);
        }
        InspectSection::Envelope => write_envelope_tree(message, out, 0),
        InspectSection::Schema => write_type_tree(&message.schema.root, out, 0),
        InspectSection::Value => {
            write_value_tree(&message.value, Some(&message.schema.root), out, 0)
        }
    }
}

fn write_envelope_tree(message: &Message<'_>, out: &mut String, indent: usize) {
    line(out, indent, "envelope");
    line(
        out,
        indent + 1,
        &format!("mode: {}", envelope_mode_name(message.envelope.mode)),
    );
    match &message.envelope.schema_id {
        Some(schema_id) => line(
            out,
            indent + 1,
            &format!("schema_id: {}", bytes_hex(schema_id.as_bytes())),
        ),
        None => line(out, indent + 1, "schema_id: null"),
    }
    line(
        out,
        indent + 1,
        &format!(
            "used_cached_schema: {}",
            message.envelope.used_cached_schema
        ),
    );
}

fn write_type_tree(ty: &TypeDescriptor, out: &mut String, indent: usize) {
    match ty {
        TypeDescriptor::Struct(fields) => {
            line(out, indent, "struct");
            for field in fields {
                line(
                    out,
                    indent + 1,
                    &format!("{}#{}: {}", field.name, field.id, type_inline(&field.ty)),
                );
                if type_needs_tree(&field.ty) {
                    write_type_tree(&field.ty, out, indent + 2);
                }
            }
        }
        TypeDescriptor::List { max_count, element } => {
            line(
                out,
                indent,
                &format!("list{}", optional_limit("max_count", *max_count)),
            );
            write_type_tree(element, out, indent + 1);
        }
        TypeDescriptor::Map {
            max_count,
            key,
            value,
        } => {
            line(
                out,
                indent,
                &format!("map{}", optional_limit("max_count", *max_count)),
            );
            line(out, indent + 1, &format!("key: {}", type_inline(key)));
            if type_needs_tree(key) {
                write_type_tree(key, out, indent + 2);
            }
            line(out, indent + 1, &format!("value: {}", type_inline(value)));
            if type_needs_tree(value) {
                write_type_tree(value, out, indent + 2);
            }
        }
        TypeDescriptor::Union(variants) => {
            line(out, indent, "union");
            for (index, variant) in variants.iter().enumerate() {
                line(
                    out,
                    indent + 1,
                    &format!("#{index} {}: {}", variant.name, type_inline(&variant.ty)),
                );
                if type_needs_tree(&variant.ty) {
                    write_type_tree(&variant.ty, out, indent + 2);
                }
            }
        }
        TypeDescriptor::Enum(symbols) => {
            line(out, indent, "enum");
            for (index, symbol) in symbols.iter().enumerate() {
                line(out, indent + 1, &format!("#{index}: {symbol}"));
            }
        }
        TypeDescriptor::Optional(inner) => {
            line(out, indent, &format!("optional: {}", type_inline(inner)));
            if type_needs_tree(inner) {
                write_type_tree(inner, out, indent + 1);
            }
        }
        _ => line(out, indent, &type_inline(ty)),
    }
}

fn write_value_tree(
    value: &TpackValue<'_>,
    ty: Option<&TypeDescriptor>,
    out: &mut String,
    indent: usize,
) {
    match value {
        TpackValue::Struct(values) => {
            line(out, indent, "struct");
            let fields = match ty {
                Some(TypeDescriptor::Struct(fields)) => Some(fields.as_slice()),
                _ => None,
            };
            for (field_id, field_value) in values {
                let field =
                    fields.and_then(|fields| fields.iter().find(|field| field.id == *field_id));
                let field_ty = field.map(|field| &field.ty);
                let name = field
                    .map(|field| field.name.as_str())
                    .unwrap_or("<unknown>");
                if value_needs_tree(field_value) {
                    line(
                        out,
                        indent + 1,
                        &format!("{name}#{field_id}: {}", value_inline(field_value)),
                    );
                    write_value_tree(field_value, field_ty, out, indent + 2);
                } else {
                    line(
                        out,
                        indent + 1,
                        &format!("{name}#{field_id}: {}", value_inline(field_value)),
                    );
                }
            }
        }
        TpackValue::List(values) => {
            line(out, indent, "list");
            let element_ty = match ty {
                Some(TypeDescriptor::List { element, .. }) => Some(element.as_ref()),
                _ => None,
            };
            for (index, item) in values.iter().enumerate() {
                if value_needs_tree(item) {
                    line(
                        out,
                        indent + 1,
                        &format!("[{index}]: {}", value_inline(item)),
                    );
                    write_value_tree(item, element_ty, out, indent + 2);
                } else {
                    line(
                        out,
                        indent + 1,
                        &format!("[{index}]: {}", value_inline(item)),
                    );
                }
            }
        }
        TpackValue::Map(entries) => {
            line(out, indent, "map");
            let (key_ty, value_ty) = match ty {
                Some(TypeDescriptor::Map { key, value, .. }) => {
                    (Some(key.as_ref()), Some(value.as_ref()))
                }
                _ => (None, None),
            };
            for ValueMapEntry { key, value } in entries {
                line(out, indent + 1, &format!("key: {}", value_inline(key)));
                if value_needs_tree(key) {
                    write_value_tree(key, key_ty, out, indent + 2);
                }
                line(out, indent + 1, &format!("value: {}", value_inline(value)));
                if value_needs_tree(value) {
                    write_value_tree(value, value_ty, out, indent + 2);
                }
            }
        }
        TpackValue::Union { index, value } => {
            let variant_ty = match ty {
                Some(TypeDescriptor::Union(variants)) => variants
                    .get(usize::try_from(*index).unwrap_or(usize::MAX))
                    .map(|variant| variant.ty.clone()),
                _ => None,
            };
            line(out, indent, &format!("union variant #{index}"));
            write_value_tree(value, variant_ty.as_ref(), out, indent + 1);
        }
        TpackValue::Optional(Some(value)) => {
            let inner_ty = match ty {
                Some(TypeDescriptor::Optional(inner)) => Some(inner.as_ref()),
                _ => None,
            };
            line(out, indent, "some");
            write_value_tree(value, inner_ty, out, indent + 1);
        }
        TpackValue::Optional(None) => line(out, indent, "none"),
        _ => line(out, indent, &value_inline(value)),
    }
}

fn write_json(message: &Message<'_>, section: InspectSection, out: &mut String) {
    match section {
        InspectSection::All => {
            line(out, 0, "{");
            write_json_property(out, 1, "envelope", |out, indent| {
                write_envelope_json(message, out, indent);
            });
            trim_last_newline(out);
            out.push_str(",\n");
            line(out, 1, "\"schema\": {");
            write_json_property(out, 2, "root", |out, indent| {
                write_type_json(&message.schema.root, out, indent);
            });
            trim_last_newline(out);
            out.push_str("\n  },\n");
            write_json_property(out, 1, "value", |out, indent| {
                write_value_json(&message.value, Some(&message.schema.root), out, indent);
            });
            trim_last_newline(out);
            out.push_str("\n}\n");
        }
        InspectSection::Envelope => write_envelope_json(message, out, 0),
        InspectSection::Schema => write_type_json(&message.schema.root, out, 0),
        InspectSection::Value => {
            write_value_json(&message.value, Some(&message.schema.root), out, 0)
        }
    }
}

fn write_envelope_json(message: &Message<'_>, out: &mut String, indent: usize) {
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

fn write_type_json(ty: &TypeDescriptor, out: &mut String, indent: usize) {
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
                    write_type_json(&field.ty, out, indent);
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
                write_type_json(element, out, indent);
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
                write_type_json(key, out, indent);
            });
            trim_last_newline(out);
            out.push_str(",\n");
            write_json_property(out, indent + 1, "value", |out, indent| {
                write_type_json(value, out, indent);
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
                    write_type_json(&variant.ty, out, indent);
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
                write_type_json(inner, out, indent);
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

fn write_value_json(
    value: &TpackValue<'_>,
    ty: Option<&TypeDescriptor>,
    out: &mut String,
    indent: usize,
) {
    match value {
        TpackValue::Struct(values) => {
            line(out, indent, "{");
            let fields = match ty {
                Some(TypeDescriptor::Struct(fields)) => Some(fields.as_slice()),
                _ => None,
            };
            for (index, (field_id, field_value)) in values.iter().enumerate() {
                let field =
                    fields.and_then(|fields| fields.iter().find(|field| field.id == *field_id));
                let field_ty = field.map(|field| &field.ty);
                let name = field
                    .map(|field| field.name.as_str())
                    .unwrap_or("<unknown>");
                let key = format!("{name}#{field_id}");
                write_json_property(out, indent + 1, &key, |out, indent| {
                    write_value_json(field_value, field_ty, out, indent);
                });
                trim_last_newline(out);
                out.push_str(comma(index + 1, values.len()));
                out.push('\n');
            }
            line(out, indent, "}");
        }
        TpackValue::List(values) => {
            line(out, indent, "[");
            let element_ty = match ty {
                Some(TypeDescriptor::List { element, .. }) => Some(element.as_ref()),
                _ => None,
            };
            for (index, item) in values.iter().enumerate() {
                write_value_json(item, element_ty, out, indent + 1);
                trim_last_newline(out);
                out.push_str(comma(index + 1, values.len()));
                out.push('\n');
            }
            line(out, indent, "]");
        }
        TpackValue::Map(entries) => {
            line(out, indent, "[");
            let (key_ty, value_ty) = match ty {
                Some(TypeDescriptor::Map { key, value, .. }) => {
                    (Some(key.as_ref()), Some(value.as_ref()))
                }
                _ => (None, None),
            };
            for (index, entry) in entries.iter().enumerate() {
                line(out, indent + 1, "{");
                write_json_property(out, indent + 2, "key", |out, indent| {
                    write_value_json(&entry.key, key_ty, out, indent);
                });
                trim_last_newline(out);
                out.push_str(",\n");
                write_json_property(out, indent + 2, "value", |out, indent| {
                    write_value_json(&entry.value, value_ty, out, indent);
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
            let variant_ty = match ty {
                Some(TypeDescriptor::Union(variants)) => variants
                    .get(usize::try_from(*index).unwrap_or(usize::MAX))
                    .map(|variant| variant.ty.clone()),
                _ => None,
            };
            line(out, indent, "{");
            line(out, indent + 1, &format!("\"variant\": {index},"));
            write_json_property(out, indent + 1, "value", |out, indent| {
                write_value_json(value, variant_ty.as_ref(), out, indent);
            });
            trim_last_newline(out);
            out.push('\n');
            line(out, indent, "}");
        }
        TpackValue::Optional(Some(value)) => {
            let inner_ty = match ty {
                Some(TypeDescriptor::Optional(inner)) => Some(inner.as_ref()),
                _ => None,
            };
            write_value_json(value, inner_ty, out, indent);
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

fn type_inline(ty: &TypeDescriptor) -> String {
    match ty {
        TypeDescriptor::Null => "null".to_string(),
        TypeDescriptor::Bool => "bool".to_string(),
        TypeDescriptor::I8 => "i8".to_string(),
        TypeDescriptor::I16 => "i16".to_string(),
        TypeDescriptor::I32 => "i32".to_string(),
        TypeDescriptor::I64 => "i64".to_string(),
        TypeDescriptor::U8 => "u8".to_string(),
        TypeDescriptor::U16 => "u16".to_string(),
        TypeDescriptor::U32 => "u32".to_string(),
        TypeDescriptor::U64 => "u64".to_string(),
        TypeDescriptor::F32 => "f32".to_string(),
        TypeDescriptor::F64 => "f64".to_string(),
        TypeDescriptor::Decimal => "decimal".to_string(),
        TypeDescriptor::DecimalFixed { precision, scale } => {
            format!("decimal_fixed(precision={precision}, scale={scale})")
        }
        TypeDescriptor::String { max_len } => {
            format!("string{}", optional_limit("max_len", *max_len))
        }
        TypeDescriptor::Bytes { max_len } => {
            format!("bytes{}", optional_limit("max_len", *max_len))
        }
        TypeDescriptor::Date => "date".to_string(),
        TypeDescriptor::Time => "time".to_string(),
        TypeDescriptor::DateTime => "datetime".to_string(),
        TypeDescriptor::DateTimeTz => "datetime_tz".to_string(),
        TypeDescriptor::Timestamp(precision) => {
            format!("timestamp({})", timestamp_precision_name(*precision))
        }
        TypeDescriptor::Duration => "duration".to_string(),
        TypeDescriptor::BigInt => "bigint".to_string(),
        TypeDescriptor::BigUInt => "biguint".to_string(),
        TypeDescriptor::CalendarInterval => "calendar_interval".to_string(),
        TypeDescriptor::Struct(_) => "struct".to_string(),
        TypeDescriptor::List { max_count, .. } => {
            format!("list{}", optional_limit("max_count", *max_count))
        }
        TypeDescriptor::Map { max_count, .. } => {
            format!("map{}", optional_limit("max_count", *max_count))
        }
        TypeDescriptor::Union(_) => "union".to_string(),
        TypeDescriptor::Enum(_) => "enum".to_string(),
        TypeDescriptor::Optional(inner) => format!("optional<{}>", type_inline(inner)),
        TypeDescriptor::Extension {
            authority,
            type_name,
            ..
        } => format!("extension({authority}/{type_name})"),
    }
}

fn value_inline(value: &TpackValue<'_>) -> String {
    match value {
        TpackValue::Null => "null".to_string(),
        TpackValue::Bool(value) => value.to_string(),
        TpackValue::I8(value) => value.to_string(),
        TpackValue::I16(value) => value.to_string(),
        TpackValue::I32(value) => value.to_string(),
        TpackValue::I64(value) => value.to_string(),
        TpackValue::U8(value) => value.to_string(),
        TpackValue::U16(value) => value.to_string(),
        TpackValue::U32(value) => value.to_string(),
        TpackValue::U64(value) => value.to_string(),
        TpackValue::F32(value) => value.to_string(),
        TpackValue::F64(value) => value.to_string(),
        TpackValue::Decimal(Decimal { scale, coefficient }) => {
            format!("decimal(scale={scale}, coefficient={coefficient})")
        }
        TpackValue::DecimalFixed(value) => value.to_string(),
        TpackValue::String(value) => format!("{value:?}"),
        TpackValue::Bytes(value) => bytes_hex(value),
        TpackValue::Date(value) => value.to_string(),
        TpackValue::Time(value) => value.to_string(),
        TpackValue::DateTime { days, nanos } => format!("datetime(days={days}, nanos={nanos})"),
        TpackValue::DateTimeTz {
            days,
            nanos,
            timezone,
        } => format!("datetime_tz(days={days}, nanos={nanos}, timezone={timezone:?})"),
        TpackValue::Timestamp(value) => value.to_string(),
        TpackValue::Duration(Duration { seconds, nanos }) => {
            format!("duration(seconds={seconds}, nanos={nanos})")
        }
        TpackValue::BigInt(value) => value.to_string(),
        TpackValue::BigUInt(value) => value.to_string(),
        TpackValue::CalendarInterval(CalendarInterval {
            months,
            days,
            nanos,
        }) => format!("calendar_interval(months={months}, days={days}, nanos={nanos})"),
        TpackValue::Struct(values) => format!("struct ({} fields)", values.len()),
        TpackValue::List(values) => format!("list ({} items)", values.len()),
        TpackValue::Map(entries) => format!("map ({} entries)", entries.len()),
        TpackValue::Union { index, .. } => format!("union variant #{index}"),
        TpackValue::Enum(index) => format!("enum #{index}"),
        TpackValue::Optional(Some(_)) => "some".to_string(),
        TpackValue::Optional(None) => "none".to_string(),
        TpackValue::Extension(value) => bytes_hex(value),
    }
}

fn type_needs_tree(ty: &TypeDescriptor) -> bool {
    matches!(
        ty,
        TypeDescriptor::Struct(_)
            | TypeDescriptor::List { .. }
            | TypeDescriptor::Map { .. }
            | TypeDescriptor::Union(_)
            | TypeDescriptor::Enum(_)
            | TypeDescriptor::Optional(_)
    )
}

fn value_needs_tree(value: &TpackValue<'_>) -> bool {
    matches!(
        value,
        TpackValue::Struct(_)
            | TpackValue::List(_)
            | TpackValue::Map(_)
            | TpackValue::Union { .. }
            | TpackValue::Optional(Some(_))
    )
}

fn optional_limit(name: &str, value: Option<u64>) -> String {
    match value {
        Some(value) => format!("({name}={value})"),
        None => String::new(),
    }
}

fn envelope_mode_name(mode: EnvelopeMode) -> &'static str {
    match mode {
        EnvelopeMode::FullSchema => "FullSchema",
        EnvelopeMode::FullSchemaWithId => "FullSchemaWithId",
        EnvelopeMode::SchemaRef => "SchemaRef",
    }
}

fn timestamp_precision_name(precision: TimestampPrecision) -> &'static str {
    match precision {
        TimestampPrecision::Seconds => "seconds",
        TimestampPrecision::Milliseconds => "milliseconds",
        TimestampPrecision::Microseconds => "microseconds",
        TimestampPrecision::Nanoseconds => "nanoseconds",
    }
}

fn line(out: &mut String, indent: usize, text: &str) {
    write_indent(out, indent);
    out.push_str(text);
    out.push('\n');
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_json_key(out: &mut String, indent: usize, key: &str) {
    write_indent(out, indent);
    let _ = write!(out, "\"{}\": ", json_escape(key));
}

fn write_json_property(
    out: &mut String,
    indent: usize,
    key: &str,
    write_value: impl FnOnce(&mut String, usize),
) {
    let mut value = String::new();
    write_value(&mut value, indent);
    trim_last_newline(&mut value);
    let prefix = indent_prefix(indent);
    if let Some(rest) = value.strip_prefix(&prefix) {
        write_json_key(out, indent, key);
        out.push_str(rest);
    } else {
        write_json_key(out, indent, key);
        out.push_str(&value);
    }
}

fn indent_prefix(indent: usize) -> String {
    let mut out = String::new();
    write_indent(&mut out, indent);
    out
}

fn bytes_hex(bytes: &[u8]) -> String {
    let mut out = String::from("0x");
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn comma(index: usize, len: usize) -> &'static str {
    if index == len { "" } else { "," }
}

fn trim_last_newline(out: &mut String) {
    if out.ends_with('\n') {
        out.pop();
    }
}

fn type_limit_json(
    out: &mut String,
    indent: usize,
    ty: &str,
    limit_name: &str,
    limit: Option<u64>,
) {
    line(out, indent, "{");
    line(out, indent + 1, &format!("\"type\": \"{ty}\","));
    write_optional_u64_json(out, indent + 1, limit_name, limit, false);
    line(out, indent, "}");
}

fn write_optional_u64_json(
    out: &mut String,
    indent: usize,
    name: &str,
    value: Option<u64>,
    trailing_comma: bool,
) {
    let comma = if trailing_comma { "," } else { "" };
    match value {
        Some(value) => line(out, indent, &format!("\"{name}\": {value}{comma}")),
        None => line(out, indent, &format!("\"{name}\": null{comma}")),
    }
}

fn decimal_json(out: &mut String, indent: usize, value: Decimal) {
    line(out, indent, "{");
    line(out, indent + 1, &format!("\"scale\": {},", value.scale));
    line(
        out,
        indent + 1,
        &format!("\"coefficient\": {}", value.coefficient),
    );
    line(out, indent, "}");
}

fn duration_json(out: &mut String, indent: usize, value: Duration) {
    line(out, indent, "{");
    line(out, indent + 1, &format!("\"seconds\": {},", value.seconds));
    line(out, indent + 1, &format!("\"nanos\": {}", value.nanos));
    line(out, indent, "}");
}

fn calendar_interval_json(out: &mut String, indent: usize, value: CalendarInterval) {
    line(out, indent, "{");
    line(out, indent + 1, &format!("\"months\": {},", value.months));
    line(out, indent + 1, &format!("\"days\": {},", value.days));
    line(out, indent + 1, &format!("\"nanos\": {}", value.nanos));
    line(out, indent, "}");
}

fn float_json(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        format!("\"{value:?}\"")
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, sync::Arc};

    use tpack::{Envelope, Field, Schema};

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
        write_tree(&sample_message(), InspectSection::All, &mut out);

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
        write_json(&sample_message(), InspectSection::Value, &mut out);

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
