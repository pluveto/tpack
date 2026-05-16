#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod codec;
mod error;
mod native;
mod registry;
mod schema;
mod value;

pub use codec::{
    CanonicalMode, DecodeOptions, Decoder, EncodeOptions, Encoder, Limits, MAGIC, VERSION,
    decode_message, encode_message, encode_schema, encode_value,
};
pub use error::{Error, ErrorKind, Result};
pub use native::{FromTpackValue, TpackDeserialize, TpackSerialize};
pub use registry::{SchemaRegistry, empty_registry};
pub use schema::{
    CalendarInterval, Decimal, Duration, Envelope, EnvelopeMode, Field, Message, Schema, SchemaId,
    TimestampPrecision, TypeDescriptor, Variant,
};
pub use value::{TpackValue, ValueMapEntry};
