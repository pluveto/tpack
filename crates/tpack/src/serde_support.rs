mod access;
mod error;
mod value;

use serde::de::Deserialize;
use tpack_core::{Decoder, Schema, SchemaRegistry, TpackValue};

use self::error::Error;
use self::value::ValueDeserializer;

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
    T::deserialize(ValueDeserializer::new(&schema.root, value)).map_err(Error::into_core)
}
