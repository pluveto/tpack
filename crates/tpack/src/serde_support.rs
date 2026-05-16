mod access;
mod error;
mod value;

use serde::de::Deserialize;
use tpack_core::{DecodeOptions, Decoder, Limits, Schema, SchemaRegistry, TpackValue};

use self::error::Error;
use self::value::ValueDeserializer;

/// Configurable entry point for serde-based TPACK deserialization.
///
/// This object carries the optional schema registry and decode limits so the
/// module API does not need a growing family of `_with_*` helper functions.
#[derive(Clone, Copy)]
pub struct Deserializer<'a> {
    registry: Option<&'a dyn SchemaRegistry>,
    decode_options: DecodeOptions,
}

impl<'a> Deserializer<'a> {
    /// Create a serde deserializer with default decode options.
    pub fn new() -> Self {
        Self {
            registry: None,
            decode_options: DecodeOptions::default(),
        }
    }

    /// Attach a schema registry for `SchemaRef` and cached `FullSchemaWithId`
    /// messages.
    pub fn registry<R>(mut self, registry: &'a R) -> Self
    where
        R: SchemaRegistry + 'a,
    {
        self.registry = Some(registry);
        self
    }

    /// Replace the underlying core decode options used for byte decoding.
    pub fn decode_options(mut self, options: DecodeOptions) -> Self {
        self.decode_options = options;
        self
    }

    /// Override only the resource limits while keeping other decode options.
    pub fn limits(mut self, limits: Limits) -> Self {
        self.decode_options.limits = limits;
        self
    }

    /// Deserialize a typed value from TPACK-encoded bytes.
    pub fn slice<'de, T>(&self, bytes: &'de [u8]) -> tpack_core::Result<T>
    where
        T: Deserialize<'de>,
    {
        let mut decoder = Decoder::with_options(bytes, self.decode_options);
        let message = match self.registry {
            Some(registry) => decoder.decode_message_with_registry(registry)?,
            None => decoder.decode_message()?,
        };
        self.value(&message.schema, message.value)
    }

    /// Deserialize a typed value from an already-decoded `TpackValue`.
    pub fn value<'de, T>(&self, schema: &Schema, value: TpackValue<'de>) -> tpack_core::Result<T>
    where
        T: Deserialize<'de>,
    {
        T::deserialize(ValueDeserializer::new(
            &schema.root,
            value,
            self.decode_options.limits.max_depth,
        ))
        .map_err(Error::into_core)
    }
}

impl Default for Deserializer<'_> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn from_slice<'de, T>(bytes: &'de [u8]) -> tpack_core::Result<T>
where
    T: Deserialize<'de>,
{
    Deserializer::new().slice(bytes)
}

pub fn from_value<'de, T>(schema: &Schema, value: TpackValue<'de>) -> tpack_core::Result<T>
where
    T: Deserialize<'de>,
{
    Deserializer::new().value(schema, value)
}
