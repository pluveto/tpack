use alloc::{borrow::Cow, boxed::Box, string::String, vec::Vec};

use crate::{Decimal, Error, ErrorKind, Result, Schema, TpackValue, TypeDescriptor};

pub trait TpackSerialize {
    fn schema() -> Schema
    where
        Self: Sized;

    fn to_tpack_value(&self) -> TpackValue<'_>;
}

pub trait TpackDeserialize<'de>: Sized {
    fn schema() -> Schema;

    fn from_tpack_value(value: TpackValue<'de>) -> Result<Self>;
}

pub trait FromTpackValue<'de>: Sized {
    fn from_value(value: TpackValue<'de>) -> Result<Self>;
}

impl TpackSerialize for bool {
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::Bool)
    }

    fn to_tpack_value(&self) -> TpackValue<'_> {
        TpackValue::Bool(*self)
    }
}

impl<'de> TpackDeserialize<'de> for bool {
    fn schema() -> Schema {
        <Self as TpackSerialize>::schema()
    }

    fn from_tpack_value(value: TpackValue<'de>) -> Result<Self> {
        bool::from_value(value)
    }
}

impl<'de> FromTpackValue<'de> for bool {
    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        match value {
            TpackValue::Bool(value) => Ok(value),
            _ => Err(type_mismatch("Bool")),
        }
    }
}

macro_rules! impl_int {
    ($ty:ty, $variant:ident, $desc:ident, $name:literal) => {
        impl TpackSerialize for $ty {
            fn schema() -> Schema {
                Schema::new(TypeDescriptor::$desc)
            }

            fn to_tpack_value(&self) -> TpackValue<'_> {
                TpackValue::$variant(*self)
            }
        }

        impl<'de> TpackDeserialize<'de> for $ty {
            fn schema() -> Schema {
                <Self as TpackSerialize>::schema()
            }

            fn from_tpack_value(value: TpackValue<'de>) -> Result<Self> {
                <$ty as FromTpackValue<'de>>::from_value(value)
            }
        }

        impl<'de> FromTpackValue<'de> for $ty {
            fn from_value(value: TpackValue<'de>) -> Result<Self> {
                match value {
                    TpackValue::$variant(value) => Ok(value),
                    _ => Err(type_mismatch($name)),
                }
            }
        }
    };
}

impl_int!(i8, I8, I8, "I8");
impl_int!(i16, I16, I16, "I16");
impl_int!(i32, I32, I32, "I32");
impl_int!(i64, I64, I64, "I64");
impl_int!(u8, U8, U8, "U8");
impl_int!(u16, U16, U16, "U16");
impl_int!(u32, U32, U32, "U32");
impl_int!(u64, U64, U64, "U64");

macro_rules! impl_float {
    ($ty:ty, $variant:ident, $desc:ident, $name:literal) => {
        impl TpackSerialize for $ty {
            fn schema() -> Schema {
                Schema::new(TypeDescriptor::$desc)
            }

            fn to_tpack_value(&self) -> TpackValue<'_> {
                TpackValue::$variant(*self)
            }
        }

        impl<'de> TpackDeserialize<'de> for $ty {
            fn schema() -> Schema {
                <Self as TpackSerialize>::schema()
            }

            fn from_tpack_value(value: TpackValue<'de>) -> Result<Self> {
                <$ty as FromTpackValue<'de>>::from_value(value)
            }
        }

        impl<'de> FromTpackValue<'de> for $ty {
            fn from_value(value: TpackValue<'de>) -> Result<Self> {
                match value {
                    TpackValue::$variant(value) => Ok(value),
                    _ => Err(type_mismatch($name)),
                }
            }
        }
    };
}

impl_float!(f32, F32, F32, "F32");
impl_float!(f64, F64, F64, "F64");

impl TpackSerialize for String {
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::String { max_len: None })
    }

    fn to_tpack_value(&self) -> TpackValue<'_> {
        TpackValue::String(Cow::Borrowed(self.as_str()))
    }
}

impl<'de> TpackDeserialize<'de> for String {
    fn schema() -> Schema {
        <Self as TpackSerialize>::schema()
    }

    fn from_tpack_value(value: TpackValue<'de>) -> Result<Self> {
        String::from_value(value)
    }
}

impl<'de> FromTpackValue<'de> for String {
    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        match value {
            TpackValue::String(value) => Ok(value.into_owned()),
            _ => Err(type_mismatch("String")),
        }
    }
}

impl TpackSerialize for &str {
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::String { max_len: None })
    }

    fn to_tpack_value(&self) -> TpackValue<'_> {
        TpackValue::String(Cow::Borrowed(*self))
    }
}

impl TpackSerialize for &[u8] {
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::Bytes { max_len: None })
    }

    fn to_tpack_value(&self) -> TpackValue<'_> {
        TpackValue::Bytes(Cow::Borrowed(*self))
    }
}

impl<T: TpackSerialize> TpackSerialize for Option<T> {
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::Optional(Box::new(T::schema().root)))
    }

    fn to_tpack_value(&self) -> TpackValue<'_> {
        TpackValue::Optional(self.as_ref().map(|value| Box::new(value.to_tpack_value())))
    }
}

impl<'de, T> TpackDeserialize<'de> for Option<T>
where
    T: TpackDeserialize<'de>,
{
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::Optional(Box::new(T::schema().root)))
    }

    fn from_tpack_value(value: TpackValue<'de>) -> Result<Self> {
        Option::<T>::from_value(value)
    }
}

impl<'de, T> FromTpackValue<'de> for Option<T>
where
    T: TpackDeserialize<'de>,
{
    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        match value {
            TpackValue::Optional(None) => Ok(None),
            TpackValue::Optional(Some(value)) => Ok(Some(T::from_tpack_value(*value)?)),
            _ => Err(type_mismatch("Optional")),
        }
    }
}

impl<T: TpackSerialize> TpackSerialize for Vec<T> {
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::List {
            max_count: None,
            element: Box::new(T::schema().root),
        })
    }

    fn to_tpack_value(&self) -> TpackValue<'_> {
        TpackValue::List(self.iter().map(TpackSerialize::to_tpack_value).collect())
    }
}

impl<'de, T> TpackDeserialize<'de> for Vec<T>
where
    T: TpackDeserialize<'de>,
{
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::List {
            max_count: None,
            element: Box::new(T::schema().root),
        })
    }

    fn from_tpack_value(value: TpackValue<'de>) -> Result<Self> {
        Vec::<T>::from_value(value)
    }
}

impl<'de, T> FromTpackValue<'de> for Vec<T>
where
    T: TpackDeserialize<'de>,
{
    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        match value {
            TpackValue::List(values) => values.into_iter().map(T::from_tpack_value).collect(),
            _ => Err(type_mismatch("List")),
        }
    }
}

impl TpackSerialize for Decimal {
    fn schema() -> Schema {
        Schema::new(TypeDescriptor::Decimal)
    }

    fn to_tpack_value(&self) -> TpackValue<'_> {
        TpackValue::Decimal(*self)
    }
}

impl<'de> TpackDeserialize<'de> for Decimal {
    fn schema() -> Schema {
        <Self as TpackSerialize>::schema()
    }

    fn from_tpack_value(value: TpackValue<'de>) -> Result<Self> {
        match value {
            TpackValue::Decimal(value) => Ok(value),
            _ => Err(type_mismatch("Decimal")),
        }
    }
}

fn type_mismatch(expected: &'static str) -> Error {
    Error::new(ErrorKind::TypeMismatch { expected })
}
