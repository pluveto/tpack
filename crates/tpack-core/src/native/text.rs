use alloc::{borrow::Cow, string::String};

use crate::{Result, Schema, TpackValue};

use super::helpers::{bytes_schema, deserialize_via_from_value, string_schema, type_mismatch};
use super::{FromTpackValue, TpackDeserialize, TpackSerialize};

impl TpackSerialize for String {
    fn schema() -> Schema {
        string_schema()
    }

    fn to_value(&self) -> TpackValue<'_> {
        TpackValue::String(Cow::Borrowed(self.as_str()))
    }
}

impl<'de> TpackDeserialize<'de> for String {
    fn schema() -> Schema {
        <Self as TpackSerialize>::schema()
    }

    fn from_value(value: TpackValue<'de>) -> Result<Self> {
        deserialize_via_from_value(value)
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
        string_schema()
    }

    fn to_value(&self) -> TpackValue<'_> {
        TpackValue::String(Cow::Borrowed(*self))
    }
}

impl TpackSerialize for &[u8] {
    fn schema() -> Schema {
        bytes_schema()
    }

    fn to_value(&self) -> TpackValue<'_> {
        TpackValue::Bytes(Cow::Borrowed(*self))
    }
}
