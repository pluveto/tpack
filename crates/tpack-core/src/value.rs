use alloc::{borrow::Cow, vec::Vec};

use crate::{CalendarInterval, Decimal, Duration};

#[derive(Debug, Clone, PartialEq)]
pub enum TpackValue<'de> {
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Decimal(Decimal),
    DecimalFixed(i64),
    String(Cow<'de, str>),
    Bytes(Cow<'de, [u8]>),
    Date(i64),
    Time(u64),
    DateTime {
        days: i64,
        nanos: u64,
    },
    DateTimeTz {
        days: i64,
        nanos: u64,
        timezone: Cow<'de, str>,
    },
    Timestamp(i64),
    Duration(Duration),
    BigInt(i64),
    BigUInt(u64),
    CalendarInterval(CalendarInterval),
    Struct(Vec<(u64, TpackValue<'de>)>),
    List(Vec<TpackValue<'de>>),
    Map(Vec<ValueMapEntry<'de>>),
    Union {
        index: u64,
        value: alloc::boxed::Box<TpackValue<'de>>,
    },
    Enum(u64),
    Optional(Option<alloc::boxed::Box<TpackValue<'de>>>),
    Extension(Cow<'de, [u8]>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueMapEntry<'de> {
    pub key: TpackValue<'de>,
    pub value: TpackValue<'de>,
}

impl<'de> TpackValue<'de> {
    pub fn is_composite(&self) -> bool {
        matches!(
            self,
            Self::Struct(_)
                | Self::List(_)
                | Self::Map(_)
                | Self::Union { .. }
                | Self::Optional(Some(_))
        )
    }

    pub fn into_owned(self) -> TpackValue<'static> {
        match self {
            TpackValue::Null => TpackValue::Null,
            TpackValue::Bool(v) => TpackValue::Bool(v),
            TpackValue::I8(v) => TpackValue::I8(v),
            TpackValue::I16(v) => TpackValue::I16(v),
            TpackValue::I32(v) => TpackValue::I32(v),
            TpackValue::I64(v) => TpackValue::I64(v),
            TpackValue::U8(v) => TpackValue::U8(v),
            TpackValue::U16(v) => TpackValue::U16(v),
            TpackValue::U32(v) => TpackValue::U32(v),
            TpackValue::U64(v) => TpackValue::U64(v),
            TpackValue::F32(v) => TpackValue::F32(v),
            TpackValue::F64(v) => TpackValue::F64(v),
            TpackValue::Decimal(v) => TpackValue::Decimal(v),
            TpackValue::DecimalFixed(v) => TpackValue::DecimalFixed(v),
            TpackValue::String(v) => TpackValue::String(Cow::Owned(v.into_owned())),
            TpackValue::Bytes(v) => TpackValue::Bytes(Cow::Owned(v.into_owned())),
            TpackValue::Date(v) => TpackValue::Date(v),
            TpackValue::Time(v) => TpackValue::Time(v),
            TpackValue::DateTime { days, nanos } => TpackValue::DateTime { days, nanos },
            TpackValue::DateTimeTz {
                days,
                nanos,
                timezone,
            } => TpackValue::DateTimeTz {
                days,
                nanos,
                timezone: Cow::Owned(timezone.into_owned()),
            },
            TpackValue::Timestamp(v) => TpackValue::Timestamp(v),
            TpackValue::Duration(v) => TpackValue::Duration(v),
            TpackValue::BigInt(v) => TpackValue::BigInt(v),
            TpackValue::BigUInt(v) => TpackValue::BigUInt(v),
            TpackValue::CalendarInterval(v) => TpackValue::CalendarInterval(v),
            TpackValue::Struct(fields) => TpackValue::Struct(
                fields
                    .into_iter()
                    .map(|(id, value)| (id, value.into_owned()))
                    .collect(),
            ),
            TpackValue::List(items) => {
                TpackValue::List(items.into_iter().map(TpackValue::into_owned).collect())
            }
            TpackValue::Map(entries) => TpackValue::Map(
                entries
                    .into_iter()
                    .map(|entry| ValueMapEntry {
                        key: entry.key.into_owned(),
                        value: entry.value.into_owned(),
                    })
                    .collect(),
            ),
            TpackValue::Union { index, value } => TpackValue::Union {
                index,
                value: alloc::boxed::Box::new(value.into_owned()),
            },
            TpackValue::Enum(index) => TpackValue::Enum(index),
            TpackValue::Optional(value) => {
                TpackValue::Optional(value.map(|value| alloc::boxed::Box::new(value.into_owned())))
            }
            TpackValue::Extension(value) => TpackValue::Extension(Cow::Owned(value.into_owned())),
        }
    }
}
