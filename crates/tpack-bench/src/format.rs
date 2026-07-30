//! Wire formats compared by the harness.

/// Wire formats measured by the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    TpackFullSchema,
    TpackFullSchemaWithId,
    TpackSchemaRef,
    Json,
    Cbor,
    MsgPack,
}

impl Format {
    pub const ALL: [Format; 6] = [
        Format::TpackFullSchema,
        Format::TpackFullSchemaWithId,
        Format::TpackSchemaRef,
        Format::Json,
        Format::Cbor,
        Format::MsgPack,
    ];

    /// Steady-state comparison set (no embedded schema tax).
    pub const STEADY: [Format; 4] = [
        Format::TpackSchemaRef,
        Format::Json,
        Format::Cbor,
        Format::MsgPack,
    ];

    /// Cold / self-contained TPACK modes.
    pub const COLD_TPACK: [Format; 2] = [Format::TpackFullSchema, Format::TpackFullSchemaWithId];

    pub fn label(self) -> &'static str {
        match self {
            Format::TpackFullSchema => "tpack-fullschema",
            Format::TpackFullSchemaWithId => "tpack-fullschema-with-id",
            Format::TpackSchemaRef => "tpack-schemaref",
            Format::Json => "json",
            Format::Cbor => "cbor",
            Format::MsgPack => "msgpack",
        }
    }

    pub fn is_tpack(self) -> bool {
        matches!(
            self,
            Format::TpackFullSchema | Format::TpackFullSchemaWithId | Format::TpackSchemaRef
        )
    }
}
