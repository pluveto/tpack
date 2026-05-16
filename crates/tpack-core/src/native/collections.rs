use alloc::{boxed::Box, vec::Vec};

use crate::{Result, Schema, TpackValue};

use super::helpers::{deserialize_via_from_value, list_schema, optional_schema, type_mismatch};
use super::{FromTpackValue, TpackDeserialize, TpackSerialize};

impl<T> TpackSerialize for Option<T>
where
    T: TpackSerialize,
{
    fn schema() -> Schema {
        optional_schema(T::schema())
    }

    fn to_value(&self) -> TpackValue<'_> {
        TpackValue::Optional(self.as_ref().map(|value| Box::new(value.to_value())))
    }
}

impl<'de, T> TpackDeserialize<'de> for Option<T>
where
    T: TpackDeserialize<'de>,
    Option<T>: FromTpackValue<'de>,
{
    fn schema() -> Schema {
        optional_schema(T::schema())
    }

    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        deserialize_via_from_value(value)
    }
}

impl<'de, T> FromTpackValue<'de> for Option<T>
where
    T: TpackDeserialize<'de>,
{
    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        match value {
            TpackValue::Optional(None) => Ok(None),
            TpackValue::Optional(Some(value)) => {
                Ok(Some(<T as TpackDeserialize<'de>>::from_value(*value)?))
            }
            _ => Err(type_mismatch("Optional")),
        }
    }
}

impl<T> TpackSerialize for Vec<T>
where
    T: TpackSerialize,
{
    fn schema() -> Schema {
        list_schema(T::schema())
    }

    fn to_value(&self) -> TpackValue<'_> {
        TpackValue::List(self.iter().map(TpackSerialize::to_value).collect())
    }
}

impl<'de, T> TpackDeserialize<'de> for Vec<T>
where
    T: TpackDeserialize<'de>,
    Vec<T>: FromTpackValue<'de>,
{
    fn schema() -> Schema {
        list_schema(T::schema())
    }

    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        deserialize_via_from_value(value)
    }
}

impl<'de, T> FromTpackValue<'de> for Vec<T>
where
    T: TpackDeserialize<'de>,
{
    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        match value {
            TpackValue::List(values) => values
                .into_iter()
                .map(<T as TpackDeserialize<'de>>::from_value)
                .collect(),
            _ => Err(type_mismatch("List")),
        }
    }
}
