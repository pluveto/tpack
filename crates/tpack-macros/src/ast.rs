use core::fmt;

#[derive(Debug, Clone)]
pub(crate) struct Item {
    pub(crate) name: String,
    pub(crate) kind: ItemKind,
}

#[derive(Debug, Clone)]
pub(crate) enum ItemKind {
    Struct(Vec<Field>),
    Enum(Vec<EnumVariant>),
}

#[derive(Debug, Clone)]
pub(crate) struct Field {
    pub(crate) rust_name: String,
    pub(crate) wire_name: String,
    pub(crate) field_id: u64,
    pub(crate) ty: TypeRef,
    pub(crate) tpack_ty: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumVariant {
    pub(crate) name: String,
    pub(crate) payload_ty: Option<TypeRef>,
}

#[derive(Debug, Clone)]
pub(crate) struct TypeRef {
    source: String,
    kind: TypeKind,
}

impl TypeRef {
    pub(crate) fn new(source: String, kind: TypeKind) -> Self {
        Self { source, kind }
    }

    pub(crate) fn is_option(&self) -> bool {
        match &self.kind {
            TypeKind::Path(path) => {
                let segments = path.segments.iter().map(String::as_str).collect::<Vec<_>>();
                matches!(
                    segments.as_slice(),
                    ["Option"] | ["std", "option", "Option"] | ["core", "option", "Option"]
                )
            }
            TypeKind::Other => false,
        }
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.source)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum TypeKind {
    Path(TypePath),
    Other,
}

#[derive(Debug, Clone)]
pub(crate) struct TypePath {
    pub(crate) segments: Vec<String>,
}
