use std::fmt::Write;

use tpack::{
    CalendarInterval, Decimal, Duration, EnvelopeMode, Field, TimestampPrecision, TpackValue,
    TypeDescriptor,
};

pub(super) fn line(out: &mut String, indent: usize, text: &str) {
    write_indent(out, indent);
    out.push_str(text);
    out.push('\n');
}

pub(super) fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

pub(super) fn write_json_key(out: &mut String, indent: usize, key: &str) {
    write_indent(out, indent);
    let _ = write!(out, "\"{}\": ", json_escape(key));
}

pub(super) fn write_json_property(
    out: &mut String,
    indent: usize,
    key: &str,
    write_value: impl FnOnce(&mut String, usize),
) {
    let mut value = String::new();
    write_value(&mut value, indent);
    trim_last_newline(&mut value);
    let prefix = indent_prefix(indent);
    write_json_key(out, indent, key);
    if let Some(rest) = value.strip_prefix(&prefix) {
        out.push_str(rest);
    } else {
        out.push_str(&value);
    }
}

pub(super) fn trim_last_newline(out: &mut String) {
    if out.ends_with('\n') {
        out.pop();
    }
}

pub(super) fn comma(index: usize, len: usize) -> &'static str {
    if index == len { "" } else { "," }
}

pub(super) fn type_limit_json(
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

pub(super) fn write_optional_u64_json(
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

pub(super) fn decimal_json(out: &mut String, indent: usize, value: Decimal) {
    line(out, indent, "{");
    line(out, indent + 1, &format!("\"scale\": {},", value.scale));
    line(
        out,
        indent + 1,
        &format!("\"coefficient\": {}", value.coefficient),
    );
    line(out, indent, "}");
}
pub(super) fn duration_json(out: &mut String, indent: usize, value: Duration) {
    line(out, indent, "{");
    line(out, indent + 1, &format!("\"seconds\": {},", value.seconds));
    line(out, indent + 1, &format!("\"nanos\": {}", value.nanos));
    line(out, indent, "}");
}

pub(super) fn calendar_interval_json(out: &mut String, indent: usize, value: CalendarInterval) {
    line(out, indent, "{");
    line(out, indent + 1, &format!("\"months\": {},", value.months));
    line(out, indent + 1, &format!("\"days\": {},", value.days));
    line(out, indent + 1, &format!("\"nanos\": {}", value.nanos));
    line(out, indent, "}");
}

pub(super) fn float_json(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        format!("\"{value:?}\"")
    }
}

pub(super) fn optional_limit(name: &str, value: Option<u64>) -> String {
    match value {
        Some(value) => format!("({name}={value})"),
        None => String::new(),
    }
}

pub(super) fn envelope_mode_name(mode: EnvelopeMode) -> &'static str {
    match mode {
        EnvelopeMode::FullSchema => "FullSchema",
        EnvelopeMode::FullSchemaWithId => "FullSchemaWithId",
        EnvelopeMode::SchemaRef => "SchemaRef",
    }
}

pub(super) fn timestamp_precision_name(precision: TimestampPrecision) -> &'static str {
    match precision {
        TimestampPrecision::Seconds => "seconds",
        TimestampPrecision::Milliseconds => "milliseconds",
        TimestampPrecision::Microseconds => "microseconds",
        TimestampPrecision::Nanoseconds => "nanoseconds",
    }
}

pub(super) fn bytes_hex(bytes: &[u8]) -> String {
    let mut out = String::from("0x");
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

pub(super) fn json_escape(value: &str) -> String {
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

pub(super) fn type_inline(ty: &TypeDescriptor) -> String {
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

pub(super) fn value_inline(value: &TpackValue<'_>) -> String {
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
        TpackValue::DateTime { days, nanos } => {
            format!("datetime(days={days}, nanos={nanos})")
        }
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

pub(super) fn type_needs_tree(ty: &TypeDescriptor) -> bool {
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

pub(super) fn value_needs_tree(value: &TpackValue<'_>) -> bool {
    matches!(
        value,
        TpackValue::Struct(_)
            | TpackValue::List(_)
            | TpackValue::Map(_)
            | TpackValue::Union { .. }
            | TpackValue::Optional(Some(_))
    )
}

pub(super) fn find_struct_field(ty: Option<&TypeDescriptor>, field_id: u64) -> Option<&Field> {
    match ty {
        Some(TypeDescriptor::Struct(fields)) => fields.iter().find(|field| field.id == field_id),
        _ => None,
    }
}

pub(super) fn list_element_type(ty: Option<&TypeDescriptor>) -> Option<&TypeDescriptor> {
    match ty {
        Some(TypeDescriptor::List { element, .. }) => Some(element.as_ref()),
        _ => None,
    }
}

pub(super) fn map_types(
    ty: Option<&TypeDescriptor>,
) -> (Option<&TypeDescriptor>, Option<&TypeDescriptor>) {
    match ty {
        Some(TypeDescriptor::Map { key, value, .. }) => (Some(key.as_ref()), Some(value.as_ref())),
        _ => (None, None),
    }
}

pub(super) fn union_variant_type(
    ty: Option<&TypeDescriptor>,
    index: u64,
) -> Option<&TypeDescriptor> {
    match ty {
        Some(TypeDescriptor::Union(variants)) => usize::try_from(index)
            .ok()
            .and_then(|index| variants.get(index))
            .map(|variant| &variant.ty),
        _ => None,
    }
}

pub(super) fn optional_inner_type(ty: Option<&TypeDescriptor>) -> Option<&TypeDescriptor> {
    match ty {
        Some(TypeDescriptor::Optional(inner)) => Some(inner.as_ref()),
        _ => None,
    }
}

fn indent_prefix(indent: usize) -> String {
    let mut out = String::new();
    write_indent(&mut out, indent);
    out
}
