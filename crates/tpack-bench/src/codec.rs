//! Single codec session: one workload, many formats, cold or warm execution.
//!
//! Outside this module nothing touches `Encoder` / `Decoder` details.

use std::sync::Arc;

use tpack::{
    Decoder, Encoder, EnvelopeMode, PreparedSchema, Schema, StdSchemaRegistry, TpackValue,
    encode_message,
};

use crate::model::{ExecPath, Format};
use crate::workload::{Twin, Workload};

#[derive(Debug)]
pub enum Error {
    Tpack(tpack::Error),
    Json(serde_json::Error),
    Cbor(String),
    MsgPack(String),
    Msg(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tpack(e) => write!(f, "{e}"),
            Self::Json(e) => write!(f, "{e}"),
            Self::Cbor(e) | Self::MsgPack(e) | Self::Msg(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<tpack::Error> for Error {
    fn from(value: tpack::Error) -> Self {
        Self::Tpack(value)
    }
}

fn tpack_mode(format: Format) -> Result<(EnvelopeMode, bool), Error> {
    match format {
        Format::TpackFullSchema => Ok((EnvelopeMode::FullSchema, false)),
        Format::TpackFullSchemaWithId => Ok((EnvelopeMode::FullSchemaWithId, true)),
        Format::TpackSchemaRef => Ok((EnvelopeMode::SchemaRef, true)),
        _ => Err(Error::Msg("not a TPACK format".into())),
    }
}

/// Ready-to-fire warm TPACK path for one workload × format.
pub struct WarmTpack {
    prepared: PreparedSchema,
    value: TpackValue<'static>,
    schema_id: [u8; 8],
    mode: EnvelopeMode,
    with_id: bool,
    encoder: Encoder,
    registry: StdSchemaRegistry,
}

impl WarmTpack {
    pub fn new(work: &Workload, format: Format) -> Result<Self, Error> {
        let (mode, with_id) = tpack_mode(format)?;
        let prepared = PreparedSchema::prepare_default(work.schema.clone())?;
        let schema_id = work.schema_id();
        let registry = StdSchemaRegistry::new();
        registry
            .insert(schema_id, work.schema.clone())
            .expect("fresh registry");
        Ok(Self {
            prepared,
            value: work.value.clone(),
            schema_id,
            mode,
            with_id,
            encoder: Encoder::new(),
            registry,
        })
    }

    pub fn encode(&mut self) -> Result<&[u8], Error> {
        self.encoder.clear();
        let id = self.with_id.then_some(self.schema_id.as_slice());
        self.encoder
            .encode_prepared_message(&self.prepared, &self.value, self.mode, id)?;
        Ok(self.encoder.as_slice())
    }

    pub fn encode_vec(&mut self) -> Result<Vec<u8>, Error> {
        self.encode()?;
        Ok(self.encoder.take_vec())
    }

    pub fn decode(&self, bytes: &[u8]) -> Result<(), Error> {
        let mut dec = Decoder::new(bytes);
        match self.mode {
            EnvelopeMode::SchemaRef => {
                dec.decode_message_with_registry(&self.registry)?;
            }
            _ => {
                dec.decode_message()?;
            }
        }
        Ok(())
    }

    pub fn schema(&self) -> &Schema {
        self.prepared.schema()
    }

    pub fn registry(&self) -> &StdSchemaRegistry {
        &self.registry
    }
}

/// Encode once according to path.
pub fn encode(work: &Workload, format: Format, path: ExecPath) -> Result<Vec<u8>, Error> {
    match (format.is_tpack(), path) {
        (true, ExecPath::WarmPrepared) => WarmTpack::new(work, format)?.encode_vec(),
        (true, ExecPath::NaiveMessage) => {
            let (mode, with_id) = tpack_mode(format)?;
            let id = work.schema_id();
            let id_ref = with_id.then_some(id.as_slice());
            Ok(encode_message(&work.schema, &work.value, mode, id_ref)?)
        }
        (false, _) => encode_twin(&work.twin, format),
    }
}

pub fn decode(work: &Workload, format: Format, bytes: &[u8]) -> Result<(), Error> {
    if format.is_tpack() {
        WarmTpack::new(work, format)?.decode(bytes)
    } else {
        decode_competitor(bytes, format)
    }
}

pub fn decode_competitor(bytes: &[u8], format: Format) -> Result<(), Error> {
    decode_twin(bytes, format)
}

fn encode_twin(twin: &Twin, format: Format) -> Result<Vec<u8>, Error> {
    match twin {
        Twin::Wide(w) => encode_named_map(&w.as_named_map(), format),
        other => match format {
            Format::Json => serde_json::to_vec(other).map_err(Error::Json),
            Format::Cbor => {
                let mut buf = Vec::new();
                ciborium::into_writer(other, &mut buf).map_err(|e| Error::Cbor(e.to_string()))?;
                Ok(buf)
            }
            Format::MsgPack => {
                rmp_serde::to_vec_named(other).map_err(|e| Error::MsgPack(e.to_string()))
            }
            _ => Err(Error::Msg("twin encode for TPACK".into())),
        },
    }
}

fn encode_named_map(
    map: &serde_json::Map<String, serde_json::Value>,
    format: Format,
) -> Result<Vec<u8>, Error> {
    match format {
        Format::Json => serde_json::to_vec(map).map_err(Error::Json),
        Format::Cbor => {
            let mut buf = Vec::new();
            ciborium::into_writer(map, &mut buf).map_err(|e| Error::Cbor(e.to_string()))?;
            Ok(buf)
        }
        Format::MsgPack => rmp_serde::to_vec_named(map).map_err(|e| Error::MsgPack(e.to_string())),
        _ => Err(Error::Msg("named map for TPACK".into())),
    }
}

fn decode_twin(bytes: &[u8], format: Format) -> Result<(), Error> {
    match format {
        Format::Json => {
            let _: serde_json::Value = serde_json::from_slice(bytes).map_err(Error::Json)?;
            Ok(())
        }
        Format::Cbor => {
            let _: serde_json::Value =
                ciborium::from_reader(bytes).map_err(|e| Error::Cbor(e.to_string()))?;
            Ok(())
        }
        Format::MsgPack => {
            let _: serde_json::Value =
                rmp_serde::from_slice(bytes).map_err(|e| Error::MsgPack(e.to_string()))?;
            Ok(())
        }
        _ => Err(Error::Msg("twin decode for TPACK".into())),
    }
}

/// Shared registry for multi-thread probes.
pub fn shared_registry(work: &Workload) -> (Arc<StdSchemaRegistry>, [u8; 8], Schema) {
    let id = work.schema_id();
    let reg = StdSchemaRegistry::new();
    reg.insert(id, work.schema.clone()).expect("insert");
    (Arc::new(reg), id, work.schema.clone())
}

pub fn parse_breakdown(
    bytes: &[u8],
    format: Format,
) -> Result<(usize, usize, usize, usize), Error> {
    if bytes.len() < 6 || &bytes[..4] != b"TPAK" {
        return Err(Error::Msg("not tpack".into()));
    }
    let (mode, _) = tpack_mode(format)?;
    let mut i = 6;
    if bytes[5] != mode.tag() {
        return Err(Error::Msg("mode mismatch".into()));
    }
    let header = 6;
    let mut sid = 0;
    let mut schema = 0;
    match mode {
        EnvelopeMode::FullSchema => {
            let (n, ni) = read_uv(bytes, i)?;
            schema = n as usize;
            i = ni + schema;
        }
        EnvelopeMode::FullSchemaWithId => {
            let (n, ni) = read_uv(bytes, i)?;
            sid = n as usize;
            i = ni + sid;
            let (n, ni) = read_uv(bytes, i)?;
            schema = n as usize;
            i = ni + schema;
        }
        EnvelopeMode::SchemaRef => {
            let (n, ni) = read_uv(bytes, i)?;
            sid = n as usize;
            i = ni + sid;
        }
    }
    Ok((header, sid, schema, bytes.len() - i))
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
