use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};

use super::{CanonicalMode, EncodeOptions, NANOS_PER_DAY, wire};
use crate::{Error, ErrorKind, Result, Schema, TpackValue, TypeDescriptor};

use super::validate::{
    decimal_digits_abs, reject_nan_map_key, validate_byte_len, validate_count, validate_duration,
    validate_schema,
};

pub(in crate::codec) fn encode_schema_with_options(
    schema: &Schema,
    options: EncodeOptions,
) -> Result<Vec<u8>> {
    validate_schema(schema, &options.limits)?;
    let mut out = Vec::new();
    SchemaEncoder::new(&mut out).write_type_descriptor(&schema.root)?;
    if out.len() > options.limits.max_schema_len {
        return Err(Error::new(ErrorKind::SchemaLengthExceeded));
    }
    Ok(out)
}

pub(in crate::codec) fn encode_value_with_options(
    ty: &TypeDescriptor,
    value: &TpackValue<'_>,
    options: EncodeOptions,
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    ValueEncoder::new(&mut out, options).write_value(ty, value)?;
    Ok(out)
}

pub(in crate::codec) struct SchemaEncoder<'a> {
    out: &'a mut Vec<u8>,
}

impl<'a> SchemaEncoder<'a> {
    pub(in crate::codec) fn new(out: &'a mut Vec<u8>) -> Self {
        Self { out }
    }

    pub(in crate::codec) fn write_type_descriptor(&mut self, ty: &TypeDescriptor) -> Result<()> {
        match ty {
            TypeDescriptor::Null => self.out.push(0x00),
            TypeDescriptor::Bool => self.out.push(0x01),
            TypeDescriptor::I8 => self.out.push(0x02),
            TypeDescriptor::I16 => self.out.push(0x03),
            TypeDescriptor::I32 => self.out.push(0x04),
            TypeDescriptor::I64 => self.out.push(0x05),
            TypeDescriptor::U8 => self.out.push(0x06),
            TypeDescriptor::U16 => self.out.push(0x07),
            TypeDescriptor::U32 => self.out.push(0x08),
            TypeDescriptor::U64 => self.out.push(0x09),
            TypeDescriptor::F32 => self.out.push(0x0A),
            TypeDescriptor::F64 => self.out.push(0x0B),
            TypeDescriptor::Decimal => self.out.push(0x0C),
            TypeDescriptor::DecimalFixed { precision, scale } => {
                self.out.push(0x0D);
                wire::write_uvarint(self.out, *precision);
                wire::write_uvarint(self.out, *scale);
            }
            TypeDescriptor::String { max_len: Some(max) } => {
                self.out.push(0x0E);
                wire::write_uvarint(self.out, *max);
            }
            TypeDescriptor::String { max_len: None } => self.out.push(0x0F),
            TypeDescriptor::Bytes { max_len: Some(max) } => {
                self.out.push(0x10);
                wire::write_uvarint(self.out, *max);
            }
            TypeDescriptor::Bytes { max_len: None } => self.out.push(0x11),
            TypeDescriptor::Date => self.out.push(0x12),
            TypeDescriptor::Time => self.out.push(0x13),
            TypeDescriptor::DateTime => self.out.push(0x14),
            TypeDescriptor::DateTimeTz => self.out.push(0x15),
            TypeDescriptor::Timestamp(precision) => {
                self.out.push(0x16);
                self.out.push(precision.tag());
            }
            TypeDescriptor::Duration => self.out.push(0x17),
            TypeDescriptor::BigInt => self.out.push(0x18),
            TypeDescriptor::BigUInt => self.out.push(0x19),
            TypeDescriptor::CalendarInterval => self.out.push(0x1A),
            TypeDescriptor::Struct(fields) => {
                self.out.push(0x20);
                wire::write_uvarint(self.out, fields.len() as u64);
                for field in fields {
                    wire::write_uvarint(self.out, field.id);
                    wire::write_text(self.out, &field.name);
                    wire::write_uvarint(self.out, 0);
                    self.write_type_descriptor(&field.ty)?;
                }
            }
            TypeDescriptor::List { max_count, element } => {
                self.out.push(0x21);
                wire::write_uvarint(self.out, max_count.unwrap_or(0));
                self.write_type_descriptor(element)?;
            }
            TypeDescriptor::Map {
                max_count,
                key,
                value,
            } => {
                self.out.push(0x22);
                wire::write_uvarint(self.out, max_count.unwrap_or(0));
                self.write_type_descriptor(key)?;
                self.write_type_descriptor(value)?;
            }
            TypeDescriptor::Union(variants) => {
                self.out.push(0x23);
                wire::write_uvarint(self.out, variants.len() as u64);
                for variant in variants {
                    wire::write_text(self.out, &variant.name);
                    self.write_type_descriptor(&variant.ty)?;
                }
            }
            TypeDescriptor::Enum(symbols) => {
                self.out.push(0x24);
                wire::write_uvarint(self.out, symbols.len() as u64);
                for symbol in symbols {
                    wire::write_text(self.out, symbol);
                }
            }
            TypeDescriptor::Optional(inner) => {
                self.out.push(0x25);
                self.write_type_descriptor(inner)?;
            }
            TypeDescriptor::Extension {
                authority,
                type_name,
                schema_params,
            } => {
                self.out.push(0x26);
                wire::write_text(self.out, authority);
                wire::write_text(self.out, type_name);
                wire::write_bytes(self.out, schema_params);
            }
        }
        Ok(())
    }
}

pub(in crate::codec) struct ValueEncoder<'a> {
    out: &'a mut Vec<u8>,
    options: EncodeOptions,
}

impl<'a> ValueEncoder<'a> {
    pub(in crate::codec) fn new(out: &'a mut Vec<u8>, options: EncodeOptions) -> Self {
        Self { out, options }
    }

    pub(in crate::codec) fn write_value(
        &mut self,
        ty: &TypeDescriptor,
        value: &TpackValue<'_>,
    ) -> Result<()> {
        match (ty, value) {
            (TypeDescriptor::Null, TpackValue::Null) => {}
            (TypeDescriptor::Bool, TpackValue::Bool(value)) => self.out.push(u8::from(*value)),
            (TypeDescriptor::I8, TpackValue::I8(value)) => {
                self.out.extend_from_slice(&value.to_be_bytes())
            }
            (TypeDescriptor::I16, TpackValue::I16(value)) => {
                self.out.extend_from_slice(&value.to_be_bytes())
            }
            (TypeDescriptor::I32, TpackValue::I32(value)) => {
                self.out.extend_from_slice(&value.to_be_bytes())
            }
            (TypeDescriptor::I64, TpackValue::I64(value)) => {
                self.out.extend_from_slice(&value.to_be_bytes())
            }
            (TypeDescriptor::U8, TpackValue::U8(value)) => self.out.push(*value),
            (TypeDescriptor::U16, TpackValue::U16(value)) => {
                self.out.extend_from_slice(&value.to_be_bytes())
            }
            (TypeDescriptor::U32, TpackValue::U32(value)) => {
                self.out.extend_from_slice(&value.to_be_bytes())
            }
            (TypeDescriptor::U64, TpackValue::U64(value)) => {
                self.out.extend_from_slice(&value.to_be_bytes())
            }
            (TypeDescriptor::F32, TpackValue::F32(value)) => {
                let bits = if self.options.canonical.is_strict() && value.is_nan() {
                    0x7FC0_0000
                } else {
                    value.to_bits()
                };
                self.out.extend_from_slice(&bits.to_be_bytes());
            }
            (TypeDescriptor::F64, TpackValue::F64(value)) => {
                let bits = if self.options.canonical.is_strict() && value.is_nan() {
                    0x7FF8_0000_0000_0000
                } else {
                    value.to_bits()
                };
                self.out.extend_from_slice(&bits.to_be_bytes());
            }
            (TypeDescriptor::Decimal, TpackValue::Decimal(value)) => {
                wire::write_svarint(self.out, value.scale);
                wire::write_svarint(self.out, value.coefficient);
            }
            (TypeDescriptor::DecimalFixed { precision, .. }, TpackValue::DecimalFixed(value)) => {
                if decimal_digits_abs(*value) > *precision {
                    return Err(Error::invalid("Decimal(P,S) coefficient exceeds precision"));
                }
                wire::write_svarint(self.out, *value);
            }
            (TypeDescriptor::String { max_len }, TpackValue::String(value)) => {
                validate_byte_len("string length", value.len(), *max_len, &self.options.limits)?;
                wire::write_text(self.out, value);
            }
            (TypeDescriptor::Bytes { max_len }, TpackValue::Bytes(value)) => {
                validate_byte_len("bytes length", value.len(), *max_len, &self.options.limits)?;
                wire::write_bytes(self.out, value);
            }
            (TypeDescriptor::Date, TpackValue::Date(value)) => {
                wire::write_svarint(self.out, *value)
            }
            (TypeDescriptor::Time, TpackValue::Time(value)) => {
                if *value >= NANOS_PER_DAY {
                    return Err(Error::invalid("time value exceeds nanos-per-day"));
                }
                wire::write_uvarint(self.out, *value);
            }
            (TypeDescriptor::DateTime, TpackValue::DateTime { days, nanos }) => {
                if *nanos >= NANOS_PER_DAY {
                    return Err(Error::invalid("datetime time value exceeds nanos-per-day"));
                }
                wire::write_svarint(self.out, *days);
                wire::write_uvarint(self.out, *nanos);
            }
            (
                TypeDescriptor::DateTimeTz,
                TpackValue::DateTimeTz {
                    days,
                    nanos,
                    timezone,
                },
            ) => {
                if *nanos >= NANOS_PER_DAY {
                    return Err(Error::invalid(
                        "datetime-tz time value exceeds nanos-per-day",
                    ));
                }
                wire::write_svarint(self.out, *days);
                wire::write_uvarint(self.out, *nanos);
                wire::write_text(self.out, timezone);
            }
            (TypeDescriptor::Timestamp(_), TpackValue::Timestamp(value)) => {
                wire::write_svarint(self.out, *value)
            }
            (TypeDescriptor::Duration, TpackValue::Duration(value)) => {
                validate_duration(value.seconds, value.nanos)?;
                wire::write_svarint(self.out, value.seconds);
                wire::write_svarint(self.out, value.nanos);
            }
            (TypeDescriptor::BigInt, TpackValue::BigInt(value)) => {
                wire::write_svarint(self.out, *value)
            }
            (TypeDescriptor::BigUInt, TpackValue::BigUInt(value)) => {
                wire::write_uvarint(self.out, *value)
            }
            (TypeDescriptor::CalendarInterval, TpackValue::CalendarInterval(value)) => {
                wire::write_svarint(self.out, value.months);
                wire::write_svarint(self.out, value.days);
                wire::write_svarint(self.out, value.nanos);
            }
            (TypeDescriptor::Struct(fields), TpackValue::Struct(values)) => {
                self.write_struct(fields, values)?
            }
            (TypeDescriptor::List { max_count, element }, TpackValue::List(values)) => {
                validate_count("list count", values.len(), *max_count, &self.options.limits)?;
                wire::write_uvarint(self.out, values.len() as u64);
                for value in values {
                    self.write_value(element, value)?;
                }
            }
            (
                TypeDescriptor::Map {
                    max_count,
                    key,
                    value,
                },
                TpackValue::Map(entries),
            ) => self.write_map(*max_count, key, value, entries)?,
            (TypeDescriptor::Union(variants), TpackValue::Union { index, value, .. }) => {
                let variant = variants
                    .get(usize::try_from(*index).map_err(|_| Error::limit("variant index"))?)
                    .ok_or(Error::invalid("union variant index out of range"))?;
                wire::write_uvarint(self.out, *index);
                self.write_value(&variant.ty, value)?;
            }
            (TypeDescriptor::Enum(symbols), TpackValue::Enum(index)) => {
                if usize::try_from(*index)
                    .ok()
                    .and_then(|index| symbols.get(index))
                    .is_none()
                {
                    return Err(Error::invalid("enum symbol index out of range"));
                }
                wire::write_uvarint(self.out, *index);
            }
            (TypeDescriptor::Optional(_), TpackValue::Optional(None)) => self.out.push(0),
            (TypeDescriptor::Optional(inner), TpackValue::Optional(Some(value))) => {
                self.out.push(1);
                self.write_value(inner, value)?;
            }
            (TypeDescriptor::Extension { .. }, TpackValue::Extension(value)) => {
                if value.len() > self.options.limits.max_extension_len {
                    return Err(Error::limit("extension payload size"));
                }
                wire::write_bytes(self.out, value);
            }
            _ => {
                return Err(Error::new(ErrorKind::TypeMismatch {
                    expected: Self::type_name(ty),
                }));
            }
        }
        Ok(())
    }

    fn write_struct(
        &mut self,
        fields: &[crate::Field],
        values: &[(u64, TpackValue<'_>)],
    ) -> Result<()> {
        if values.len() == fields.len()
            && fields
                .iter()
                .zip(values.iter())
                .all(|(field, (id, _))| field.id == *id)
        {
            for (field, (_, field_value)) in fields.iter().zip(values) {
                self.write_value(&field.ty, field_value)?;
            }
            return Ok(());
        }

        let known_field_ids: BTreeSet<u64> = fields.iter().map(|field| field.id).collect();
        let mut provided_values = BTreeMap::new();
        for (id, field_value) in values {
            if !known_field_ids.contains(id) {
                continue;
            }
            if provided_values.insert(*id, field_value).is_some() {
                return Err(Error::invalid("duplicate struct field value"));
            }
        }

        for field in fields {
            let field_value = provided_values
                .get(&field.id)
                .copied()
                .ok_or(Error::invalid("missing struct field value"))?;
            self.write_value(&field.ty, field_value)?;
        }

        Ok(())
    }

    fn write_map(
        &mut self,
        max_count: Option<u64>,
        key_ty: &TypeDescriptor,
        value_ty: &TypeDescriptor,
        entries: &[crate::ValueMapEntry<'_>],
    ) -> Result<()> {
        validate_count("map count", entries.len(), max_count, &self.options.limits)?;
        let mut encoded_entries = Vec::with_capacity(entries.len());
        for entry in entries {
            reject_nan_map_key(&entry.key)?;
            let key_bytes = encode_value_with_options(
                key_ty,
                &entry.key,
                EncodeOptions {
                    canonical: CanonicalMode::Strict,
                    limits: self.options.limits,
                },
            )?;
            let mut value_bytes = Vec::new();
            ValueEncoder::new(&mut value_bytes, self.options)
                .write_value(value_ty, &entry.value)?;
            encoded_entries.push((key_bytes, value_bytes));
        }
        encoded_entries.sort_by(|a, b| a.0.cmp(&b.0));
        for pair in encoded_entries.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(Error::invalid("duplicate map key"));
            }
        }
        if !self.options.canonical.is_strict() {
            encoded_entries.clear();
            for entry in entries {
                let mut key_bytes = Vec::new();
                ValueEncoder::new(&mut key_bytes, self.options).write_value(key_ty, &entry.key)?;
                let mut value_bytes = Vec::new();
                ValueEncoder::new(&mut value_bytes, self.options)
                    .write_value(value_ty, &entry.value)?;
                encoded_entries.push((key_bytes, value_bytes));
            }
        }
        wire::write_uvarint(self.out, entries.len() as u64);
        for (key_bytes, value_bytes) in encoded_entries {
            self.out.extend_from_slice(&key_bytes);
            self.out.extend_from_slice(&value_bytes);
        }
        Ok(())
    }

    fn type_name(ty: &TypeDescriptor) -> &'static str {
        match ty {
            TypeDescriptor::Null => "Null",
            TypeDescriptor::Bool => "Bool",
            TypeDescriptor::I8 => "I8",
            TypeDescriptor::I16 => "I16",
            TypeDescriptor::I32 => "I32",
            TypeDescriptor::I64 => "I64",
            TypeDescriptor::U8 => "U8",
            TypeDescriptor::U16 => "U16",
            TypeDescriptor::U32 => "U32",
            TypeDescriptor::U64 => "U64",
            TypeDescriptor::F32 => "F32",
            TypeDescriptor::F64 => "F64",
            TypeDescriptor::Decimal => "Decimal",
            TypeDescriptor::DecimalFixed { .. } => "Decimal(P,S)",
            TypeDescriptor::String { .. } => "String",
            TypeDescriptor::Bytes { .. } => "Bytes",
            TypeDescriptor::Date => "Date",
            TypeDescriptor::Time => "Time",
            TypeDescriptor::DateTime => "DateTime",
            TypeDescriptor::DateTimeTz => "DateTimeTZ",
            TypeDescriptor::Timestamp(_) => "Timestamp(P)",
            TypeDescriptor::Duration => "Duration",
            TypeDescriptor::BigInt => "BigInt",
            TypeDescriptor::BigUInt => "BigUInt",
            TypeDescriptor::CalendarInterval => "CalendarInterval",
            TypeDescriptor::Struct(_) => "Struct",
            TypeDescriptor::List { .. } => "List",
            TypeDescriptor::Map { .. } => "Map",
            TypeDescriptor::Union(_) => "Union",
            TypeDescriptor::Enum(_) => "Enum",
            TypeDescriptor::Optional(_) => "Optional",
            TypeDescriptor::Extension { .. } => "Extension",
        }
    }
}
