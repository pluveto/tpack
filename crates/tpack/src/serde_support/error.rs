use std::borrow::Cow;
use std::fmt;

use serde::de;
use tpack_core::{Error as CoreError, ErrorKind, TypeDescriptor};

#[derive(Debug)]
pub(super) struct Error {
    inner: CoreError,
}

impl Error {
    pub(super) fn new(kind: ErrorKind) -> Self {
        Self {
            inner: CoreError::new(kind),
        }
    }

    pub(super) fn invalid(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Invalid(message.into()))
    }

    pub(super) fn limit(name: &'static str) -> Self {
        Self::new(ErrorKind::LimitExceeded(name))
    }

    pub(super) fn type_mismatch(ty: &TypeDescriptor) -> Self {
        Self::new(ErrorKind::TypeMismatch {
            expected: type_name(ty),
        })
    }

    pub(super) fn into_core(self) -> CoreError {
        self.inner
    }
}

fn type_name(ty: &TypeDescriptor) -> &'static str {
    match ty {
        TypeDescriptor::Null => "Null",
        TypeDescriptor::Bool => "Bool",
        TypeDescriptor::I8 => "I8",
        TypeDescriptor::I16 => "I16",
        TypeDescriptor::I32 => "I32",
        TypeDescriptor::I64 => "I64",
        TypeDescriptor::U8 => "U8",
        TypeDescriptor::U16 => "U16",
        TypeDescriptor::U32 => "U32",
        TypeDescriptor::U64 => "U64",
        TypeDescriptor::F32 => "F32",
        TypeDescriptor::F64 => "F64",
        TypeDescriptor::Decimal => "Decimal",
        TypeDescriptor::DecimalFixed { .. } => "Decimal(P,S)",
        TypeDescriptor::String { .. } => "String",
        TypeDescriptor::Bytes { .. } => "Bytes",
        TypeDescriptor::Date => "Date",
        TypeDescriptor::Time => "Time",
        TypeDescriptor::DateTime => "DateTime",
        TypeDescriptor::DateTimeTz => "DateTimeTZ",
        TypeDescriptor::Timestamp(_) => "Timestamp(P)",
        TypeDescriptor::Duration => "Duration",
        TypeDescriptor::BigInt => "BigInt",
        TypeDescriptor::BigUInt => "BigUInt",
        TypeDescriptor::CalendarInterval => "CalendarInterval",
        TypeDescriptor::Struct(_) => "Struct",
        TypeDescriptor::List { .. } => "List",
        TypeDescriptor::Map { .. } => "Map",
        TypeDescriptor::Union(_) => "Union",
        TypeDescriptor::Enum(_) => "Enum",
        TypeDescriptor::Optional(_) => "Optional",
        TypeDescriptor::Extension { .. } => "Extension",
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Error::invalid(msg.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {}
