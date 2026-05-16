use std::borrow::Cow;

use serde::de::{self, Visitor};
use tpack_core::{TpackValue, TypeDescriptor};

use super::access::{
    EnumValueAccess, MapValueAccess, SeqValueAccess, StructAccess, StructTupleAccess,
};
use super::error::Error;

pub(super) struct ValueDeserializer<'a, 'de> {
    ty: &'a TypeDescriptor,
    value: TpackValue<'de>,
    remaining_depth: usize,
}

impl<'a, 'de> ValueDeserializer<'a, 'de> {
    pub(super) fn new(
        ty: &'a TypeDescriptor,
        value: TpackValue<'de>,
        remaining_depth: usize,
    ) -> Self {
        Self {
            ty,
            value,
            remaining_depth,
        }
    }

    pub(super) fn child(
        ty: &'a TypeDescriptor,
        value: TpackValue<'de>,
        remaining_depth: usize,
    ) -> Result<Self, Error> {
        let remaining_depth = remaining_depth
            .checked_sub(1)
            .ok_or_else(|| Error::limit("value depth"))?;
        Ok(Self::new(ty, value, remaining_depth))
    }
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
            return Err(Error::type_mismatch(self.ty));
        };
        visitor.visit_bool(value)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::I8(value) = self.value else {
            return Err(Error::type_mismatch(self.ty));
        };
        visitor.visit_i8(value)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::I16(value) = self.value else {
            return Err(Error::type_mismatch(self.ty));
        };
        visitor.visit_i16(value)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::I32(value) = self.value else {
            return Err(Error::type_mismatch(self.ty));
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
            _ => Err(Error::type_mismatch(self.ty)),
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
            return Err(Error::type_mismatch(self.ty));
        };
        visitor.visit_u8(value)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::U16(value) = self.value else {
            return Err(Error::type_mismatch(self.ty));
        };
        visitor.visit_u16(value)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::U32(value) = self.value else {
            return Err(Error::type_mismatch(self.ty));
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
            _ => Err(Error::type_mismatch(self.ty)),
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
            return Err(Error::type_mismatch(self.ty));
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
            _ => Err(Error::type_mismatch(self.ty)),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::String(value) = self.value else {
            return Err(Error::type_mismatch(self.ty));
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
            _ => Err(Error::type_mismatch(self.ty)),
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
            _ => Err(Error::type_mismatch(self.ty)),
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
            TpackValue::Optional(Some(value)) => visitor.visit_some(ValueDeserializer::child(
                inner,
                *value,
                self.remaining_depth,
            )?),
            _ => Err(Error::type_mismatch(self.ty)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let TpackValue::Null = self.value else {
            return Err(Error::type_mismatch(self.ty));
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
                visitor.visit_seq(SeqValueAccess::new_synthetic(
                    vec![
                        (TypeDescriptor::I64, TpackValue::I64(value.scale)),
                        (TypeDescriptor::I64, TpackValue::I64(value.coefficient)),
                    ],
                    self.remaining_depth,
                ))
            }
            (TypeDescriptor::DecimalFixed { .. }, TpackValue::DecimalFixed(value)) => {
                visitor.visit_i64(value)
            }
            (TypeDescriptor::DateTime, TpackValue::DateTime { days, nanos }) => {
                visitor.visit_seq(SeqValueAccess::new_synthetic(
                    vec![
                        (TypeDescriptor::I64, TpackValue::I64(days)),
                        (TypeDescriptor::U64, TpackValue::U64(nanos)),
                    ],
                    self.remaining_depth,
                ))
            }
            (TypeDescriptor::Duration, TpackValue::Duration(value)) => {
                visitor.visit_seq(SeqValueAccess::new_synthetic(
                    vec![
                        (TypeDescriptor::I64, TpackValue::I64(value.seconds)),
                        (TypeDescriptor::I64, TpackValue::I64(value.nanos)),
                    ],
                    self.remaining_depth,
                ))
            }
            (TypeDescriptor::CalendarInterval, TpackValue::CalendarInterval(value)) => visitor
                .visit_seq(SeqValueAccess::new_synthetic(
                    vec![
                        (TypeDescriptor::I64, TpackValue::I64(value.months)),
                        (TypeDescriptor::I64, TpackValue::I64(value.days)),
                        (TypeDescriptor::I64, TpackValue::I64(value.nanos)),
                    ],
                    self.remaining_depth,
                )),
            (ty, value) => visitor.visit_newtype_struct(ValueDeserializer::new(
                ty,
                value,
                self.remaining_depth,
            )),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match (self.ty, self.value) {
            (TypeDescriptor::List { element, .. }, TpackValue::List(values)) => {
                visitor.visit_seq(SeqValueAccess::new(element, values, self.remaining_depth))
            }
            (TypeDescriptor::Struct(fields), TpackValue::Struct(values)) => visitor.visit_seq(
                StructTupleAccess::new(fields, values, self.remaining_depth)?,
            ),
            _ => Err(Error::type_mismatch(self.ty)),
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
                visitor.visit_map(StructAccess::new(fields, values, self.remaining_depth)?)
            }
            (TypeDescriptor::Map { key, value, .. }, TpackValue::Map(entries)) => visitor
                .visit_map(MapValueAccess::new(
                    key,
                    value,
                    entries,
                    self.remaining_depth,
                )),
            _ => Err(Error::type_mismatch(self.ty)),
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
                visitor.visit_enum(EnumValueAccess::unit(name))
            }
            (TypeDescriptor::Union(variants), TpackValue::Union { index, value }) => {
                let index = usize::try_from(index).map_err(|_| Error::limit("variant index"))?;
                let variant = variants
                    .get(index)
                    .ok_or(Error::invalid("union variant index out of range"))?;
                visitor.visit_enum(EnumValueAccess::with_payload(
                    &variant.name,
                    &variant.ty,
                    *value,
                    self.remaining_depth,
                ))
            }
            _ => Err(Error::type_mismatch(self.ty)),
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
