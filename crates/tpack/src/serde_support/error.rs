use std::error::Error as StdError;
use std::fmt;

use serde::de;
use tpack_core::{Error as CoreError, ErrorPath, TypeDescriptor};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Core,
    DepthLimitExceeded,
    TypeMismatch { expected: &'static str },
    DuplicateStructFieldValue,
    MissingStructFieldValue,
    StructKeyBeforeValue,
    StructValueBeforeKey,
    MapValueBeforeKey,
    ExpectedUnitEnumVariant,
    ExpectedDataEnumVariant,
    MissingUnionVariantPayload,
    ExpectedTupleEnumVariant,
    ExpectedStructEnumVariant,
    EmptyStringForChar,
    MultiCharacterStringForChar,
    EnumSymbolIndexOutOfRange,
    UnionVariantIndexOutOfRange,
    Custom(String),
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    path: ErrorPath,
    source: Option<Box<dyn StdError + 'static>>,
}

impl Error {
    pub(super) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            path: ErrorPath::default(),
            source: None,
        }
    }

    pub(super) fn from_core(error: CoreError) -> Self {
        Self::new(ErrorKind::Core).with_source(error)
    }

    pub(super) fn depth_limit() -> Self {
        Self::new(ErrorKind::DepthLimitExceeded)
    }

    pub(super) fn type_mismatch(ty: &TypeDescriptor) -> Self {
        Self::new(ErrorKind::TypeMismatch {
            expected: ty.type_label(),
        })
    }

    pub(super) fn custom(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Custom(message.into()))
    }

    pub(super) fn duplicate_struct_field_value() -> Self {
        Self::new(ErrorKind::DuplicateStructFieldValue)
    }

    pub(super) fn missing_struct_field_value() -> Self {
        Self::new(ErrorKind::MissingStructFieldValue)
    }

    pub(super) fn struct_key_before_value() -> Self {
        Self::new(ErrorKind::StructKeyBeforeValue)
    }

    pub(super) fn struct_value_before_key() -> Self {
        Self::new(ErrorKind::StructValueBeforeKey)
    }

    pub(super) fn map_value_before_key() -> Self {
        Self::new(ErrorKind::MapValueBeforeKey)
    }

    pub(super) fn expected_unit_enum_variant() -> Self {
        Self::new(ErrorKind::ExpectedUnitEnumVariant)
    }

    pub(super) fn expected_data_enum_variant() -> Self {
        Self::new(ErrorKind::ExpectedDataEnumVariant)
    }

    pub(super) fn missing_union_variant_payload() -> Self {
        Self::new(ErrorKind::MissingUnionVariantPayload)
    }

    pub(super) fn expected_tuple_enum_variant() -> Self {
        Self::new(ErrorKind::ExpectedTupleEnumVariant)
    }

    pub(super) fn expected_struct_enum_variant() -> Self {
        Self::new(ErrorKind::ExpectedStructEnumVariant)
    }

    pub(super) fn empty_string_for_char() -> Self {
        Self::new(ErrorKind::EmptyStringForChar)
    }

    pub(super) fn multi_character_string_for_char() -> Self {
        Self::new(ErrorKind::MultiCharacterStringForChar)
    }

    pub(super) fn enum_symbol_index_out_of_range() -> Self {
        Self::new(ErrorKind::EnumSymbolIndexOutOfRange)
    }

    pub(super) fn union_variant_index_out_of_range() -> Self {
        Self::new(ErrorKind::UnionVariantIndexOutOfRange)
    }

    pub(super) fn at_field(mut self, field: impl Into<String>) -> Self {
        self.path.prepend(tpack_core::PathSegment::field(field));
        self
    }

    pub(super) fn at_index(mut self, index: usize) -> Self {
        self.path.prepend(tpack_core::PathSegment::index(index));
        self
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn path(&self) -> &ErrorPath {
        &self.path
    }

    fn with_source(mut self, source: impl StdError + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Error::custom(msg.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::Core => {
                if let Some(source) = &self.source {
                    write!(f, "TPACK decode error: {source}")?
                } else {
                    f.write_str("TPACK decode error")?
                }
            }
            ErrorKind::DepthLimitExceeded => f.write_str("value depth limit exceeded")?,
            ErrorKind::TypeMismatch { expected } => {
                write!(f, "TPACK type mismatch, expected {expected}")?
            }
            ErrorKind::DuplicateStructFieldValue => f.write_str("duplicate struct field value")?,
            ErrorKind::MissingStructFieldValue => f.write_str("missing struct field value")?,
            ErrorKind::StructKeyBeforeValue => f.write_str("struct key requested before value")?,
            ErrorKind::StructValueBeforeKey => f.write_str("struct value requested before key")?,
            ErrorKind::MapValueBeforeKey => f.write_str("map value requested before key")?,
            ErrorKind::ExpectedUnitEnumVariant => f.write_str("expected unit enum variant")?,
            ErrorKind::ExpectedDataEnumVariant => f.write_str("expected data enum variant")?,
            ErrorKind::MissingUnionVariantPayload => {
                f.write_str("missing union variant payload")?
            }
            ErrorKind::ExpectedTupleEnumVariant => f.write_str("expected tuple enum variant")?,
            ErrorKind::ExpectedStructEnumVariant => f.write_str("expected struct enum variant")?,
            ErrorKind::EmptyStringForChar => {
                f.write_str("empty string cannot deserialize as char")?
            }
            ErrorKind::MultiCharacterStringForChar => {
                f.write_str("multi-character string cannot deserialize as char")?
            }
            ErrorKind::EnumSymbolIndexOutOfRange => {
                f.write_str("enum symbol index out of range")?
            }
            ErrorKind::UnionVariantIndexOutOfRange => {
                f.write_str("union variant index out of range")?
            }
            ErrorKind::Custom(message) => f.write_str(message)?,
        }
        if !self.path.is_empty() {
            write!(f, " at {}", self.path)?;
        }
        Ok(())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref()
    }
}
