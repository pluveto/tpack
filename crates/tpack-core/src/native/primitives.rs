use crate::{Decimal, Result, Schema, TpackValue, TypeDescriptor};

use super::helpers::{deserialize_via_from_value, type_mismatch};
use super::{FromTpackValue, TpackDeserialize, TpackSerialize};

macro_rules! impl_scalar {
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
                deserialize_via_from_value(value)
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

impl_scalar!(bool, Bool, Bool, "Bool");
impl_scalar!(i8, I8, I8, "I8");
impl_scalar!(i16, I16, I16, "I16");
impl_scalar!(i32, I32, I32, "I32");
impl_scalar!(i64, I64, I64, "I64");
impl_scalar!(u8, U8, U8, "U8");
impl_scalar!(u16, U16, U16, "U16");
impl_scalar!(u32, U32, U32, "U32");
impl_scalar!(u64, U64, U64, "U64");
impl_scalar!(f32, F32, F32, "F32");
impl_scalar!(f64, F64, F64, "F64");
impl_scalar!(Decimal, Decimal, Decimal, "Decimal");
