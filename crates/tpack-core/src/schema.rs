use alloc::{borrow::Cow, boxed::Box, string::String, sync::Arc, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeMode {
    FullSchema,
    FullSchemaWithId,
    SchemaRef,
}

impl EnvelopeMode {
    pub fn tag(self) -> u8 {
        match self {
            EnvelopeMode::FullSchema => 0x00,
            EnvelopeMode::FullSchemaWithId => 0x01,
            EnvelopeMode::SchemaRef => 0x02,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaId<'de>(pub Cow<'de, [u8]>);

impl<'de> SchemaId<'de> {
    pub fn borrowed(bytes: &'de [u8]) -> Self {
        Self(Cow::Borrowed(bytes))
    }

    pub fn owned(bytes: Vec<u8>) -> Self {
        Self(Cow::Owned(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message<'de> {
    pub envelope: Envelope<'de>,
    pub schema: Arc<Schema>,
    pub value: crate::TpackValue<'de>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Envelope<'de> {
    pub mode: EnvelopeMode,
    pub schema_id: Option<SchemaId<'de>>,
    pub used_cached_schema: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub root: TypeDescriptor,
}

impl Schema {
    pub fn new(root: TypeDescriptor) -> Self {
        Self { root }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDescriptor {
    Null,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Decimal,
    DecimalFixed {
        precision: u64,
        scale: u64,
    },
    String {
        max_len: Option<u64>,
    },
    Bytes {
        max_len: Option<u64>,
    },
    Date,
    Time,
    DateTime,
    DateTimeTz,
    Timestamp(TimestampPrecision),
    Duration,
    BigInt,
    BigUInt,
    CalendarInterval,
    Struct(Vec<Field>),
    List {
        max_count: Option<u64>,
        element: Box<TypeDescriptor>,
    },
    Map {
        max_count: Option<u64>,
        key: Box<TypeDescriptor>,
        value: Box<TypeDescriptor>,
    },
    Union(Vec<Variant>),
    Enum(Vec<String>),
    Optional(Box<TypeDescriptor>),
    Extension {
        authority: String,
        type_name: String,
        schema_params: Vec<u8>,
    },
}

impl TypeDescriptor {
    pub fn is_composite(&self) -> bool {
        matches!(
            self,
            Self::Struct(_)
                | Self::List { .. }
                | Self::Map { .. }
                | Self::Union(_)
                | Self::Enum(_)
                | Self::Optional(_)
        )
    }

    pub fn struct_field(&self, field_id: u64) -> Option<&Field> {
        match self {
            Self::Struct(fields) => fields.iter().find(|field| field.id == field_id),
            _ => None,
        }
    }

    pub fn list_element(&self) -> Option<&TypeDescriptor> {
        match self {
            Self::List { element, .. } => Some(element.as_ref()),
            _ => None,
        }
    }

    pub fn map_key_value(&self) -> Option<(&TypeDescriptor, &TypeDescriptor)> {
        match self {
            Self::Map { key, value, .. } => Some((key.as_ref(), value.as_ref())),
            _ => None,
        }
    }

    pub fn union_variant(&self, index: u64) -> Option<&Variant> {
        match self {
            Self::Union(variants) => usize::try_from(index)
                .ok()
                .and_then(|index| variants.get(index)),
            _ => None,
        }
    }

    pub fn optional_inner(&self) -> Option<&TypeDescriptor> {
        match self {
            Self::Optional(inner) => Some(inner.as_ref()),
            _ => None,
        }
    }

    pub fn type_label(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Bool => "Bool",
            Self::I8 => "I8",
            Self::I16 => "I16",
            Self::I32 => "I32",
            Self::I64 => "I64",
            Self::U8 => "U8",
            Self::U16 => "U16",
            Self::U32 => "U32",
            Self::U64 => "U64",
            Self::F32 => "F32",
            Self::F64 => "F64",
            Self::Decimal => "Decimal",
            Self::DecimalFixed { .. } => "Decimal(P,S)",
            Self::String { .. } => "String",
            Self::Bytes { .. } => "Bytes",
            Self::Date => "Date",
            Self::Time => "Time",
            Self::DateTime => "DateTime",
            Self::DateTimeTz => "DateTimeTZ",
            Self::Timestamp(_) => "Timestamp(P)",
            Self::Duration => "Duration",
            Self::BigInt => "BigInt",
            Self::BigUInt => "BigUInt",
            Self::CalendarInterval => "CalendarInterval",
            Self::Struct(_) => "Struct",
            Self::List { .. } => "List",
            Self::Map { .. } => "Map",
            Self::Union(_) => "Union",
            Self::Enum(_) => "Enum",
            Self::Optional(_) => "Optional",
            Self::Extension { .. } => "Extension",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub id: u64,
    pub name: String,
    pub ty: TypeDescriptor,
}

impl Field {
    pub fn new(id: u64, name: impl Into<String>, ty: TypeDescriptor) -> Self {
        Self {
            id,
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub name: String,
    pub ty: TypeDescriptor,
}

impl Variant {
    pub fn new(name: impl Into<String>, ty: TypeDescriptor) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampPrecision {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl TimestampPrecision {
    pub fn tag(self) -> u8 {
        match self {
            TimestampPrecision::Seconds => 0,
            TimestampPrecision::Milliseconds => 1,
            TimestampPrecision::Microseconds => 2,
            TimestampPrecision::Nanoseconds => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duration {
    pub seconds: i64,
    pub nanos: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarInterval {
    pub months: i64,
    pub days: i64,
    pub nanos: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal {
    pub scale: i64,
    pub coefficient: i64,
}
