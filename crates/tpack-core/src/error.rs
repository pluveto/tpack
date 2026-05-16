use alloc::{borrow::Cow, boxed::Box, string::String, vec::Vec};
use core::{fmt, fmt::Write};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ErrorPath {
    segments: Vec<PathSegment>,
}

impl ErrorPath {
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn segments(&self) -> &[PathSegment] {
        &self.segments
    }

    pub fn prepend(&mut self, segment: PathSegment) {
        self.segments.insert(0, segment);
    }
}

impl fmt::Display for ErrorPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for segment in &self.segments {
            f.write_str("/")?;
            match segment {
                PathSegment::Field(name) => write_pointer_segment(f, name)?,
                PathSegment::Index(index) => write!(f, "{index}")?,
                PathSegment::Key => f.write_str("<key>")?,
                PathSegment::Value => f.write_str("<value>")?,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Field(String),
    Index(usize),
    Key,
    Value,
}

impl PathSegment {
    pub fn field(name: impl Into<String>) -> Self {
        Self::Field(name.into())
    }

    pub fn index(index: usize) -> Self {
        Self::Index(index)
    }
}

fn write_pointer_segment(f: &mut fmt::Formatter<'_>, segment: &str) -> fmt::Result {
    for ch in segment.chars() {
        match ch {
            '~' => f.write_str("~0")?,
            '/' => f.write_str("~1")?,
            _ => f.write_char(ch)?,
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    path: ErrorPath,
    source: Option<ErrorSource>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            path: ErrorPath::default(),
            source: None,
        }
    }

    pub fn at_field(mut self, field: impl Into<String>) -> Self {
        self.path.prepend(PathSegment::field(field));
        self
    }

    pub fn at_index(mut self, index: usize) -> Self {
        self.path.prepend(PathSegment::index(index));
        self
    }

    pub fn at_key(mut self) -> Self {
        self.path.prepend(PathSegment::Key);
        self
    }

    pub fn at_value(mut self) -> Self {
        self.path.prepend(PathSegment::Value);
        self
    }

    pub fn with_source(mut self, source: impl Into<ErrorSource>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn path(&self) -> &ErrorPath {
        &self.path
    }

    pub fn source_ref(&self) -> Option<&ErrorSource> {
        self.source.as_ref()
    }

    pub(crate) fn invalid(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Invalid(message.into()))
    }

    pub(crate) fn limit(name: &'static str) -> Self {
        Self::new(ErrorKind::LimitExceeded(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSource {
    Core(Box<Error>),
    Utf8(core::str::Utf8Error),
}

impl From<Error> for ErrorSource {
    fn from(error: Error) -> Self {
        Self::Core(Box::new(error))
    }
}

impl From<core::str::Utf8Error> for ErrorSource {
    fn from(error: core::str::Utf8Error) -> Self {
        Self::Utf8(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedEof,
    TrailingBytes,
    InvalidMagic,
    UnsupportedVersion(u8),
    UnknownEnvelopeMode(u8),
    UnknownTypeTag(u8),
    OverlongVarint,
    VarintOverflow,
    SchemaLengthMismatch,
    SchemaLengthExceeded,
    InvalidSchemaId,
    UnknownSchemaId,
    SchemaRefNotAllowed,
    EmbeddedSchemaMismatch,
    InvalidDecimalParameters,
    InvalidTimestampPrecision(u8),
    StructFieldIdZero,
    StructFieldNameEmpty,
    StructFieldFlagsNonZero(u64),
    DuplicateStructFieldDefinition,
    DuplicateStructFieldValue,
    MissingStructFieldValue,
    InvalidMapKeyType,
    DuplicateMapKey,
    NonCanonicalMapKeyOrder,
    NaNMapKey,
    UnionVariantNameEmpty,
    DuplicateUnionVariantName,
    UnionVariantIndexOutOfRange,
    EnumSymbolEmpty,
    DuplicateEnumSymbol,
    EnumSymbolIndexOutOfRange,
    InvalidBoolValue(u8),
    NonCanonicalF32NaN,
    NonCanonicalF64NaN,
    DecimalCoefficientExceedsPrecision,
    TimeOutOfRange,
    DateTimeTimeOutOfRange,
    DateTimeTzTimeOutOfRange,
    InvalidOptionalPresenceMarker(u8),
    DurationNanosOutOfRange,
    DurationSignMismatch,
    Invalid(Cow<'static, str>),
    LimitExceeded(&'static str),
    Utf8,
    TypeMismatch { expected: &'static str },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::UnexpectedEof => f.write_str("unexpected end of input")?,
            ErrorKind::TrailingBytes => f.write_str("trailing bytes after value")?,
            ErrorKind::InvalidMagic => f.write_str("invalid TPACK magic")?,
            ErrorKind::UnsupportedVersion(version) => {
                write!(f, "unsupported TPACK version {version}")?
            }
            ErrorKind::UnknownEnvelopeMode(mode) => {
                write!(f, "unknown envelope mode 0x{mode:02X}")?
            }
            ErrorKind::UnknownTypeTag(tag) => write!(f, "unknown type tag 0x{tag:02X}")?,
            ErrorKind::OverlongVarint => f.write_str("overlong variable-length integer")?,
            ErrorKind::VarintOverflow => {
                f.write_str("variable-length integer exceeds supported size")?
            }
            ErrorKind::SchemaLengthMismatch => {
                f.write_str("schema length does not match descriptor")?
            }
            ErrorKind::SchemaLengthExceeded => {
                f.write_str("schema length exceeds configured limit")?
            }
            ErrorKind::InvalidSchemaId => f.write_str("invalid schema id")?,
            ErrorKind::UnknownSchemaId => f.write_str("unknown schema id")?,
            ErrorKind::SchemaRefNotAllowed => f.write_str("schema references are not allowed")?,
            ErrorKind::EmbeddedSchemaMismatch => {
                f.write_str("embedded schema does not match cached schema")?
            }
            ErrorKind::InvalidDecimalParameters => {
                f.write_str("invalid Decimal(P,S) parameters")?
            }
            ErrorKind::InvalidTimestampPrecision(precision) => {
                write!(f, "invalid timestamp precision {precision}")?
            }
            ErrorKind::StructFieldIdZero => {
                f.write_str("struct FieldId must be greater than zero")?
            }
            ErrorKind::StructFieldNameEmpty => {
                f.write_str("struct field name must be non-empty")?
            }
            ErrorKind::StructFieldFlagsNonZero(flags) => {
                write!(f, "struct field flags must be zero, got {flags}")?
            }
            ErrorKind::DuplicateStructFieldDefinition => {
                f.write_str("duplicate struct field identifier or name")?
            }
            ErrorKind::DuplicateStructFieldValue => f.write_str("duplicate struct field value")?,
            ErrorKind::MissingStructFieldValue => f.write_str("missing struct field value")?,
            ErrorKind::InvalidMapKeyType => f.write_str("invalid map key type")?,
            ErrorKind::DuplicateMapKey => f.write_str("duplicate map key")?,
            ErrorKind::NonCanonicalMapKeyOrder => f.write_str("non-canonical map key order")?,
            ErrorKind::NaNMapKey => f.write_str("NaN map key")?,
            ErrorKind::UnionVariantNameEmpty => {
                f.write_str("union variant name must be non-empty")?
            }
            ErrorKind::DuplicateUnionVariantName => f.write_str("duplicate union variant name")?,
            ErrorKind::UnionVariantIndexOutOfRange => {
                f.write_str("union variant index out of range")?
            }
            ErrorKind::EnumSymbolEmpty => f.write_str("enum symbol must be non-empty")?,
            ErrorKind::DuplicateEnumSymbol => f.write_str("duplicate enum symbol")?,
            ErrorKind::EnumSymbolIndexOutOfRange => {
                f.write_str("enum symbol index out of range")?
            }
            ErrorKind::InvalidBoolValue(value) => write!(f, "invalid bool value {value}")?,
            ErrorKind::NonCanonicalF32NaN => f.write_str("non-canonical f32 NaN")?,
            ErrorKind::NonCanonicalF64NaN => f.write_str("non-canonical f64 NaN")?,
            ErrorKind::DecimalCoefficientExceedsPrecision => {
                f.write_str("Decimal(P,S) coefficient exceeds precision")?
            }
            ErrorKind::TimeOutOfRange => f.write_str("time value exceeds nanos-per-day")?,
            ErrorKind::DateTimeTimeOutOfRange => {
                f.write_str("datetime time value exceeds nanos-per-day")?
            }
            ErrorKind::DateTimeTzTimeOutOfRange => {
                f.write_str("datetime-tz time value exceeds nanos-per-day")?
            }
            ErrorKind::InvalidOptionalPresenceMarker(marker) => {
                write!(f, "invalid optional presence marker {marker}")?
            }
            ErrorKind::DurationNanosOutOfRange => f.write_str("duration nanos out of range")?,
            ErrorKind::DurationSignMismatch => {
                f.write_str("duration seconds and nanos signs differ")?
            }
            ErrorKind::Invalid(message) => f.write_str(message)?,
            ErrorKind::LimitExceeded(name) => write!(f, "{name} limit exceeded")?,
            ErrorKind::Utf8 => f.write_str("invalid UTF-8")?,
            ErrorKind::TypeMismatch { expected } => {
                write!(f, "TPACK type mismatch, expected {expected}")?
            }
        }
        if !self.path.is_empty() {
            write!(f, " at {}", self.path)?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.source.as_ref()? {
            ErrorSource::Core(error) => Some(error),
            ErrorSource::Utf8(error) => Some(error),
        }
    }
}

impl From<core::str::Utf8Error> for Error {
    fn from(error: core::str::Utf8Error) -> Self {
        Self::new(ErrorKind::Utf8).with_source(error)
    }
}
