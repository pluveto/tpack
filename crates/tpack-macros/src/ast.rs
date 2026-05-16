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
    pub(crate) ty: String,
    pub(crate) tpack_ty: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumVariant {
    pub(crate) name: String,
    pub(crate) payload_ty: Option<String>,
}
