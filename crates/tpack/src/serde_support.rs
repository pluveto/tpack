use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use tpack_core::{
    Decoder, Error as CoreError, ErrorKind, Field, Schema, SchemaRegistry, TpackValue,
    TypeDescriptor, ValueMapEntry,
};

pub fn from_slice<'de, T>(bytes: &'de [u8]) -> tpack_core::Result<T>
where
    T: Deserialize<'de>,
{
    let message = Decoder::new(bytes).decode_message()?;
    from_value(&message.schema, message.value)
}

pub fn from_slice_with_registry<'de, T, R>(bytes: &'de [u8], registry: &R) -> tpack_core::Result<T>
where
    T: Deserialize<'de>,
    R: SchemaRegistry + ?Sized,
{
    let message = Decoder::new(bytes).decode_message_with_registry(registry)?;
    from_value(&message.schema, message.value)
}

pub fn from_value<'de, T>(schema: &Schema, value: TpackValue<'de>) -> tpack_core::Result<T>
where
    T: Deserialize<'de>,
{
    T::deserialize(ValueDeserializer {
        ty: &schema.root,
        value,
    })
    .map_err(Error::into_core)
}

#[derive(Debug)]
struct Error {
    inner: CoreError,
}

impl Error {
    fn new(kind: ErrorKind) -> Self {
        Self {
            inner: CoreError::new(kind),
        }
    }

    fn invalid(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Invalid(message.into()))
    }

    fn limit(name: &'static str) -> Self {
        Self::new(ErrorKind::LimitExceeded(name))
    }

    fn into_core(self) -> CoreError {
        self.inner
    }
}

struct ValueDeserializer<'a, 'de> {
    ty: &'a TypeDescriptor,
    value: TpackValue<'de>,
}

struct SeqValueAccess<'a, 'de> {
    element_ty: &'a TypeDescriptor,
    values: std::vec::IntoIter<TpackValue<'de>>,
}

struct StructAccess<'a, 'de> {
    fields: &'a [Field],
    entries: std::vec::IntoIter<(usize, TpackValue<'de>)>,
    pending_value: Option<(usize, TpackValue<'de>)>,
}

struct MapValueAccess<'a, 'de> {
    key_ty: &'a TypeDescriptor,
    value_ty: &'a TypeDescriptor,
    entries: std::vec::IntoIter<ValueMapEntry<'de>>,
    pending_value: Option<TpackValue<'de>>,
}

struct EnumValueAccess<'a, 'de> {
    variant_name: &'a str,
    payload_ty: Option<&'a TypeDescriptor>,
    payload: Option<TpackValue<'de>>,
}

struct VariantValueAccess<'a, 'de> {
    payload_ty: Option<&'a TypeDescriptor>,
    payload: Option<TpackValue<'de>>,
}

fn struct_entries_by_id<'de>(
    fields: &[Field],
    values: Vec<(u64, TpackValue<'de>)>,
) -> Result<Vec<(usize, TpackValue<'de>)>, Error> {
    let field_indices = fields
        .iter()
        .enumerate()
        .map(|(index, field)| (field.id, index))
        .collect::<HashMap<_, _>>();
    let mut seen = vec![false; fields.len()];
    let mut entries = Vec::with_capacity(values.len().min(fields.len()));
    for (id, value) in values {
        let Some(&index) = field_indices.get(&id) else {
            continue;
        };
        if seen[index] {
            return Err(Error::invalid("duplicate struct field value"));
        }
        seen[index] = true;
        entries.push((index, value));
    }
    Ok(entries)
}

fn struct_values_by_schema<'de>(
    fields: &[Field],
    values: Vec<(u64, TpackValue<'de>)>,
) -> Result<Vec<Option<TpackValue<'de>>>, Error> {
    let mut by_schema = (0..fields.len()).map(|_| None).collect::<Vec<_>>();
    for (index, value) in struct_entries_by_id(fields, values)? {
        by_schema[index] = Some(value);
    }
    Ok(by_schema)
}

impl<'de, 'a> de::Deserializer<'de> for ValueDeserializer<'a, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.ty {
            TypeDescriptor::Null => self.deserialize_unit(visitor),
            TypeDescriptor::Bool => self.deserialize_bool(visitor),
            TypeDescriptor::I8 => self.deserialize_i8(visitor),
            TypeDescriptor::I16 => self.deserialize_i16(visitor),
            TypeDescriptor::I32 => self.deserialize_i32(visitor),
            TypeDescriptor::I64 | TypeDescriptor::Date | TypeDescriptor::Timestamp(_) => {
                self.deserialize_i64(visitor)
            }
            TypeDescriptor::U8 => self.deserialize_u8(visitor),
            TypeDescriptor::U16 => self.deserialize_u16(visitor),
            TypeDescriptor::U32 => self.deserialize_u32(visitor),
            TypeDescriptor::U64 | TypeDescriptor::Time => self.deserialize_u64(visitor),
            TypeDescriptor::F32 => self.deserialize_f32(visitor),
            TypeDescriptor::F64 => self.deserialize_f64(visitor),
            TypeDescriptor::String { .. } | TypeDescriptor::DateTimeTz => {
                self.deserialize_str(visitor)
            }
            TypeDescriptor::Bytes { .. } | TypeDescriptor::Extension { .. } => {
                self.deserialize_bytes(visitor)
            }
            TypeDescriptor::Optional(_) => self.deserialize_option(visitor),
            TypeDescriptor::Struct(_) => de::Deserializer::deserialize_map(self, visitor),
            TypeDescriptor::List { .. } => de::Deserializer::deserialize_seq(self, visitor),
            TypeDescriptor::Map { .. } => de::Deserializer::deserialize_map(self, visitor),
            TypeDescriptor::Union(_) | TypeDescriptor::Enum(_) => {
                self.deserialize_enum("", &[], visitor)
            }
            TypeDescriptor::Decimal
            | TypeDescriptor::DecimalFixed { .. }
            | TypeDescriptor::DateTime
            | TypeDescriptor::Duration
            | TypeDescriptor::BigInt
            | TypeDescriptor::BigUInt
            | TypeDescriptor::CalendarInterval => self.deserialize_newtype_struct("", visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::Bool(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_bool(value)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::I8(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_i8(value)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::I16(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_i16(value)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::I32(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_i32(value)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            TpackValue::I64(value)
            | TpackValue::Date(value)
            | TpackValue::Timestamp(value)
            | TpackValue::BigInt(value) => visitor.visit_i64(value),
            TpackValue::DecimalFixed(value) => visitor.visit_i64(value),
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::U8(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_u8(value)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::U16(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_u16(value)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::U32(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_u32(value)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            TpackValue::U64(value) | TpackValue::Time(value) | TpackValue::BigUInt(value) => {
                visitor.visit_u64(value)
            }
            TpackValue::Enum(index) => visitor.visit_u64(index),
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::F32(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            TpackValue::F64(value) => visitor.visit_f64(value),
            TpackValue::F32(value) => visitor.visit_f64(f64::from(value)),
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::String(value) = self.value else {
            return Err(type_mismatch(self.ty));
        };
        let mut chars = value.chars();
        let Some(ch) = chars.next() else {
            return Err(Error::invalid("empty string cannot deserialize as char"));
        };
        if chars.next().is_some() {
            return Err(Error::invalid(
                "multi-character string cannot deserialize as char",
            ));
        }
        visitor.visit_char(ch)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            TpackValue::String(Cow::Borrowed(value)) => visitor.visit_borrowed_str(value),
            TpackValue::String(Cow::Owned(value)) => visitor.visit_string(value),
            TpackValue::DateTimeTz { timezone, .. } => match timezone {
                Cow::Borrowed(value) => visitor.visit_borrowed_str(value),
                Cow::Owned(value) => visitor.visit_string(value),
            },
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            TpackValue::Bytes(Cow::Borrowed(value))
            | TpackValue::Extension(Cow::Borrowed(value)) => visitor.visit_borrowed_bytes(value),
            TpackValue::Bytes(Cow::Owned(value)) | TpackValue::Extension(Cow::Owned(value)) => {
                visitor.visit_byte_buf(value)
            }
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TypeDescriptor::Optional(inner) = self.ty else {
            return visitor.visit_some(self);
        };
        match self.value {
            TpackValue::Optional(None) => visitor.visit_none(),
            TpackValue::Optional(Some(value)) => visitor.visit_some(ValueDeserializer {
                ty: inner,
                value: *value,
            }),
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::Null = self.value else {
            return Err(type_mismatch(self.ty));
        };
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match (self.ty, self.value) {
            (TypeDescriptor::Decimal, TpackValue::Decimal(value)) => {
                visitor.visit_seq(SeqValueAccess::new_synthetic(vec![
                    (TypeDescriptor::I64, TpackValue::I64(value.scale)),
                    (TypeDescriptor::I64, TpackValue::I64(value.coefficient)),
                ]))
            }
            (TypeDescriptor::DecimalFixed { .. }, TpackValue::DecimalFixed(value)) => {
                visitor.visit_i64(value)
            }
            (TypeDescriptor::DateTime, TpackValue::DateTime { days, nanos }) => {
                visitor.visit_seq(SeqValueAccess::new_synthetic(vec![
                    (TypeDescriptor::I64, TpackValue::I64(days)),
                    (TypeDescriptor::U64, TpackValue::U64(nanos)),
                ]))
            }
            (TypeDescriptor::Duration, TpackValue::Duration(value)) => {
                visitor.visit_seq(SeqValueAccess::new_synthetic(vec![
                    (TypeDescriptor::I64, TpackValue::I64(value.seconds)),
                    (TypeDescriptor::I64, TpackValue::I64(value.nanos)),
                ]))
            }
            (TypeDescriptor::CalendarInterval, TpackValue::CalendarInterval(value)) => visitor
                .visit_seq(SeqValueAccess::new_synthetic(vec![
                    (TypeDescriptor::I64, TpackValue::I64(value.months)),
                    (TypeDescriptor::I64, TpackValue::I64(value.days)),
                    (TypeDescriptor::I64, TpackValue::I64(value.nanos)),
                ])),
            (ty, value) => visitor.visit_newtype_struct(ValueDeserializer { ty, value }),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match (self.ty, self.value) {
            (TypeDescriptor::List { element, .. }, TpackValue::List(values)) => {
                visitor.visit_seq(SeqValueAccess {
                    element_ty: element,
                    values: values.into_iter(),
                })
            }
            (TypeDescriptor::Struct(fields), TpackValue::Struct(values)) => {
                let values = struct_values_by_schema(fields, values)?;
                visitor.visit_seq(StructTupleAccess {
                    fields,
                    values,
                    index: 0,
                })
            }
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self, visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match (self.ty, self.value) {
            (TypeDescriptor::Struct(fields), TpackValue::Struct(values)) => {
                let entries = struct_entries_by_id(fields, values)?;
                visitor.visit_map(StructAccess {
                    fields,
                    entries: entries.into_iter(),
                    pending_value: None,
                })
            }
            (TypeDescriptor::Map { key, value, .. }, TpackValue::Map(entries)) => visitor
                .visit_map(MapValueAccess {
                    key_ty: key,
                    value_ty: value,
                    entries: entries.into_iter(),
                    pending_value: None,
                }),
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self, visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match (self.ty, self.value) {
            (TypeDescriptor::Enum(symbols), TpackValue::Enum(index)) => {
                let index = usize::try_from(index).map_err(|_| Error::limit("enum index"))?;
                let name = symbols
                    .get(index)
                    .ok_or(Error::invalid("enum symbol index out of range"))?;
                visitor.visit_enum(EnumValueAccess {
                    variant_name: name,
                    payload_ty: None,
                    payload: None,
                })
            }
            (TypeDescriptor::Union(variants), TpackValue::Union { index, value }) => {
                let index = usize::try_from(index).map_err(|_| Error::limit("variant index"))?;
                let variant = variants
                    .get(index)
                    .ok_or(Error::invalid("union variant index out of range"))?;
                visitor.visit_enum(EnumValueAccess {
                    variant_name: &variant.name,
                    payload_ty: Some(&variant.ty),
                    payload: Some(*value),
                })
            }
            _ => Err(type_mismatch(self.ty)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct StructTupleAccess<'a, 'de> {
    fields: &'a [Field],
    values: Vec<Option<TpackValue<'de>>>,
    index: usize,
}

impl<'de> SeqValueAccess<'_, 'de> {
    fn new_synthetic(values: Vec<(TypeDescriptor, TpackValue<'de>)>) -> SyntheticSeqAccess<'de> {
        SyntheticSeqAccess {
            values: values.into_iter(),
        }
    }
}

struct SyntheticSeqAccess<'de> {
    values: std::vec::IntoIter<(TypeDescriptor, TpackValue<'de>)>,
}

impl<'de, 'a> SeqAccess<'de> for SeqValueAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(value) = self.values.next() else {
            return Ok(None);
        };
        seed.deserialize(ValueDeserializer {
            ty: self.element_ty,
            value,
        })
        .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

impl<'de, 'a> SeqAccess<'de> for StructTupleAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(field) = self.fields.get(self.index) else {
            return Ok(None);
        };
        let value = self.values[self.index]
            .take()
            .ok_or(Error::invalid("missing struct field value"))?;
        self.index += 1;
        seed.deserialize(ValueDeserializer {
            ty: &field.ty,
            value,
        })
        .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.fields.len().saturating_sub(self.index))
    }
}

impl<'de> SeqAccess<'de> for SyntheticSeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some((ty, value)) = self.values.next() else {
            return Ok(None);
        };
        seed.deserialize(ValueDeserializer { ty: &ty, value })
            .map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.values.len())
    }
}

impl<'de, 'a> MapAccess<'de> for StructAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.pending_value.is_some() {
            return Err(Error::invalid("struct key requested before value"));
        }
        let Some((index, value)) = self.entries.next() else {
            return Ok(None);
        };
        self.pending_value = Some((index, value));
        let key = serde::de::value::StrDeserializer::<Error>::new(self.fields[index].name.as_str());
        seed.deserialize(key).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let (index, value) = self
            .pending_value
            .take()
            .ok_or(Error::invalid("struct value requested before key"))?;
        seed.deserialize(ValueDeserializer {
            ty: &self.fields[index].ty,
            value,
        })
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len() + usize::from(self.pending_value.is_some()))
    }
}

impl<'de, 'a> MapAccess<'de> for MapValueAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some(entry) = self.entries.next() else {
            return Ok(None);
        };
        self.pending_value = Some(entry.value);
        seed.deserialize(ValueDeserializer {
            ty: self.key_ty,
            value: entry.key,
        })
        .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self
            .pending_value
            .take()
            .ok_or(Error::invalid("map value requested before key"))?;
        seed.deserialize(ValueDeserializer {
            ty: self.value_ty,
            value,
        })
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}

impl<'de, 'a> EnumAccess<'de> for EnumValueAccess<'a, 'de> {
    type Error = Error;
    type Variant = VariantValueAccess<'a, 'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(serde::de::value::StrDeserializer::<Error>::new(
            self.variant_name,
        ))?;
        Ok((
            variant,
            VariantValueAccess {
                payload_ty: self.payload_ty,
                payload: self.payload,
            },
        ))
    }
}

impl<'de, 'a> VariantAccess<'de> for VariantValueAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        if self.payload.is_some() {
            return Err(Error::invalid("expected unit enum variant"));
        }
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or(Error::invalid("expected data enum variant"))?;
        let value = self
            .payload
            .ok_or(Error::invalid("missing union variant payload"))?;
        seed.deserialize(ValueDeserializer { ty, value })
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or(Error::invalid("expected tuple enum variant"))?;
        let value = self
            .payload
            .ok_or(Error::invalid("missing union variant payload"))?;
        de::Deserializer::deserialize_seq(ValueDeserializer { ty, value }, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let ty = self
            .payload_ty
            .ok_or(Error::invalid("expected struct enum variant"))?;
        let value = self
            .payload
            .ok_or(Error::invalid("missing union variant payload"))?;
        de::Deserializer::deserialize_map(ValueDeserializer { ty, value }, visitor)
    }
}

fn type_mismatch(ty: &TypeDescriptor) -> Error {
    Error::new(ErrorKind::TypeMismatch {
        expected: type_name(ty),
    })
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

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Error::invalid(msg.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {}
