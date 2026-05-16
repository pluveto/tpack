use crate::{Result, Schema, TpackValue};

pub trait TpackSerialize {
    fn schema() -> Schema
    where
        Self: Sized;

    fn to_value(&self) -> TpackValue<'_>;
}

pub trait TpackDeserialize<'de>: Sized {
    fn schema() -> Schema;

    fn from_value(value: TpackValue<'de>) -> Result<Self>;
}

pub trait FromTpackValue<'de>: Sized {
    fn from_value(value: TpackValue<'de>) -> Result<Self>;
}

mod collections;
mod helpers;
mod primitives;
mod text;
