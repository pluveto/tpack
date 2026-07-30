//! Endpoints understand encode/decode. Workloads never match on format.
//!
//! An endpoint is a *place* a workload can be sent. Warm TPACK endpoints keep
//! prepared schema + encoder state; serde endpoints are stateless.

use std::sync::Arc;

use tpack::{
    Decoder, Encoder, EnvelopeMode, PreparedSchema, Schema, StdSchemaRegistry, encode_message,
};

use crate::work::{Twin, Workload};

#[derive(Debug)]
pub enum Error {
    Tpack(tpack::Error),
    Serde(String),
    Msg(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tpack(e) => write!(f, "{e}"),
            Self::Serde(e) | Self::Msg(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<tpack::Error> for Error {
    fn from(e: tpack::Error) -> Self {
        Self::Tpack(e)
    }
}

/// How TPACK is driven (never collapse these in reports).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpackStyle {
    /// Prepared schema + reused encoder buffer.
    Warm,
    /// Fresh `encode_message` every call (re-encodes schema for FullSchema*).
    Naive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpackEnvelope {
    FullSchema,
    FullSchemaWithId,
    SchemaRef,
}

impl TpackEnvelope {
    pub fn label(self) -> &'static str {
        match self {
            Self::FullSchema => "tpack-fullschema",
            Self::FullSchemaWithId => "tpack-fullschema-with-id",
            Self::SchemaRef => "tpack-schemaref",
        }
    }

    fn mode(self) -> EnvelopeMode {
        match self {
            Self::FullSchema => EnvelopeMode::FullSchema,
            Self::FullSchemaWithId => EnvelopeMode::FullSchemaWithId,
            Self::SchemaRef => EnvelopeMode::SchemaRef,
        }
    }

    fn needs_id(self) -> bool {
        !matches!(self, Self::FullSchema)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerdeFormat {
    Json,
    Cbor,
    MsgPack,
}

impl SerdeFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Cbor => "cbor",
            Self::MsgPack => "msgpack",
        }
    }
}

/// Something that can encode/decode a [`Workload`].
pub struct Endpoint {
    label: String,
    path_label: &'static str,
    kind: Kind,
}

enum Kind {
    TpackWarm {
        envelope: TpackEnvelope,
        prepared: PreparedSchema,
        value: tpack::TpackValue<'static>,
        schema_id: [u8; 8],
        encoder: Encoder,
        registry: StdSchemaRegistry,
    },
    TpackNaive {
        envelope: TpackEnvelope,
        schema: Schema,
        value: tpack::TpackValue<'static>,
        schema_id: [u8; 8],
    },
    Serde {
        format: SerdeFormat,
        twin: Twin,
    },
}

impl Endpoint {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn path_label(&self) -> &'static str {
        self.path_label
    }

    pub fn tpack_warm(work: &Workload, envelope: TpackEnvelope) -> Result<Self, Error> {
        let prepared = PreparedSchema::prepare_default(work.schema().clone())?;
        let schema_id = work.schema_id();
        let registry = StdSchemaRegistry::new();
        registry
            .insert(schema_id, work.schema().clone())
            .expect("fresh");
        Ok(Self {
            label: envelope.label().into(),
            path_label: "warm-prepared",
            kind: Kind::TpackWarm {
                envelope,
                prepared,
                value: work.value().clone(),
                schema_id,
                encoder: Encoder::new(),
                registry,
            },
        })
    }

    pub fn tpack_naive(work: &Workload, envelope: TpackEnvelope) -> Self {
        Self {
            label: envelope.label().into(),
            path_label: "naive-message",
            kind: Kind::TpackNaive {
                envelope,
                schema: work.schema().clone(),
                value: work.value().clone(),
                schema_id: work.schema_id(),
            },
        }
    }

    pub fn serde(work: &Workload, format: SerdeFormat) -> Self {
        Self {
            label: format.label().into(),
            path_label: "oneshot",
            kind: Kind::Serde {
                format,
                twin: work.twin().clone(),
            },
        }
    }

    /// Encode; warm endpoints reuse their buffer (returned slice is internal).
    pub fn encode(&mut self) -> Result<Vec<u8>, Error> {
        match &mut self.kind {
            Kind::TpackWarm {
                envelope,
                prepared,
                value,
                schema_id,
                encoder,
                ..
            } => {
                encoder.clear();
                let id = envelope.needs_id().then_some(schema_id.as_slice());
                encoder.encode_prepared_message(prepared, value, envelope.mode(), id)?;
                Ok(encoder.as_slice().to_vec())
            }
            Kind::TpackNaive {
                envelope,
                schema,
                value,
                schema_id,
            } => {
                let id = envelope.needs_id().then_some(schema_id.as_slice());
                Ok(encode_message(schema, value, envelope.mode(), id)?)
            }
            Kind::Serde { format, twin } => encode_serde(format, twin),
        }
    }

    pub fn decode(&self, bytes: &[u8]) -> Result<(), Error> {
        match &self.kind {
            Kind::TpackWarm {
                envelope, registry, ..
            } => {
                let mut d = Decoder::new(bytes);
                match envelope {
                    TpackEnvelope::SchemaRef => {
                        d.decode_message_with_registry(registry)?;
                    }
                    _ => {
                        d.decode_message()?;
                    }
                }
                Ok(())
            }
            Kind::TpackNaive {
                envelope,
                schema,
                schema_id,
                ..
            } => {
                // Naive decode still needs a registry for SchemaRef.
                let mut d = Decoder::new(bytes);
                match envelope {
                    TpackEnvelope::SchemaRef => {
                        let reg = StdSchemaRegistry::new();
                        reg.insert(*schema_id, schema.clone()).expect("i");
                        d.decode_message_with_registry(&reg)?;
                    }
                    _ => {
                        d.decode_message()?;
                    }
                }
                Ok(())
            }
            Kind::Serde { format, .. } => decode_serde(format, bytes),
        }
    }

    pub fn shared_registry(work: &Workload) -> Arc<StdSchemaRegistry> {
        let reg = StdSchemaRegistry::new();
        reg.insert(work.schema_id(), work.schema().clone())
            .expect("i");
        Arc::new(reg)
    }

    pub fn decode_with_registry(bytes: &[u8], registry: &StdSchemaRegistry) -> Result<(), Error> {
        let mut d = Decoder::new(bytes);
        d.decode_message_with_registry(registry)?;
        Ok(())
    }
}

fn encode_serde(format: &SerdeFormat, twin: &Twin) -> Result<Vec<u8>, Error> {
    let Twin::Value(v) = twin;
    match format {
        SerdeFormat::Json => serde_json::to_vec(v).map_err(|e| Error::Serde(e.to_string())),
        SerdeFormat::Cbor => {
            let mut buf = Vec::new();
            ciborium::into_writer(v, &mut buf).map_err(|e| Error::Serde(e.to_string()))?;
            Ok(buf)
        }
        SerdeFormat::MsgPack => rmp_serde::to_vec_named(v).map_err(|e| Error::Serde(e.to_string())),
    }
}

fn decode_serde(format: &SerdeFormat, bytes: &[u8]) -> Result<(), Error> {
    match format {
        SerdeFormat::Json => {
            let _: serde_json::Value =
                serde_json::from_slice(bytes).map_err(|e| Error::Serde(e.to_string()))?;
            Ok(())
        }
        SerdeFormat::Cbor => {
            let _: serde_json::Value =
                ciborium::from_reader(bytes).map_err(|e| Error::Serde(e.to_string()))?;
            Ok(())
        }
        SerdeFormat::MsgPack => {
            let _: serde_json::Value =
                rmp_serde::from_slice(bytes).map_err(|e| Error::Serde(e.to_string()))?;
            Ok(())
        }
    }
}

/// Parse TPACK envelope anatomy from bytes (for breakdown samples).
pub fn tpack_parts(
    bytes: &[u8],
    envelope: TpackEnvelope,
) -> Result<(usize, usize, usize, usize), Error> {
    if bytes.len() < 6 || &bytes[..4] != b"TPAK" {
        return Err(Error::Msg("not tpack".into()));
    }
    if bytes[5] != envelope.mode().tag() {
        return Err(Error::Msg("mode mismatch".into()));
    }
    let mut i = 6;
    let header = 6;
    let mut sid = 0usize;
    let mut schema = 0usize;
    match envelope {
        TpackEnvelope::FullSchema => {
            let (n, ni) = read_uv(bytes, i)?;
            schema = n as usize;
            i = ni + schema;
        }
        TpackEnvelope::FullSchemaWithId => {
            let (n, ni) = read_uv(bytes, i)?;
            sid = n as usize;
            i = ni + sid;
            let (n, ni) = read_uv(bytes, i)?;
            schema = n as usize;
            i = ni + schema;
        }
        TpackEnvelope::SchemaRef => {
            let (n, ni) = read_uv(bytes, i)?;
            sid = n as usize;
            i = ni + sid;
        }
    }
    Ok((header, sid, schema, bytes.len().saturating_sub(i)))
}

fn read_uv(b: &[u8], mut i: usize) -> Result<(u64, usize), Error> {
    let mut v = 0u64;
    let mut s = 0u32;
    loop {
        let c = *b.get(i).ok_or_else(|| Error::Msg("eof".into()))?;
        i += 1;
        v |= u64::from(c & 0x7f) << s;
        if c < 0x80 {
            return Ok((v, i));
        }
        s += 7;
        if s > 63 {
            return Err(Error::Msg("varint".into()));
        }
    }
}
