use alloc::{borrow::Cow, boxed::Box, collections::BTreeSet, string::String, sync::Arc, vec::Vec};
use core::cmp::Ordering;
use sha2::{Digest, Sha256};

mod encode;
mod validate;
mod wire;

use crate::{
    CalendarInterval, Decimal, Duration, Envelope, EnvelopeMode, Error, ErrorKind, Field, Message,
    Result, Schema, SchemaId, SchemaRegistry, TimestampPrecision, TpackValue, TypeDescriptor,
    ValueMapEntry, Variant, empty_registry,
};

pub const MAGIC: [u8; 4] = *b"TPAK";
pub const VERSION: u8 = 0x01;

const NANOS_PER_DAY: u64 = 86_400_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalMode {
    Off,
    Strict,
}

impl CanonicalMode {
    pub fn is_strict(self) -> bool {
        matches!(self, CanonicalMode::Strict)
    }
}

/// Resource limits applied during schema validation and message encode/decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum encoded schema size in bytes.
    ///
    /// This limit is enforced symmetrically on decode and encode paths.
    pub max_schema_len: usize,
    pub max_schema_id_len: usize,
    pub max_depth: usize,
    pub max_fields: usize,
    pub max_variants: usize,
    pub max_collection_len: usize,
    pub max_string_len: usize,
    pub max_bytes_len: usize,
    pub max_extension_len: usize,
    pub max_varint_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_schema_len: 1024 * 1024,
            max_schema_id_len: 1024,
            max_depth: 128,
            max_fields: 16_384,
            max_variants: 16_384,
            max_collection_len: 1_000_000,
            max_string_len: 16 * 1024 * 1024,
            max_bytes_len: 16 * 1024 * 1024,
            max_extension_len: 16 * 1024 * 1024,
            max_varint_bytes: 10,
        }
    }
}

/// Decoder behavior switches and resource limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeOptions {
    pub canonical: CanonicalMode,
    pub allow_schema_ref: bool,
    /// Validate embedded schema bytes on `FullSchemaWithId` registry hits.
    ///
    /// When enabled, the decoder reparses the embedded schema block and
    /// requires it to match the cached schema before reusing the cached AST.
    /// Disable this only when the registry entry is already trusted and the
    /// embedded schema bytes do not need to be checked.
    pub validate_embedded_schema_on_cache_hit: bool,
    pub limits: Limits,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            canonical: CanonicalMode::Off,
            allow_schema_ref: true,
            validate_embedded_schema_on_cache_hit: true,
            limits: Limits::default(),
        }
    }
}

/// Encoder behavior switches and resource limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeOptions {
    pub canonical: CanonicalMode,
    pub limits: Limits,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            canonical: CanonicalMode::Off,
            limits: Limits::default(),
        }
    }
}

pub struct Decoder<'de> {
    input: &'de [u8],
    pos: usize,
    options: DecodeOptions,
}

impl<'de> Decoder<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        Self::with_options(input, DecodeOptions::default())
    }

    pub fn with_options(input: &'de [u8], options: DecodeOptions) -> Self {
        Self {
            input,
            pos: 0,
            options,
        }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn is_eof(&self) -> bool {
        self.pos == self.input.len()
    }

    pub fn decode_message(&mut self) -> Result<Message<'de>> {
        self.decode_message_with_registry(&empty_registry())
    }

    pub fn decode_message_with_registry<R: SchemaRegistry + ?Sized>(
        &mut self,
        registry: &R,
    ) -> Result<Message<'de>> {
        self.read_header()?;
        let mode = match self.read_u8()? {
            0x00 => EnvelopeMode::FullSchema,
            0x01 => EnvelopeMode::FullSchemaWithId,
            0x02 => EnvelopeMode::SchemaRef,
            other => return Err(Error::new(ErrorKind::UnknownEnvelopeMode(other))),
        };

        let (schema_id, schema, used_cached_schema) = match mode {
            EnvelopeMode::FullSchema => {
                let schema = self.decode_schema_block()?;
                (None, Arc::new(schema), false)
            }
            EnvelopeMode::FullSchemaWithId => {
                let schema_id = self.read_schema_id(false)?;
                let schema_len = self.read_len("schema length")?;
                if schema_len > self.options.limits.max_schema_len {
                    return Err(Error::new(ErrorKind::SchemaLengthExceeded));
                }

                let schema_start = self.pos;
                let schema_end = schema_start
                    .checked_add(schema_len)
                    .ok_or(Error::new(ErrorKind::SchemaLengthExceeded))?;
                if schema_end > self.input.len() {
                    return Err(Error::new(ErrorKind::UnexpectedEof));
                }

                if let Some(schema) = registry.get(schema_id.as_bytes()) {
                    if self.options.validate_embedded_schema_on_cache_hit {
                        self.validate_cached_schema_bytes(schema_len, schema.as_ref())?;
                    } else {
                        // Cache hits can skip the embedded schema by byte length and
                        // reuse the shared AST when callers explicitly trust the
                        // registry entry for this schema id.
                        self.pos = schema_end;
                    }
                    (Some(schema_id), schema, true)
                } else {
                    let schema = self.decode_schema_at_exact_len(schema_len)?;
                    (Some(schema_id), Arc::new(schema), false)
                }
            }
            EnvelopeMode::SchemaRef => {
                if !self.options.allow_schema_ref {
                    return Err(Error::new(ErrorKind::SchemaRefNotAllowed));
                }
                let schema_id = self.read_schema_id(true)?;
                let schema = registry
                    .get(schema_id.as_bytes())
                    .ok_or(Error::new(ErrorKind::UnknownSchemaId))?;
                (Some(schema_id), schema, true)
            }
        };

        let value = self.decode_value_for(&schema.root, 0)?;
        if !self.is_eof() {
            return Err(Error::new(ErrorKind::TrailingBytes));
        }

        Ok(Message {
            envelope: Envelope {
                mode,
                schema_id,
                used_cached_schema,
            },
            schema,
            value,
        })
    }

    pub fn decode_schema(&mut self) -> Result<Schema> {
        let schema = Schema::new(self.decode_type_descriptor(0)?);
        validate::validate_schema(&schema, &self.options.limits)?;
        Ok(schema)
    }

    pub fn decode_value(&mut self, schema: &Schema) -> Result<TpackValue<'de>> {
        let value = self.decode_value_for(&schema.root, 0)?;
        if !self.is_eof() {
            return Err(Error::new(ErrorKind::TrailingBytes));
        }
        Ok(value)
    }

    fn read_header(&mut self) -> Result<()> {
        if self.read_bytes(4)? != MAGIC {
            return Err(Error::new(ErrorKind::InvalidMagic));
        }
        let version = self.read_u8()?;
        if version != VERSION {
            return Err(Error::new(ErrorKind::UnsupportedVersion(version)));
        }
        Ok(())
    }

    fn decode_schema_block(&mut self) -> Result<Schema> {
        let schema_len = self.read_len("schema length")?;
        if schema_len > self.options.limits.max_schema_len {
            return Err(Error::new(ErrorKind::SchemaLengthExceeded));
        }
        self.decode_schema_at_exact_len(schema_len)
    }

    fn decode_schema_at_exact_len(&mut self, schema_len: usize) -> Result<Schema> {
        let start = self.pos;
        let schema = self.decode_schema()?;
        let consumed = self.pos - start;
        if consumed != schema_len {
            return Err(Error::new(ErrorKind::SchemaLengthMismatch));
        }
        Ok(schema)
    }

    fn validate_cached_schema_bytes(
        &mut self,
        schema_len: usize,
        cached_schema: &Schema,
    ) -> Result<()> {
        let embedded_schema = self.decode_schema_at_exact_len(schema_len)?;
        if &embedded_schema != cached_schema {
            return Err(Error::new(ErrorKind::EmbeddedSchemaMismatch));
        }
        Ok(())
    }

    fn read_schema_id(&mut self, require_non_empty: bool) -> Result<SchemaId<'de>> {
        let len = self.read_len("schema id length")?;
        if len > self.options.limits.max_schema_id_len {
            return Err(Error::new(ErrorKind::InvalidSchemaId));
        }
        if require_non_empty && len == 0 {
            return Err(Error::new(ErrorKind::InvalidSchemaId));
        }
        Ok(SchemaId::borrowed(self.read_bytes(len)?))
    }

    fn decode_type_descriptor(&mut self, depth: usize) -> Result<TypeDescriptor> {
        if depth > self.options.limits.max_depth {
            return Err(Error::limit("schema depth"));
        }
        let tag = self.read_u8()?;
        let ty = match tag {
            0x00 => TypeDescriptor::Null,
            0x01 => TypeDescriptor::Bool,
            0x02 => TypeDescriptor::I8,
            0x03 => TypeDescriptor::I16,
            0x04 => TypeDescriptor::I32,
            0x05 => TypeDescriptor::I64,
            0x06 => TypeDescriptor::U8,
            0x07 => TypeDescriptor::U16,
            0x08 => TypeDescriptor::U32,
            0x09 => TypeDescriptor::U64,
            0x0A => TypeDescriptor::F32,
            0x0B => TypeDescriptor::F64,
            0x0C => TypeDescriptor::Decimal,
            0x0D => {
                let precision = self.read_uvarint()?;
                let scale = self.read_uvarint()?;
                if precision == 0 || scale > precision {
                    return Err(Error::new(ErrorKind::InvalidDecimalParameters));
                }
                TypeDescriptor::DecimalFixed { precision, scale }
            }
            0x0E => TypeDescriptor::String {
                max_len: Some(self.read_uvarint()?),
            },
            0x0F => TypeDescriptor::String { max_len: None },
            0x10 => TypeDescriptor::Bytes {
                max_len: Some(self.read_uvarint()?),
            },
            0x11 => TypeDescriptor::Bytes { max_len: None },
            0x12 => TypeDescriptor::Date,
            0x13 => TypeDescriptor::Time,
            0x14 => TypeDescriptor::DateTime,
            0x15 => TypeDescriptor::DateTimeTz,
            0x16 => {
                let precision = match self.read_u8()? {
                    0 => TimestampPrecision::Seconds,
                    1 => TimestampPrecision::Milliseconds,
                    2 => TimestampPrecision::Microseconds,
                    3 => TimestampPrecision::Nanoseconds,
                    other => return Err(Error::new(ErrorKind::InvalidTimestampPrecision(other))),
                };
                TypeDescriptor::Timestamp(precision)
            }
            0x17 => TypeDescriptor::Duration,
            0x18 => TypeDescriptor::BigInt,
            0x19 => TypeDescriptor::BigUInt,
            0x1A => TypeDescriptor::CalendarInterval,
            0x20 => {
                let count = self.read_count("struct field count")?;
                if count > self.options.limits.max_fields {
                    return Err(Error::limit("struct field count"));
                }
                let mut fields = Vec::with_capacity(count);
                let mut seen_ids = BTreeSet::new();
                let mut seen_names = BTreeSet::new();
                for _ in 0..count {
                    let id = self.read_uvarint()?;
                    if id == 0 {
                        return Err(Error::new(ErrorKind::StructFieldIdZero));
                    }
                    let name = self.read_text_owned()?;
                    if name.is_empty() {
                        return Err(Error::new(ErrorKind::StructFieldNameEmpty));
                    }
                    let flags = self.read_uvarint()?;
                    if flags != 0 {
                        return Err(Error::new(ErrorKind::StructFieldFlagsNonZero(flags)));
                    }
                    let ty = self.decode_type_descriptor(depth + 1)?;
                    if !seen_ids.insert(id) || !seen_names.insert(name.clone()) {
                        return Err(Error::new(ErrorKind::DuplicateStructFieldDefinition));
                    }
                    fields.push(Field { id, name, ty });
                }
                TypeDescriptor::Struct(fields)
            }
            0x21 => {
                let max_count = wire::max_count_from_wire(self.read_uvarint()?);
                let element = Box::new(self.decode_type_descriptor(depth + 1)?);
                TypeDescriptor::List { max_count, element }
            }
            0x22 => {
                let max_count = wire::max_count_from_wire(self.read_uvarint()?);
                let key = Box::new(self.decode_type_descriptor(depth + 1)?);
                if !validate::is_valid_map_key_type(&key) {
                    return Err(Error::new(ErrorKind::InvalidMapKeyType));
                }
                let value = Box::new(self.decode_type_descriptor(depth + 1)?);
                TypeDescriptor::Map {
                    max_count,
                    key,
                    value,
                }
            }
            0x23 => {
                let count = self.read_count("union variant count")?;
                if count > self.options.limits.max_variants {
                    return Err(Error::limit("union variant count"));
                }
                let mut variants = Vec::with_capacity(count);
                let mut seen_names = BTreeSet::new();
                for _ in 0..count {
                    let name = self.read_text_owned()?;
                    if name.is_empty() {
                        return Err(Error::new(ErrorKind::UnionVariantNameEmpty));
                    }
                    if !seen_names.insert(name.clone()) {
                        return Err(Error::new(ErrorKind::DuplicateUnionVariantName));
                    }
                    let ty = self.decode_type_descriptor(depth + 1)?;
                    variants.push(Variant { name, ty });
                }
                TypeDescriptor::Union(variants)
            }
            0x24 => {
                let count = self.read_count("enum symbol count")?;
                if count > self.options.limits.max_variants {
                    return Err(Error::limit("enum symbol count"));
                }
                let mut symbols = Vec::with_capacity(count);
                let mut seen_symbols = BTreeSet::new();
                for _ in 0..count {
                    let symbol = self.read_text_owned()?;
                    if symbol.is_empty() {
                        return Err(Error::new(ErrorKind::EnumSymbolEmpty));
                    }
                    if !seen_symbols.insert(symbol.clone()) {
                        return Err(Error::new(ErrorKind::DuplicateEnumSymbol));
                    }
                    symbols.push(symbol);
                }
                TypeDescriptor::Enum(symbols)
            }
            0x25 => {
                let inner = Box::new(self.decode_type_descriptor(depth + 1)?);
                TypeDescriptor::Optional(inner)
            }
            0x26 => {
                let authority = self.read_text_owned()?;
                let type_label = self.read_text_owned()?;
                let schema_params = self.read_bytes_owned(self.options.limits.max_extension_len)?;
                TypeDescriptor::Extension {
                    authority,
                    type_name: type_label,
                    schema_params,
                }
            }
            other => return Err(Error::new(ErrorKind::UnknownTypeTag(other))),
        };
        Ok(ty)
    }

    fn decode_value_for(&mut self, ty: &TypeDescriptor, depth: usize) -> Result<TpackValue<'de>> {
        if depth > self.options.limits.max_depth {
            return Err(Error::limit("value depth"));
        }
        let value = match ty {
            TypeDescriptor::Null => TpackValue::Null,
            TypeDescriptor::Bool => match self.read_u8()? {
                0 => TpackValue::Bool(false),
                1 => TpackValue::Bool(true),
                _ => return Err(Error::invalid("invalid bool value")),
            },
            TypeDescriptor::I8 => TpackValue::I8(self.read_i8()?),
            TypeDescriptor::I16 => TpackValue::I16(i16::from_be_bytes(self.read_array()?)),
            TypeDescriptor::I32 => TpackValue::I32(i32::from_be_bytes(self.read_array()?)),
            TypeDescriptor::I64 => TpackValue::I64(i64::from_be_bytes(self.read_array()?)),
            TypeDescriptor::U8 => TpackValue::U8(self.read_u8()?),
            TypeDescriptor::U16 => TpackValue::U16(u16::from_be_bytes(self.read_array()?)),
            TypeDescriptor::U32 => TpackValue::U32(u32::from_be_bytes(self.read_array()?)),
            TypeDescriptor::U64 => TpackValue::U64(u64::from_be_bytes(self.read_array()?)),
            TypeDescriptor::F32 => {
                let bits = u32::from_be_bytes(self.read_array()?);
                if self.options.canonical.is_strict()
                    && f32::from_bits(bits).is_nan()
                    && bits != 0x7FC0_0000
                {
                    return Err(Error::invalid("non-canonical f32 NaN"));
                }
                TpackValue::F32(f32::from_bits(bits))
            }
            TypeDescriptor::F64 => {
                let bits = u64::from_be_bytes(self.read_array()?);
                if self.options.canonical.is_strict()
                    && f64::from_bits(bits).is_nan()
                    && bits != 0x7FF8_0000_0000_0000
                {
                    return Err(Error::invalid("non-canonical f64 NaN"));
                }
                TpackValue::F64(f64::from_bits(bits))
            }
            TypeDescriptor::Decimal => {
                let scale = self.read_svarint()?;
                let coefficient = self.read_svarint()?;
                TpackValue::Decimal(Decimal { scale, coefficient })
            }
            TypeDescriptor::DecimalFixed { precision, .. } => {
                let coefficient = self.read_svarint()?;
                if validate::decimal_digits_abs(coefficient) > *precision {
                    return Err(Error::invalid("Decimal(P,S) coefficient exceeds precision"));
                }
                TpackValue::DecimalFixed(coefficient)
            }
            TypeDescriptor::String { max_len } => {
                let value = self.read_text_borrowed(*max_len)?;
                TpackValue::String(Cow::Borrowed(value))
            }
            TypeDescriptor::Bytes { max_len } => {
                let value = self.read_byte_component(*max_len)?;
                TpackValue::Bytes(Cow::Borrowed(value))
            }
            TypeDescriptor::Date => TpackValue::Date(self.read_svarint()?),
            TypeDescriptor::Time => {
                let nanos = self.read_uvarint()?;
                if nanos >= NANOS_PER_DAY {
                    return Err(Error::invalid("time value exceeds nanos-per-day"));
                }
                TpackValue::Time(nanos)
            }
            TypeDescriptor::DateTime => {
                let days = self.read_svarint()?;
                let nanos = self.read_uvarint()?;
                if nanos >= NANOS_PER_DAY {
                    return Err(Error::invalid("datetime time value exceeds nanos-per-day"));
                }
                TpackValue::DateTime { days, nanos }
            }
            TypeDescriptor::DateTimeTz => {
                let days = self.read_svarint()?;
                let nanos = self.read_uvarint()?;
                if nanos >= NANOS_PER_DAY {
                    return Err(Error::invalid(
                        "datetime-tz time value exceeds nanos-per-day",
                    ));
                }
                let timezone = self.read_text_borrowed(None)?;
                TpackValue::DateTimeTz {
                    days,
                    nanos,
                    timezone: Cow::Borrowed(timezone),
                }
            }
            TypeDescriptor::Timestamp(_) => TpackValue::Timestamp(self.read_svarint()?),
            TypeDescriptor::Duration => {
                let seconds = self.read_svarint()?;
                let nanos = self.read_svarint()?;
                validate::validate_duration(seconds, nanos)?;
                TpackValue::Duration(Duration { seconds, nanos })
            }
            TypeDescriptor::BigInt => TpackValue::BigInt(self.read_svarint()?),
            TypeDescriptor::BigUInt => TpackValue::BigUInt(self.read_uvarint()?),
            TypeDescriptor::CalendarInterval => {
                let months = self.read_svarint()?;
                let days = self.read_svarint()?;
                let nanos = self.read_svarint()?;
                TpackValue::CalendarInterval(CalendarInterval {
                    months,
                    days,
                    nanos,
                })
            }
            TypeDescriptor::Struct(fields) => {
                let mut values = Vec::with_capacity(fields.len());
                for field in fields {
                    let value = self
                        .decode_value_for(&field.ty, depth + 1)
                        .map_err(|err| err.at_field(field.name.clone()))?;
                    values.push((field.id, value));
                }
                TpackValue::Struct(values)
            }
            TypeDescriptor::List { max_count, element } => {
                let count = self.read_count("list count")?;
                validate::validate_count("list count", count, *max_count, &self.options.limits)?;
                let mut values = Vec::with_capacity(count);
                for index in 0..count {
                    let value = self
                        .decode_value_for(element, depth + 1)
                        .map_err(|err| err.at_index(index))?;
                    values.push(value);
                }
                TpackValue::List(values)
            }
            TypeDescriptor::Map {
                max_count,
                key,
                value,
            } => {
                let count = self.read_count("map count")?;
                validate::validate_count("map count", count, *max_count, &self.options.limits)?;
                let mut entries = Vec::with_capacity(count);
                let mut seen_key_bytes = if self.options.canonical.is_strict() {
                    None
                } else {
                    Some(BTreeSet::new())
                };
                let mut last_key_bytes: Option<&'de [u8]> = None;
                for _ in 0..count {
                    let key_start = self.pos;
                    let key_value = self.decode_value_for(key, depth + 1)?;
                    let raw_key_bytes = &self.input[key_start..self.pos];
                    validate::reject_nan_map_key(&key_value)?;
                    if self.options.canonical.is_strict() {
                        // Strict canonical input means the bytes just consumed are
                        // already the canonical key representation. Compare slices
                        // directly instead of re-encoding every key into a Vec.
                        if let Some(previous) = last_key_bytes {
                            match previous.cmp(raw_key_bytes) {
                                Ordering::Less => {}
                                Ordering::Equal => {
                                    return Err(Error::invalid("duplicate map key"));
                                }
                                Ordering::Greater => {
                                    return Err(Error::invalid("non-canonical map key order"));
                                }
                            }
                        }
                        last_key_bytes = Some(raw_key_bytes);
                    }
                    if !self.options.canonical.is_strict() {
                        let canonical_key = encode::value(
                            key,
                            &key_value,
                            EncodeOptions {
                                canonical: CanonicalMode::Strict,
                                limits: self.options.limits,
                            },
                        )?;
                        if !seen_key_bytes
                            .as_mut()
                            .expect("non-strict mode allocates a map-key set")
                            .insert(canonical_key)
                        {
                            return Err(Error::invalid("duplicate map key"));
                        }
                    }
                    let value = self.decode_value_for(value, depth + 1)?;
                    entries.push(ValueMapEntry {
                        key: key_value,
                        value,
                    });
                }
                TpackValue::Map(entries)
            }
            TypeDescriptor::Union(variants) => {
                let index = self.read_uvarint()?;
                let variant = variants
                    .get(usize::try_from(index).map_err(|_| Error::limit("variant index"))?)
                    .ok_or(Error::invalid("union variant index out of range"))?;
                let value = self.decode_value_for(&variant.ty, depth + 1)?;
                TpackValue::Union {
                    index,
                    value: Box::new(value),
                }
            }
            TypeDescriptor::Enum(symbols) => {
                let index = self.read_uvarint()?;
                symbols
                    .get(usize::try_from(index).map_err(|_| Error::limit("enum index"))?)
                    .ok_or(Error::invalid("enum symbol index out of range"))?;
                TpackValue::Enum(index)
            }
            TypeDescriptor::Optional(inner) => match self.read_u8()? {
                0 => TpackValue::Optional(None),
                1 => TpackValue::Optional(Some(Box::new(self.decode_value_for(inner, depth + 1)?))),
                _ => return Err(Error::invalid("invalid optional presence marker")),
            },
            TypeDescriptor::Extension { .. } => {
                let bytes = self.read_extension_component()?;
                TpackValue::Extension(Cow::Borrowed(bytes))
            }
        };
        Ok(value)
    }

    fn read_u8(&mut self) -> Result<u8> {
        let byte = *self
            .input
            .get(self.pos)
            .ok_or(Error::new(ErrorKind::UnexpectedEof))?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_i8(&mut self) -> Result<i8> {
        Ok(i8::from_be_bytes([self.read_u8()?]))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.read_bytes(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'de [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or(Error::new(ErrorKind::UnexpectedEof))?;
        let bytes = self
            .input
            .get(self.pos..end)
            .ok_or(Error::new(ErrorKind::UnexpectedEof))?;
        self.pos = end;
        Ok(bytes)
    }

    fn read_uvarint(&mut self) -> Result<u64> {
        // The common case is a one-byte length/id/count. Keep it on a tiny
        // predictable path and push overflow/canonical checks to the cold loop.
        if let Some(&byte) = self.input.get(self.pos) {
            if byte < 0x80 {
                self.pos += 1;
                return Ok(u64::from(byte));
            }
        }
        self.read_uvarint_slow()
    }

    #[cold]
    fn read_uvarint_slow(&mut self) -> Result<u64> {
        let start = self.pos;
        let mut value = 0u64;
        for i in 0..self.options.limits.max_varint_bytes {
            let byte = self.read_u8()?;
            let payload = u64::from(byte & 0x7F);
            if i == 9 && payload > 1 {
                return Err(Error::new(ErrorKind::VarintOverflow));
            }
            value |= payload << (7 * i);
            if byte & 0x80 == 0 {
                let encoded_len = self.pos - start;
                if self.options.canonical.is_strict() && encoded_len != wire::uvarint_len(value) {
                    return Err(Error::new(ErrorKind::OverlongVarint));
                }
                return Ok(value);
            }
        }
        Err(Error::new(ErrorKind::VarintOverflow))
    }

    fn read_svarint(&mut self) -> Result<i64> {
        let raw = self.read_uvarint()?;
        Ok(((raw >> 1) as i64) ^ (-((raw & 1) as i64)))
    }

    fn read_len(&mut self, name: &'static str) -> Result<usize> {
        usize::try_from(self.read_uvarint()?).map_err(|_| Error::limit(name))
    }

    fn read_count(&mut self, name: &'static str) -> Result<usize> {
        usize::try_from(self.read_uvarint()?).map_err(|_| Error::limit(name))
    }

    fn read_text_owned(&mut self) -> Result<String> {
        Ok(String::from(self.read_text_borrowed(None)?))
    }

    fn read_text_borrowed(&mut self, schema_max: Option<u64>) -> Result<&'de str> {
        let bytes = self.read_limited_component(
            "string length",
            schema_max,
            self.options.limits.max_string_len,
        )?;
        Ok(core::str::from_utf8(bytes)?)
    }

    fn read_bytes_owned(&mut self, limit: usize) -> Result<Vec<u8>> {
        Ok(self
            .read_limited_component("byte string length", None, limit)?
            .to_vec())
    }

    fn read_byte_component(&mut self, schema_max: Option<u64>) -> Result<&'de [u8]> {
        self.read_limited_component(
            "byte string length",
            schema_max,
            self.options.limits.max_bytes_len,
        )
    }

    fn read_extension_component(&mut self) -> Result<&'de [u8]> {
        self.read_limited_component(
            "extension payload size",
            None,
            self.options.limits.max_extension_len,
        )
    }

    fn read_limited_component(
        &mut self,
        limit_name: &'static str,
        schema_max: Option<u64>,
        max_len: usize,
    ) -> Result<&'de [u8]> {
        let len = self.read_len(limit_name)?;
        let limit = schema_max
            .and_then(|max| usize::try_from(max).ok())
            .unwrap_or(max_len)
            .min(max_len);
        if len > limit {
            return Err(Error::limit(limit_name));
        }
        self.read_bytes(len)
    }
}

pub struct Encoder {
    out: Vec<u8>,
    options: EncodeOptions,
}

impl Encoder {
    pub fn new() -> Self {
        Self::with_options(EncodeOptions::default())
    }

    pub fn with_options(options: EncodeOptions) -> Self {
        Self {
            out: Vec::new(),
            options,
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.out
    }

    pub fn encode_message(
        &mut self,
        schema: &Schema,
        value: &TpackValue<'_>,
        mode: EnvelopeMode,
        schema_id: Option<&[u8]>,
    ) -> Result<()> {
        let schema_bytes = encode::schema(schema, self.options)?;
        self.out.extend_from_slice(&MAGIC);
        self.out.push(VERSION);
        self.out.push(mode.tag());
        match mode {
            EnvelopeMode::FullSchema => {
                wire::write_uvarint(&mut self.out, schema_bytes.len() as u64);
                self.out.extend_from_slice(&schema_bytes);
            }
            EnvelopeMode::FullSchemaWithId => {
                let schema_id = schema_id.unwrap_or(&[]);
                if schema_id.len() > self.options.limits.max_schema_id_len {
                    return Err(Error::new(ErrorKind::InvalidSchemaId));
                }
                wire::write_uvarint(&mut self.out, schema_id.len() as u64);
                self.out.extend_from_slice(schema_id);
                wire::write_uvarint(&mut self.out, schema_bytes.len() as u64);
                self.out.extend_from_slice(&schema_bytes);
            }
            EnvelopeMode::SchemaRef => {
                let schema_id = schema_id.ok_or(Error::new(ErrorKind::InvalidSchemaId))?;
                if schema_id.is_empty() || schema_id.len() > self.options.limits.max_schema_id_len {
                    return Err(Error::new(ErrorKind::InvalidSchemaId));
                }
                wire::write_uvarint(&mut self.out, schema_id.len() as u64);
                self.out.extend_from_slice(schema_id);
            }
        }
        encode::ValueEncoder::new(&mut self.out, self.options).write_value(&schema.root, value)?;
        Ok(())
    }

    pub fn encode_schema(&mut self, schema: &Schema) -> Result<()> {
        let schema_bytes = encode::schema(schema, self.options)?;
        self.out.extend_from_slice(&schema_bytes);
        Ok(())
    }

    pub fn encode_value(&mut self, schema: &Schema, value: &TpackValue<'_>) -> Result<()> {
        encode::ValueEncoder::new(&mut self.out, self.options).write_value(&schema.root, value)
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn decode_message(input: &[u8]) -> Result<Message<'_>> {
    Decoder::new(input).decode_message()
}

pub fn encode_message(
    schema: &Schema,
    value: &TpackValue<'_>,
    mode: EnvelopeMode,
    schema_id: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut encoder = Encoder::new();
    encoder.encode_message(schema, value, mode, schema_id)?;
    Ok(encoder.into_vec())
}

pub fn encode_schema(schema: &Schema) -> Result<Vec<u8>> {
    encode::schema(schema, EncodeOptions::default())
}

/// Derive the recommended SHA-256-based SchemaId bytes for a schema.
///
/// This helper follows the draft's recommended convention for
/// uncoordinated deployments: hash the canonical encoded TypeDescriptor
/// bytes only. It does not change the core wire format and does not make
/// SchemaId hash-derived by requirement.
pub fn recommended_schema_id_sha256(schema: &Schema) -> Result<[u8; 32]> {
    let schema_bytes = encode_schema(schema)?;
    let digest = Sha256::digest(schema_bytes);
    let mut output = [0u8; 32];
    output.copy_from_slice(digest.as_slice());
    Ok(output)
}

pub fn encode_value(schema: &Schema, value: &TpackValue<'_>) -> Result<Vec<u8>> {
    encode::value(&schema.root, value, EncodeOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{borrow::Cow, vec};

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

    fn flat_value<'a>() -> TpackValue<'a> {
        TpackValue::Struct(vec![
            (1, TpackValue::String(Cow::Borrowed("prod_001"))),
            (2, TpackValue::DecimalFixed(2_999_900)),
            (
                3,
                TpackValue::Decimal(Decimal {
                    scale: 3,
                    coefficient: 13_725,
                }),
            ),
            (4, TpackValue::I32(10)),
            (5, TpackValue::I64(1_715_000_000)),
        ])
    }

    fn flat_example_bytes() -> Vec<u8> {
        vec![
            0x54, 0x50, 0x41, 0x4B, 0x01, 0x00, 0x28, 0x20, 0x05, 0x01, 0x02, 0x69, 0x64, 0x00,
            0x0E, 0x40, 0x02, 0x05, 0x70, 0x72, 0x69, 0x63, 0x65, 0x00, 0x0D, 0x12, 0x04, 0x03,
            0x03, 0x74, 0x61, 0x78, 0x00, 0x0C, 0x04, 0x03, 0x71, 0x74, 0x79, 0x00, 0x04, 0x05,
            0x02, 0x74, 0x73, 0x00, 0x05, 0x08, 0x70, 0x72, 0x6F, 0x64, 0x5F, 0x30, 0x30, 0x31,
            0xB8, 0x99, 0xEE, 0x02, 0x06, 0xBA, 0xD6, 0x01, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00,
            0x00, 0x00, 0x66, 0x38, 0xD2, 0xC0,
        ]
    }

    #[test]
    fn draft_flat_record_roundtrips_exactly() {
        let schema = flat_schema();
        let value = flat_value();
        let encoded =
            encode_message(&schema, &value, EnvelopeMode::FullSchema, None).expect("encode");
        assert_eq!(encoded, flat_example_bytes());

        let decoded = decode_message(&encoded).expect("decode");
        assert_eq!(decoded.schema.as_ref(), &schema);
        assert_eq!(decoded.value, value);
    }

    #[test]
    fn canonical_rejects_overlong_varint() {
        let mut bytes = flat_example_bytes();
        bytes[6] = 0xA8;
        bytes.insert(7, 0x00);
        let mut decoder = Decoder::with_options(
            &bytes,
            DecodeOptions {
                canonical: CanonicalMode::Strict,
                ..DecodeOptions::default()
            },
        );
        assert!(matches!(
            decoder.decode_message().unwrap_err().kind(),
            ErrorKind::OverlongVarint
        ));
    }

    #[test]
    fn rejects_duplicate_map_keys() {
        let schema = Schema::new(TypeDescriptor::Map {
            max_count: None,
            key: Box::new(TypeDescriptor::String { max_len: None }),
            value: Box::new(TypeDescriptor::I32),
        });
        let value = TpackValue::Map(vec![
            ValueMapEntry {
                key: TpackValue::String(Cow::Borrowed("a")),
                value: TpackValue::I32(1),
            },
            ValueMapEntry {
                key: TpackValue::String(Cow::Borrowed("a")),
                value: TpackValue::I32(2),
            },
        ]);
        assert!(encode_message(&schema, &value, EnvelopeMode::FullSchema, None).is_err());
    }

    #[test]
    fn encode_schema_helper_rejects_oversized_serialized_schema() {
        let schema = Schema::new(TypeDescriptor::Struct(vec![Field::new(
            1,
            "schema_name",
            TypeDescriptor::Null,
        )]));
        let schema_len = encode::schema(&schema, EncodeOptions::default())
            .expect("encode schema")
            .len();
        let options = EncodeOptions {
            limits: Limits {
                max_schema_len: schema_len - 1,
                ..Limits::default()
            },
            ..EncodeOptions::default()
        };

        assert!(matches!(
            encode::schema(&schema, options).unwrap_err().kind(),
            ErrorKind::SchemaLengthExceeded
        ));
    }

    #[test]
    fn recommended_schema_id_sha256_is_stable_and_schema_sensitive() {
        let schema = flat_schema();
        let digest_a = recommended_schema_id_sha256(&schema).expect("derive schema id");
        let digest_b = recommended_schema_id_sha256(&schema).expect("derive schema id");
        assert_eq!(digest_a, digest_b);
        assert_eq!(digest_a.len(), 32);

        let modified_schema = Schema::new(TypeDescriptor::Struct(vec![
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
            Field::new(4, "qty", TypeDescriptor::I64),
            Field::new(5, "ts", TypeDescriptor::I64),
        ]));
        let digest_modified =
            recommended_schema_id_sha256(&modified_schema).expect("derive schema id");
        assert_ne!(digest_a, digest_modified);
    }
}
