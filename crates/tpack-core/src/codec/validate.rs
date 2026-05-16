use alloc::collections::BTreeSet;

use super::Limits;
use crate::{Error, Result, Schema, TpackValue, TypeDescriptor};

pub(in crate::codec) fn validate_schema(schema: &Schema, limits: &Limits) -> Result<()> {
    SchemaValidator::new(limits).validate_schema(schema)
}

pub(in crate::codec) fn is_valid_map_key_type(ty: &TypeDescriptor) -> bool {
    !matches!(
        ty,
        TypeDescriptor::Null
            | TypeDescriptor::Optional(_)
            | TypeDescriptor::List { .. }
            | TypeDescriptor::Map { .. }
            | TypeDescriptor::Struct(_)
            | TypeDescriptor::Union(_)
            | TypeDescriptor::Extension { .. }
    )
}

pub(in crate::codec) fn reject_nan_map_key(value: &TpackValue<'_>) -> Result<()> {
    match value {
        TpackValue::F32(value) if value.is_nan() => Err(Error::invalid("NaN map key")),
        TpackValue::F64(value) if value.is_nan() => Err(Error::invalid("NaN map key")),
        _ => Ok(()),
    }
}

pub(in crate::codec) fn validate_duration(seconds: i64, nanos: i64) -> Result<()> {
    if nanos <= -1_000_000_000 || nanos >= 1_000_000_000 {
        return Err(Error::invalid("duration nanos out of range"));
    }
    if seconds != 0
        && nanos != 0
        && ((seconds.is_positive() && nanos.is_negative())
            || (seconds.is_negative() && nanos.is_positive()))
    {
        return Err(Error::invalid("duration seconds and nanos signs differ"));
    }
    Ok(())
}

pub(in crate::codec) fn validate_count(
    name: &'static str,
    actual: usize,
    schema_max: Option<u64>,
    limits: &Limits,
) -> Result<()> {
    if let Some(max) = schema_max {
        if u64::try_from(actual).map_or(true, |actual| actual > max) {
            return Err(Error::limit(name));
        }
    }
    if actual > limits.max_collection_len {
        return Err(Error::limit(name));
    }
    Ok(())
}

pub(in crate::codec) fn validate_byte_len(
    name: &'static str,
    len: usize,
    schema_max: Option<u64>,
    limits: &Limits,
) -> Result<()> {
    if let Some(max) = schema_max {
        if u64::try_from(len).map_or(true, |len| len > max) {
            return Err(Error::limit(name));
        }
    }
    let limit = match name {
        "string length" => limits.max_string_len,
        _ => limits.max_bytes_len,
    };
    if len > limit {
        return Err(Error::limit(name));
    }
    Ok(())
}

pub(in crate::codec) fn decimal_digits_abs(value: i64) -> u64 {
    let mut value = if value < 0 {
        -(value as i128)
    } else {
        value as i128
    };
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

struct SchemaValidator<'a> {
    limits: &'a Limits,
}

impl<'a> SchemaValidator<'a> {
    fn new(limits: &'a Limits) -> Self {
        Self { limits }
    }

    fn validate_schema(&self, schema: &Schema) -> Result<()> {
        self.validate_type_descriptor(&schema.root, 0)
    }

    fn validate_type_descriptor(&self, ty: &TypeDescriptor, depth: usize) -> Result<()> {
        if depth > self.limits.max_depth {
            return Err(Error::limit("schema depth"));
        }
        match ty {
            TypeDescriptor::DecimalFixed { precision, scale }
                if *precision == 0 || scale > precision =>
            {
                return Err(Error::invalid("invalid Decimal(P,S) parameters"));
            }
            TypeDescriptor::Struct(fields) => {
                if fields.len() > self.limits.max_fields {
                    return Err(Error::limit("struct field count"));
                }
                let mut seen_ids = BTreeSet::new();
                let mut seen_names = BTreeSet::new();
                for field in fields {
                    if field.id == 0 {
                        return Err(Error::invalid("struct FieldId must be greater than zero"));
                    }
                    if field.name.is_empty() {
                        return Err(Error::invalid("struct field name must be non-empty"));
                    }
                    if !seen_ids.insert(field.id) || !seen_names.insert(field.name.as_str()) {
                        return Err(Error::invalid("duplicate struct field identifier or name"));
                    }
                    self.validate_type_descriptor(&field.ty, depth + 1)?;
                }
            }
            TypeDescriptor::List { element, .. } => {
                self.validate_type_descriptor(element, depth + 1)?
            }
            TypeDescriptor::Map { key, value, .. } => {
                if !is_valid_map_key_type(key) {
                    return Err(Error::invalid("invalid map key type"));
                }
                self.validate_type_descriptor(key, depth + 1)?;
                self.validate_type_descriptor(value, depth + 1)?;
            }
            TypeDescriptor::Union(variants) => {
                if variants.len() > self.limits.max_variants {
                    return Err(Error::limit("union variant count"));
                }
                let mut seen_names = BTreeSet::new();
                for variant in variants {
                    if variant.name.is_empty() {
                        return Err(Error::invalid("union variant name must be non-empty"));
                    }
                    if !seen_names.insert(variant.name.as_str()) {
                        return Err(Error::invalid("duplicate union variant name"));
                    }
                    self.validate_type_descriptor(&variant.ty, depth + 1)?;
                }
            }
            TypeDescriptor::Enum(symbols) => {
                if symbols.len() > self.limits.max_variants {
                    return Err(Error::limit("enum symbol count"));
                }
                let mut seen_symbols = BTreeSet::new();
                for symbol in symbols {
                    if symbol.is_empty() {
                        return Err(Error::invalid("enum symbol must be non-empty"));
                    }
                    if !seen_symbols.insert(symbol.as_str()) {
                        return Err(Error::invalid("duplicate enum symbol"));
                    }
                }
            }
            TypeDescriptor::Optional(inner) => self.validate_type_descriptor(inner, depth + 1)?,
            _ => {}
        }
        Ok(())
    }
}
