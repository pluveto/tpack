use alloc::boxed::Box;

use crate::{Error, ErrorKind, Result, Schema, TpackValue, TypeDescriptor};

use super::FromTpackValue;

pub(super) fn bytes_schema() -> Schema {
    Schema::new(TypeDescriptor::Bytes { max_len: None })
}

pub(super) fn deserialize_via_from_value<'de, T>(value: TpackValue<'de>) -> Result<T>
where
    T: FromTpackValue<'de>,
{
    <T as FromTpackValue<'de>>::from_value(value)
}

pub(super) fn list_schema(element: Schema) -> Schema {
    Schema::new(TypeDescriptor::List {
        max_count: None,
        element: Box::new(element.root),
    })
}

pub(super) fn optional_schema(element: Schema) -> Schema {
    Schema::new(TypeDescriptor::Optional(Box::new(element.root)))
}

pub(super) fn string_schema() -> Schema {
    Schema::new(TypeDescriptor::String { max_len: None })
}

pub(super) fn type_mismatch(expected: &'static str) -> Error {
    Error::new(ErrorKind::TypeMismatch { expected })
}
