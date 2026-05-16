use alloc::{borrow::Cow, string::String};
use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    path: Option<String>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, path: None }
    }

    pub fn at_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub(crate) fn invalid(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(ErrorKind::Invalid(message.into()))
    }

    pub(crate) fn limit(name: &'static str) -> Self {
        Self::new(ErrorKind::LimitExceeded(name))
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
    Invalid(Cow<'static, str>),
    OverlongVarint,
    VarintOverflow,
    SchemaLengthMismatch,
    SchemaLengthExceeded,
    InvalidSchemaId,
    UnknownSchemaId,
    SchemaRefNotAllowed,
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
            ErrorKind::Invalid(message) => f.write_str(message)?,
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
            ErrorKind::LimitExceeded(name) => write!(f, "{name} limit exceeded")?,
            ErrorKind::Utf8 => f.write_str("invalid UTF-8")?,
            ErrorKind::TypeMismatch { expected } => {
                write!(f, "TPACK type mismatch, expected {expected}")?
            }
        }
        if let Some(path) = &self.path {
            write!(f, " at {path}")?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl From<core::str::Utf8Error> for Error {
    fn from(_: core::str::Utf8Error) -> Self {
        Self::new(ErrorKind::Utf8)
    }
}
